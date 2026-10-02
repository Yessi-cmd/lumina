//! A fresh, administrator-only directory for executable update payloads.

use std::fs::File;
use std::path::PathBuf;

use crate::error::{AppError, Result};

pub(super) struct Staging {
    dir: PathBuf,
    // Prevent renaming/replacing the directory while its path is in use.
    directory_lock: Option<File>,
    keep: bool,
}

impl Staging {
    pub(super) fn new() -> Result<Self> {
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            use std::os::windows::fs::OpenOptionsExt;
            use std::time::{SystemTime, UNIX_EPOCH};

            use windows_sys::Win32::Foundation::LocalFree;
            use windows_sys::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;
            use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
            use windows_sys::Win32::Storage::FileSystem::{
                CreateDirectoryW, FILE_FLAG_BACKUP_SEMANTICS, FILE_SHARE_READ, FILE_SHARE_WRITE,
            };

            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|err| AppError::Message(err.to_string()))?
                .as_nanos();
            let name = format!("lumina-update-{}-{stamp}", std::process::id());
            let dir = std::env::temp_dir().join(name);
            let path: Vec<u16> = dir.as_os_str().encode_wide().chain(Some(0)).collect();
            // Protected DACL, inherited by files: only SYSTEM and elevated
            // Administrators. Never create permissively and change ACLs later.
            let sddl: Vec<u16> = "O:BAG:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)"
                .encode_utf16()
                .chain(Some(0))
                .collect();
            let mut descriptor = std::ptr::null_mut();
            // SAFETY: both strings are terminated UTF-16; the descriptor is
            // allocated by Windows, used during CreateDirectoryW, then freed.
            unsafe {
                if ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    sddl.as_ptr(),
                    1,
                    &mut descriptor,
                    std::ptr::null_mut(),
                ) == 0
                {
                    return Err(std::io::Error::last_os_error().into());
                }
                let attributes = SECURITY_ATTRIBUTES {
                    nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                    lpSecurityDescriptor: descriptor,
                    bInheritHandle: 0,
                };
                let created = CreateDirectoryW(path.as_ptr(), &attributes);
                let error = std::io::Error::last_os_error();
                LocalFree(descriptor);
                if created == 0 {
                    // In particular, never reuse an existing directory.
                    return Err(error.into());
                }
            }
            let mut staging = Self {
                dir,
                directory_lock: None,
                keep: false,
            };
            staging.directory_lock = Some(
                std::fs::OpenOptions::new()
                    .read(true)
                    .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
                    .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
                    .open(&staging.dir)?,
            );
            Ok(staging)
        }
        #[cfg(not(windows))]
        Err(AppError::Message("此平台不支持自动安装更新".to_owned()))
    }

    pub(super) fn path(&self) -> PathBuf {
        // Never use the remotely supplied asset name as a local path.
        self.dir.join("installer.exe")
    }

    pub(super) fn open_verified(&self) -> Result<File> {
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            use windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ;
            options.share_mode(FILE_SHARE_READ);
        }
        Ok(options.open(self.path())?)
    }

    pub(super) fn keep(mut self) {
        self.keep = true;
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        if !self.keep {
            let _ = std::fs::remove_file(self.path());
            self.directory_lock.take();
            let _ = std::fs::remove_dir(&self.dir);
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    // These tests use ordinary directories so they also run without elevation.
    // Production uses the atomic administrator-only directory creation above.
    fn fixture() -> Staging {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("lumina-staging-test-{stamp}"));
        std::fs::create_dir(&dir).unwrap();
        Staging {
            dir,
            directory_lock: None,
            keep: false,
        }
    }

    #[test]
    fn verified_handle_prevents_writes_and_replacement() {
        let staging = fixture();
        std::fs::write(staging.path(), b"trusted").unwrap();
        let guard = staging.open_verified().unwrap();
        assert!(std::fs::OpenOptions::new()
            .write(true)
            .open(staging.path())
            .is_err());
        assert!(std::fs::remove_file(staging.path()).is_err());
        assert!(std::fs::rename(staging.path(), staging.dir.join("moved.exe")).is_err());
        drop(guard);
        assert!(std::fs::OpenOptions::new()
            .write(true)
            .open(staging.path())
            .is_ok());
    }

    #[test]
    fn verified_executable_can_start_while_locked() {
        let staging = fixture();
        std::fs::copy(std::env::current_exe().unwrap(), staging.path()).unwrap();
        let _guard = staging.open_verified().unwrap();
        // List tests in the copied harness; do not recursively run them.
        let output = std::process::Command::new(staging.path())
            .arg("--list")
            .output()
            .unwrap();
        assert!(output.status.success());
    }

    #[tokio::test]
    async fn verifies_disk_contents_and_rejects_tampering() {
        use sha2::{Digest, Sha256};

        let staging = fixture();
        let expected = super::super::hex(&Sha256::digest(b"trusted"));
        for (contents, valid) in [
            (b"trusted".as_slice(), true),
            (b"altered".as_slice(), false),
            (b"short".as_slice(), false),
            (b"too long".as_slice(), false),
        ] {
            std::fs::write(staging.path(), contents).unwrap();
            let mut file = tokio::fs::File::from_std(staging.open_verified().unwrap());
            assert_eq!(
                super::super::verify_file(&mut file, 7, &expected)
                    .await
                    .is_ok(),
                valid
            );
        }
    }
}
