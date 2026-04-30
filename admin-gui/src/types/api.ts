export type HeartbeatResponse = { status: string, service: string, };


export type BrowserKind = "chrome" | "chromium" | "brave" | "edge" | "firefox" | "firefox_developer_edition" | "firefox_nightly" | "safari";


export type BrowserProfile = { id: string, name: string, path: string, is_running: boolean, };


export type DetectedBrowser = { kind: BrowserKind, executable: string, user_data_dir: string, profiles: Array<BrowserProfile>, is_running: boolean, };


export type LaunchBrowserRequest = { kind: BrowserKind, executable: string, 
/**
 * Chrome family: the user-data-dir root. Unused for Firefox.
 */
user_data_dir: string, 
/**
 * Chrome family: profile directory name (e.g. "Default", "Profile 1").
 * Firefox: relative or absolute profile path as stored in profiles.ini.
 */
profile_id: string, 
/**
 * Absolute filesystem path to the profile directory.
 */
profile_path: string, 
/**
 * Preferred debug port. Backend picks a free one if None or if the port is taken.
 */
debug_port: number | null, };


export type LaunchBrowserResponse = { debug_port: number, pid: number, };


export type BrowserStateSnapshot = { browsers: Array<DetectedBrowser>, version: bigint, };
