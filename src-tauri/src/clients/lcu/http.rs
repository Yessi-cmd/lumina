use std::time::Duration;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::de::DeserializeOwned;

use super::discovery::Credentials;
use crate::error::{AppError, Result};

const RIOT_ROOT_PEM: &[u8] = include_bytes!("../../../resources/riotgames.pem");
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// TLS settings shared by HTTP and WebSocket: trust only Riot's root certificate.
///
/// The LCU leaf certificate is not issued for `127.0.0.1`, so the hostname check is
/// skipped; the chain itself is still verified against `riotgames.pem`.
pub fn tls_connector() -> Result<native_tls::TlsConnector> {
    let root = native_tls::Certificate::from_pem(RIOT_ROOT_PEM)?;
    let connector = native_tls::TlsConnector::builder()
        .add_root_certificate(root)
        .disable_built_in_roots(true)
        .danger_accept_invalid_hostnames(true)
        .build()?;
    Ok(connector)
}

pub fn basic_auth(creds: &Credentials) -> String {
    let raw = format!("riot:{}", creds.auth_token);
    format!("Basic {}", STANDARD.encode(raw))
}

pub struct LcuHttp {
    client: reqwest::Client,
    base_url: String,
}

impl LcuHttp {
    pub fn new(creds: &Credentials) -> Result<Self> {
        let value = basic_auth(creds);
        let mut auth = HeaderValue::from_str(&value).expect("base64 is ASCII");
        auth.set_sensitive(true);
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, auth);

        let client = reqwest::Client::builder()
            .use_preconfigured_tls(tls_connector()?)
            .default_headers(headers)
            .timeout(REQUEST_TIMEOUT)
            .no_proxy()
            .build()?;

        Ok(Self {
            client,
            base_url: format!("https://127.0.0.1:{}", creds.port),
        })
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}{path}", self.base_url);
        let resp = self.client.get(url).send().await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(AppError::LcuStatus {
                status: status.as_u16(),
                path: path.to_owned(),
            });
        }
        Ok(resp.json().await?)
    }
}
