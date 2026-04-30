use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LaunchBrowserRequest {
    pub kind: BrowserKind,
    pub executable: String,
    /// Chrome family: the user-data-dir root. Unused for Firefox.
    pub user_data_dir: String,
    /// Chrome family: profile directory name (e.g. "Default", "Profile 1").
    /// Firefox: relative or absolute profile path as stored in profiles.ini.
    pub profile_id: String,
    /// Absolute filesystem path to the profile directory.
    pub profile_path: String,
    /// Preferred debug port. Backend picks a free one if None or if the port is taken.
    pub debug_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LaunchBrowserResponse {
    pub debug_port: u16,
    pub pid: u32,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BrowserStateSnapshot {
    pub browsers: Vec<DetectedBrowser>,
    pub version: u64,
}
