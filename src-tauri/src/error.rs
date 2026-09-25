use serde::{Serialize, Serializer};

pub type Result<T, E = AppError> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("TLS 初始化失败: {0}")]
    Tls(#[from] native_tls::Error),

    #[error("HTTP 请求失败: {}", error_chain(.0))]
    Http(#[from] reqwest::Error),

    #[error("LCU 返回 {status}: {path}")]
    LcuStatus { status: u16, path: String },

    #[error("WebSocket 错误: {0}")]
    WebSocket(Box<tokio_tungstenite::tungstenite::Error>),

    #[error("{0}超时")]
    Timeout(&'static str),

    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Message(String),
}

impl From<tokio_tungstenite::tungstenite::Error> for AppError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        Self::WebSocket(Box::new(err))
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

/// reqwest hides the interesting part (e.g. the TLS failure) in the source chain.
fn error_chain(err: &dyn std::error::Error) -> String {
    let mut msg = err.to_string();
    let mut source = err.source();
    while let Some(s) = source {
        msg.push_str(": ");
        msg.push_str(&s.to_string());
        source = s.source();
    }
    msg
}
