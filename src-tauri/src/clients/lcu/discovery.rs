//! Locates a running League client and extracts its LCU credentials.
//!
//! Primary source is the `LeagueClientUx.exe` command line. When the client runs
//! elevated (typical for the Tencent client) neither its command line nor its path can be
//! read from a normal process, so we fall back to the `lockfile` next to the executable:
//! found through the process when possible, otherwise in directories remembered from an
//! earlier (elevated) connection.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use serde::Serialize;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

const UX_PROCESS: &str = "LeagueClientUx.exe";
const CLIENT_PROCESS: &str = "LeagueClient.exe";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CredentialSource {
    CommandLine,
    Lockfile,
}

#[derive(Clone)]
pub struct Credentials {
    pub pid: u32,
    pub port: u16,
    pub auth_token: String,
    pub platform_id: Option<String>,
    pub source: CredentialSource,
    /// Directory of the client executables, when it could be seen; remembered so the
    /// lockfile can be found without admin rights next time.
    pub client_dir: Option<PathBuf>,
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("pid", &self.pid)
            .field("port", &self.port)
            .field("auth_token", &"<redacted>")
            .field("platform_id", &self.platform_id)
            .field("source", &self.source)
            .field("client_dir", &self.client_dir)
            .finish()
    }
}

pub enum Discovery {
    NotRunning,
    /// A client process exists but neither its command line nor its lockfile is readable.
    Unreadable,
    Found(Credentials),
}

/// Blocking: enumerates processes. Call from `spawn_blocking`.
/// `known_dirs` are client directories remembered from earlier connections.
pub fn discover(known_dirs: &[PathBuf]) -> Discovery {
    let mut sys = System::new();
    let names_only = ProcessRefreshKind::nothing();
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, names_only);

    let ux_pids = pids_by_name(&sys, UX_PROCESS);
    let client_pids = pids_by_name(&sys, CLIENT_PROCESS);
    if ux_pids.is_empty() && client_pids.is_empty() {
        return Discovery::NotRunning;
    }

    let targets: Vec<Pid> = ux_pids.iter().chain(&client_pids).copied().collect();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::Some(&targets),
        false,
        ProcessRefreshKind::nothing()
            .with_cmd(UpdateKind::Always)
            .with_exe(UpdateKind::Always),
    );

    for pid in &ux_pids {
        let Some(process) = sys.process(*pid) else {
            continue;
        };
        if let Some(mut creds) = parse_command_line(process.cmd()) {
            let exe = process.exe();
            creds.client_dir = exe.and_then(Path::parent).map(Path::to_path_buf);
            return Discovery::Found(creds);
        }
    }

    for pid in &targets {
        let exe = sys.process(*pid).and_then(|p| p.exe());
        let Some(dir) = exe.and_then(Path::parent) else {
            continue;
        };
        if let Some(mut creds) = read_lockfile(&dir.join("lockfile")) {
            creds.client_dir = Some(dir.to_path_buf());
            return Discovery::Found(creds);
        }
    }

    // The lockfile outlives a crashed client, so only trust one whose pid is running.
    for dir in known_dirs {
        let Some(mut creds) = read_lockfile(&dir.join("lockfile")) else {
            continue;
        };
        if targets.contains(&Pid::from_u32(creds.pid)) {
            creds.client_dir = Some(dir.clone());
            return Discovery::Found(creds);
        }
    }

    Discovery::Unreadable
}

fn pids_by_name(sys: &System, name: &str) -> Vec<Pid> {
    let processes = sys.processes_by_exact_name(OsStr::new(name));
    processes.map(|p| p.pid()).collect()
}

fn parse_command_line<S: AsRef<OsStr>>(args: &[S]) -> Option<Credentials> {
    let mut port: Option<u16> = None;
    let mut auth_token: Option<String> = None;
    let mut pid: Option<u32> = None;
    let mut platform_id: Option<String> = None;

    // Arguments normally arrive split, but tolerate a single unsplit string with quotes.
    let mut joined = String::new();
    for arg in args {
        joined.push_str(&arg.as_ref().to_string_lossy());
        joined.push(' ');
    }

    for token in joined.split_whitespace() {
        let Some((key, value)) = token.trim_matches('"').split_once('=') else {
            continue;
        };
        match key {
            "--app-port" => port = value.parse().ok(),
            "--remoting-auth-token" => auth_token = Some(value.to_owned()),
            "--app-pid" => pid = value.parse().ok(),
            "--rso_platform_id" | "--rso-platform-id" => platform_id = Some(value.to_owned()),
            _ => {}
        }
    }

    Some(Credentials {
        pid: pid?,
        port: port?,
        auth_token: auth_token.filter(|t| !t.is_empty())?,
        platform_id: platform_id.filter(|p| !p.is_empty()),
        source: CredentialSource::CommandLine,
        client_dir: None,
    })
}

fn read_lockfile(path: &Path) -> Option<Credentials> {
    let content = std::fs::read_to_string(path).ok()?;
    parse_lockfile(&content)
}

/// `name:pid:port:password:protocol`
fn parse_lockfile(content: &str) -> Option<Credentials> {
    let mut parts = content.trim().split(':');
    let _name = parts.next()?;
    let pid = parts.next()?.parse().ok()?;
    let port = parts.next()?.parse().ok()?;
    let auth_token = parts.next()?.to_owned();
    if auth_token.is_empty() {
        return None;
    }
    Some(Credentials {
        pid,
        port,
        auth_token,
        platform_id: None,
        source: CredentialSource::Lockfile,
        client_dir: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_split_command_line() {
        let args = [
            "C:/Riot Games/League of Legends/LeagueClientUx.exe",
            "--riotclient-auth-token=abc",
            "--app-port=51234",
            "--remoting-auth-token=Xy_z-1",
            "--app-pid=4242",
            "--rso_platform_id=HN1",
        ];
        let c = parse_command_line(&args).unwrap();
        assert_eq!(c.port, 51234);
        assert_eq!(c.pid, 4242);
        assert_eq!(c.auth_token, "Xy_z-1");
        assert_eq!(c.platform_id.as_deref(), Some("HN1"));
        assert_eq!(c.source, CredentialSource::CommandLine);
    }

    #[test]
    fn parses_unsplit_quoted_command_line() {
        let args = ["\"Ux.exe\" \"--app-port=1\" \"--remoting-auth-token=t\" \"--app-pid=2\""];
        let c = parse_command_line(&args).unwrap();
        assert_eq!((c.port, c.pid), (1, 2));
        assert_eq!(c.platform_id, None);
    }

    #[test]
    fn rejects_incomplete_command_line() {
        assert!(parse_command_line(&["--app-port=1", "--app-pid=2"]).is_none());
    }

    #[test]
    fn parses_lockfile() {
        let c = parse_lockfile("LeagueClient:9876:62000:s3cr3t:https\n").unwrap();
        assert_eq!((c.pid, c.port), (9876, 62000));
        assert_eq!(c.auth_token, "s3cr3t");
        assert_eq!(c.source, CredentialSource::Lockfile);
    }
}
