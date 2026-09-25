use tokio::sync::OnceCell;

use crate::clients::lcu::http::LcuHttp;
use crate::clients::sgp::http::SgpClient;
use crate::services::game_data::GameData;

/// Everything bound to one connection to one client process; dropped on disconnect.
pub struct LcuSession {
    pub http: LcuHttp,
    /// `None` when the server has no known SGP endpoint.
    pub sgp: Option<SgpClient>,
    pub game_data: OnceCell<GameData>,
}
