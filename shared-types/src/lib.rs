use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct HeartbeatResponse {
    pub status: String,
    pub service: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum BrowserKind {
    Chrome,
    Chromium,
    Brave,
    Edge,
    Firefox,
    FirefoxDeveloperEdition,
    FirefoxNightly,
    Safari,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BrowserProfile {
    pub id: String,
    pub name: String,
    pub path: String,
    pub is_running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DetectedBrowser {
    pub kind: BrowserKind,
    pub executable: String,
    pub user_data_dir: String,
    pub profiles: Vec<BrowserProfile>,
    pub is_running: bool,
}
