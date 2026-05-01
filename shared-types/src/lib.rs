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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BrowserStateSnapshot {
    pub browsers: Vec<DetectedBrowser>,
    pub version: u64,
}

/// Full tab metadata as reported by the browser extension.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TabInfo {
    pub id: i64,
    pub window_id: i64,
    pub url: String,
    pub title: String,
    pub pinned: bool,
    pub active: bool,
    pub discarded: bool,
    /// None means the tab is not in any group.
    pub group_id: Option<i64>,
}

/// Whether the browser extension is currently connected via WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExtensionStatus {
    pub connected: bool,
}

/// Commands sent from the backend to the extension over WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ToExtension {
    /// Request a fresh tab list.
    ListTabs,
    /// Read the full DOM of a tab.
    ReadDom { tab_id: i64 },
    /// Scroll a tab to the given scrollTop value.
    ScrollTab { tab_id: i64, target_top: i64 },
    /// Bring a tab to the foreground.
    ActivateTab { tab_id: i64 },
    /// Open a new tab at the given URL.
    OpenTab { url: String },
}

/// Messages sent from the extension to the backend over WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum FromExtension {
    /// Sent once when the extension connects; identifies which browser it's running in.
    Connected { browser: String },
    /// Response to ListTabs, or proactive update when tabs change.
    TabList { tabs: Vec<TabInfo> },
    /// Response to ReadDom.
    DomContent { tab_id: i64, html: String },
    /// Response to ScrollTab; reports actual scroll position after scrolling.
    ScrollDone { tab_id: i64, reached_bottom: bool },
    /// Generic error from the extension.
    Error { message: String },
}
