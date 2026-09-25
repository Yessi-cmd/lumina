use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::header::AUTHORIZATION;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{
    connect_async_tls_with_config, Connector, MaybeTlsStream, WebSocketStream,
};

use super::discovery::Credentials;
use super::http::{basic_auth, tls_connector};
use super::models::LcuEvent;
use crate::error::{AppError, Result};

/// WAMP SUBSCRIBE to every LCU JSON API event.
const SUBSCRIBE_ALL: &str = r#"[5,"OnJsonApiEvent"]"#;
/// WAMP EVENT message type id.
const WAMP_EVENT: u64 = 8;
/// League Akari uses 17.5s; the socket is local, so anything slower means a stuck client.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

pub struct LcuSocket {
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl LcuSocket {
    pub async fn connect(creds: &Credentials) -> Result<Self> {
        let url = format!("wss://127.0.0.1:{}", creds.port);
        let mut request = url.into_client_request()?;
        let value = basic_auth(creds);
        let mut auth = HeaderValue::from_str(&value).expect("base64 is ASCII");
        auth.set_sensitive(true);
        request.headers_mut().insert(AUTHORIZATION, auth);

        let connector = Connector::NativeTls(tls_connector()?);
        let connect = connect_async_tls_with_config(request, None, false, Some(connector));
        let Ok(result) = tokio::time::timeout(CONNECT_TIMEOUT, connect).await else {
            return Err(AppError::Timeout("LCU WebSocket 连接"));
        };
        let (mut stream, _) = result?;
        stream.send(Message::Text(SUBSCRIBE_ALL.into())).await?;
        Ok(Self { stream })
    }

    /// Feeds every event to `on_event` until the client closes the socket.
    pub async fn run(mut self, mut on_event: impl FnMut(LcuEvent)) -> Result<()> {
        while let Some(message) = self.stream.next().await {
            match message? {
                Message::Text(text) => {
                    if let Some(event) = parse_event(text.as_str()) {
                        on_event(event);
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
        Ok(())
    }
}

/// `[8, "OnJsonApiEvent", { "uri": ..., "eventType": ..., "data": ... }]`
fn parse_event(text: &str) -> Option<LcuEvent> {
    let mut frame: Vec<Value> = serde_json::from_str(text).ok()?;
    if frame.len() < 3 || frame[0].as_u64() != Some(WAMP_EVENT) {
        return None;
    }
    serde_json::from_value(frame.swap_remove(2)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::lcu::models::LcuEventType;

    #[test]
    fn parses_event_frame() {
        let text = r#"[8,"OnJsonApiEvent",{"data":"Lobby","eventType":"Update","uri":"/x"}]"#;
        let event = parse_event(text).unwrap();
        assert_eq!(event.uri, "/x");
        assert_eq!(event.event_type, LcuEventType::Update);
        assert_eq!(event.data, "Lobby");
    }

    #[test]
    fn ignores_other_frames() {
        assert!(parse_event("").is_none());
        assert!(parse_event(r#"[0,"session","1",""]"#).is_none());
    }
}
