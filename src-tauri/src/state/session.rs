use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::OnceCell;

use crate::clients::lcu::http::LcuHttp;
use crate::clients::sgp::http::SgpClient;
use crate::services::game_data::GameData;

/// Everything bound to one connection to one client process; dropped on disconnect.
pub struct LcuSession {
    /// Unique even when reconnecting to the same account and server.
    pub id: u64,
    pub http: LcuHttp,
    /// `None` when the server has no known SGP endpoint.
    pub sgp: Option<SgpClient>,
    pub game_data: OnceCell<GameData>,
}

pub fn next_session_id() -> u64 {
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}
