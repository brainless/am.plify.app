export type HeartbeatResponse = { status: string; service: string };

export type BrowserKind =
  | "chrome"
  | "chromium"
  | "brave"
  | "edge"
  | "firefox"
  | "firefox_developer_edition"
  | "firefox_nightly"
  | "safari";

export type BrowserProfile = { id: string; name: string; path: string; is_running: boolean };

export type DetectedBrowser = {
  kind: BrowserKind;
  executable: string;
  user_data_dir: string;
  profiles: Array<BrowserProfile>;
  is_running: boolean;
};

export type BrowserStateSnapshot = { browsers: Array<DetectedBrowser>; version: number };

export type TabInfo = {
  id: number;
  window_id: number;
  url: string;
  title: string;
  pinned: boolean;
  active: boolean;
  discarded: boolean;
  /** null means not in any group */
  group_id: number | null;
};

export type ExtensionStatus = { connected: boolean };

/** Commands sent from the backend to the extension over WebSocket. */
export type ToExtension =
  | { type: "listTabs" }
  | { type: "readDom"; tab_id: number }
  | { type: "scrollTab"; tab_id: number; target_top: number }
  | { type: "activateTab"; tab_id: number }
  | { type: "openTab"; url: string };

/** Messages sent from the extension to the backend over WebSocket. */
export type FromExtension =
  | { type: "connected"; browser: string }
  | { type: "tabList"; tabs: Array<TabInfo> }
  | { type: "domContent"; tab_id: number; html: string }
  | { type: "scrollDone"; tab_id: number; reached_bottom: boolean }
  | { type: "error"; message: string };
