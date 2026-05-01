// Matches shared-types/src/lib.rs — keep in sync with TabInfo, FromExtension, ToExtension.
// serde uses snake_case for TabInfo fields, camelCase for enum variant names and their fields.

export interface TabInfo {
  id: number;
  window_id: number;
  url: string;
  title: string;
  pinned: boolean;
  active: boolean;
  discarded: boolean;
  group_id: number | null;
}

export type FromExtension =
  | { type: "connected"; browser: string }
  | { type: "tabList"; tabs: TabInfo[] }
  | { type: "domContent"; tabId: number; html: string }
  | { type: "scrollDone"; tabId: number; reachedBottom: boolean }
  | { type: "error"; message: string };

export type ToExtension =
  | { type: "listTabs" }
  | { type: "readDom"; tabId: number }
  | { type: "scrollTab"; tabId: number; targetTop: number }
  | { type: "activateTab"; tabId: number }
  | { type: "openTab"; url: string };
