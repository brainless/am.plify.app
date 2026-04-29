use shared_types::{BrowserKind, BrowserProfile, DetectedBrowser};
use std::path::{Path, PathBuf};

pub fn detect_browsers() -> Vec<DetectedBrowser> {
    let mut results = Vec::new();

    for candidate in platform::candidates() {
        let Some(executable) = resolve_executable(&candidate.exe_hints) else {
            continue;
        };
        if !candidate.data_dir.is_dir() {
            continue;
        }
        let profiles = match candidate.kind {
            BrowserKind::Firefox
            | BrowserKind::FirefoxDeveloperEdition
            | BrowserKind::FirefoxNightly => {
                parse_firefox_profiles(&candidate.data_dir)
            }
            BrowserKind::Safari => vec![BrowserProfile {
                id: "default".into(),
                name: "Default".into(),
                path: candidate.data_dir.to_string_lossy().into_owned(),
                is_running: false,
            }],
            _ => parse_chrome_profiles(&candidate.data_dir),
        };
        results.push(DetectedBrowser {
            kind: candidate.kind,
            executable: executable.to_string_lossy().into_owned(),
            user_data_dir: candidate.data_dir.to_string_lossy().into_owned(),
            profiles,
            is_running: false,
        });
    }

    annotate_running_state(&mut results);
    results
}

struct Candidate {
    kind: BrowserKind,
    /// Ordered list of paths to probe. Absolute paths are checked directly;
    /// bare names (no path separator) are resolved via PATH.
    exe_hints: Vec<PathBuf>,
    data_dir: PathBuf,
}

fn resolve_executable(hints: &[PathBuf]) -> Option<PathBuf> {
    for hint in hints {
        if hint.as_os_str().is_empty() {
            continue;
        }
        if hint.is_absolute() {
            if hint.is_file() {
                return Some(hint.clone());
            }
        } else if let Ok(path) = which::which(hint) {
            return Some(path);
        }
    }
    None
}

fn parse_chrome_profiles(data_dir: &Path) -> Vec<BrowserProfile> {
    let Ok(content) = std::fs::read_to_string(data_dir.join("Local State")) else {
        return vec![];
    };
    let Ok(json): Result<serde_json::Value, _> = serde_json::from_str(&content) else {
        return vec![];
    };
    let Some(info_cache) = json
        .get("profile")
        .and_then(|p| p.get("info_cache"))
        .and_then(|v| v.as_object())
    else {
        return vec![];
    };

    info_cache
        .iter()
        .map(|(dir_name, info)| {
            let name = info
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or(dir_name)
                .to_string();
            BrowserProfile {
                id: dir_name.clone(),
                name,
                path: data_dir.join(dir_name).to_string_lossy().into_owned(),
                is_running: false,
            }
        })
        .collect()
}

fn parse_firefox_profiles(root_dir: &Path) -> Vec<BrowserProfile> {
    let Ok(content) = std::fs::read_to_string(root_dir.join("profiles.ini")) else {
        return vec![];
    };
    parse_ini_profiles(&content, root_dir)
}

fn parse_ini_profiles(content: &str, root_dir: &Path) -> Vec<BrowserProfile> {
    let mut profiles = Vec::new();
    let mut in_profile = false;
    let mut name: Option<String> = None;
    let mut path: Option<String> = None;
    let mut is_relative = false;

    let flush = |name: &mut Option<String>,
                 path: &mut Option<String>,
                 is_relative: &mut bool,
                 profiles: &mut Vec<BrowserProfile>| {
        if let Some(p) = path.take() {
            let abs = if *is_relative {
                root_dir.join(&p)
            } else {
                PathBuf::from(&p)
            };
            profiles.push(BrowserProfile {
                id: p.clone(),
                name: name.take().unwrap_or_else(|| p.clone()),
                path: abs.to_string_lossy().into_owned(),
                is_running: false,
            });
        } else {
            name.take();
        }
        *is_relative = false;
    };

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            if in_profile {
                flush(&mut name, &mut path, &mut is_relative, &mut profiles);
            }
            let section = &line[1..line.len() - 1];
            in_profile = section.starts_with("Profile");
            continue;
        }
        if !in_profile {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            match k.trim() {
                "Name" => name = Some(v.trim().to_string()),
                "Path" => path = Some(v.trim().to_string()),
                "IsRelative" => is_relative = v.trim() == "1",
                _ => {}
            }
        }
    }
    if in_profile {
        flush(&mut name, &mut path, &mut is_relative, &mut profiles);
    }

    profiles
}

#[cfg(target_os = "macos")]
mod platform {
    use super::{BrowserKind, Candidate};
    use std::path::PathBuf;

    pub fn candidates() -> Vec<Candidate> {
        let config = dirs::config_dir().unwrap_or_default();
        let home = dirs::home_dir().unwrap_or_default();
        vec![
            Candidate {
                kind: BrowserKind::Chrome,
                exe_hints: vec![PathBuf::from(
                    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
                )],
                data_dir: config.join("Google/Chrome"),
            },
            Candidate {
                kind: BrowserKind::Chromium,
                exe_hints: vec![PathBuf::from(
                    "/Applications/Chromium.app/Contents/MacOS/Chromium",
                )],
                data_dir: config.join("Chromium"),
            },
            Candidate {
                kind: BrowserKind::Brave,
                exe_hints: vec![PathBuf::from(
                    "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
                )],
                data_dir: config.join("BraveSoftware/Brave-Browser"),
            },
            Candidate {
                kind: BrowserKind::Edge,
                exe_hints: vec![PathBuf::from(
                    "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
                )],
                data_dir: config.join("Microsoft Edge"),
            },
            Candidate {
                kind: BrowserKind::Firefox,
                exe_hints: vec![PathBuf::from(
                    "/Applications/Firefox.app/Contents/MacOS/firefox",
                )],
                data_dir: config.join("Firefox"),
            },
            Candidate {
                kind: BrowserKind::FirefoxDeveloperEdition,
                exe_hints: vec![PathBuf::from(
                    "/Applications/Firefox Developer Edition.app/Contents/MacOS/firefox",
                )],
                data_dir: config.join("Firefox"),
            },
            Candidate {
                kind: BrowserKind::FirefoxNightly,
                exe_hints: vec![PathBuf::from(
                    "/Applications/Firefox Nightly.app/Contents/MacOS/firefox",
                )],
                data_dir: config.join("Firefox"),
            },
            Candidate {
                kind: BrowserKind::Safari,
                exe_hints: vec![PathBuf::from(
                    "/Applications/Safari.app/Contents/MacOS/Safari",
                )],
                data_dir: home.join("Library/Safari"),
            },
        ]
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::{BrowserKind, Candidate};
    use std::path::PathBuf;

    pub fn candidates() -> Vec<Candidate> {
        let config = dirs::config_dir().unwrap_or_default();
        let home = dirs::home_dir().unwrap_or_default();
        vec![
            Candidate {
                kind: BrowserKind::Chrome,
                exe_hints: vec![
                    PathBuf::from("google-chrome"),
                    PathBuf::from("google-chrome-stable"),
                ],
                data_dir: config.join("google-chrome"),
            },
            Candidate {
                kind: BrowserKind::Chromium,
                exe_hints: vec![
                    PathBuf::from("chromium"),
                    PathBuf::from("chromium-browser"),
                ],
                data_dir: config.join("chromium"),
            },
            Candidate {
                kind: BrowserKind::Brave,
                exe_hints: vec![PathBuf::from("brave-browser")],
                data_dir: config.join("BraveSoftware/Brave-Browser"),
            },
            Candidate {
                kind: BrowserKind::Edge,
                exe_hints: vec![PathBuf::from("microsoft-edge")],
                data_dir: config.join("microsoft-edge"),
            },
            Candidate {
                kind: BrowserKind::Firefox,
                exe_hints: vec![PathBuf::from("firefox")],
                data_dir: home.join(".mozilla/firefox"),
            },
            Candidate {
                kind: BrowserKind::FirefoxDeveloperEdition,
                exe_hints: vec![PathBuf::from("firefox-developer-edition")],
                data_dir: home.join(".mozilla/firefox"),
            },
            Candidate {
                kind: BrowserKind::FirefoxNightly,
                exe_hints: vec![
                    PathBuf::from("firefox-nightly"),
                    PathBuf::from("firefox-trunk"),
                ],
                data_dir: home.join(".mozilla/firefox"),
            },
        ]
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use super::{BrowserKind, Candidate};
    use std::path::PathBuf;

    pub fn candidates() -> Vec<Candidate> {
        let local = dirs::data_local_dir().unwrap_or_default();
        let roaming = dirs::config_dir().unwrap_or_default();
        let pf = PathBuf::from(
            std::env::var("ProgramFiles").unwrap_or_else(|_| "C:\\Program Files".into()),
        );
        let pf86 = PathBuf::from(
            std::env::var("ProgramFiles(x86)")
                .unwrap_or_else(|_| "C:\\Program Files (x86)".into()),
        );
        vec![
            Candidate {
                kind: BrowserKind::Chrome,
                exe_hints: vec![
                    local.join("Google\\Chrome\\Application\\chrome.exe"),
                    pf.join("Google\\Chrome\\Application\\chrome.exe"),
                ],
                data_dir: local.join("Google\\Chrome\\User Data"),
            },
            Candidate {
                kind: BrowserKind::Chromium,
                exe_hints: vec![local.join("Chromium\\Application\\chrome.exe")],
                data_dir: local.join("Chromium\\User Data"),
            },
            Candidate {
                kind: BrowserKind::Brave,
                exe_hints: vec![
                    local.join("BraveSoftware\\Brave-Browser\\Application\\brave.exe"),
                ],
                data_dir: local.join("BraveSoftware\\Brave-Browser\\User Data"),
            },
            Candidate {
                kind: BrowserKind::Edge,
                exe_hints: vec![
                    pf86.join("Microsoft\\Edge\\Application\\msedge.exe"),
                    pf.join("Microsoft\\Edge\\Application\\msedge.exe"),
                ],
                data_dir: local.join("Microsoft\\Edge\\User Data"),
            },
            Candidate {
                kind: BrowserKind::Firefox,
                exe_hints: vec![
                    pf.join("Mozilla Firefox\\firefox.exe"),
                    pf86.join("Mozilla Firefox\\firefox.exe"),
                ],
                data_dir: roaming.join("Mozilla\\Firefox"),
            },
            Candidate {
                kind: BrowserKind::FirefoxDeveloperEdition,
                exe_hints: vec![
                    pf.join("Firefox Developer Edition\\firefox.exe"),
                    pf86.join("Firefox Developer Edition\\firefox.exe"),
                ],
                data_dir: roaming.join("Mozilla\\Firefox"),
            },
            Candidate {
                kind: BrowserKind::FirefoxNightly,
                exe_hints: vec![
                    pf.join("Firefox Nightly\\firefox.exe"),
                    pf86.join("Firefox Nightly\\firefox.exe"),
                ],
                data_dir: roaming.join("Mozilla\\Firefox"),
            },
        ]
    }
}

fn annotate_running_state(browsers: &mut Vec<DetectedBrowser>) {
    use std::collections::HashSet;
    use sysinfo::{ProcessesToUpdate, System};

    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, false);

    for browser in browsers.iter_mut() {
        let exe = Path::new(&browser.executable);

        match browser.kind {
            BrowserKind::Firefox
            | BrowserKind::FirefoxDeveloperEdition
            | BrowserKind::FirefoxNightly => {
                browser.is_running = sys
                    .processes()
                    .values()
                    .any(|p| p.exe().is_some_and(|e| e == exe));
                for profile in browser.profiles.iter_mut() {
                    profile.is_running = is_firefox_profile_locked(Path::new(&profile.path));
                }
            }
            BrowserKind::Safari => {
                browser.is_running = sys
                    .processes()
                    .values()
                    .any(|p| p.exe().is_some_and(|e| e == exe));
                for profile in browser.profiles.iter_mut() {
                    profile.is_running = browser.is_running;
                }
            }
            _ => {
                // Chrome family: parse --profile-directory=<id> from process args
                let matching: Vec<_> = sys
                    .processes()
                    .values()
                    .filter(|p| p.exe().is_some_and(|e| e == exe))
                    .collect();

                browser.is_running = !matching.is_empty();

                let active_profiles: HashSet<String> = matching
                    .iter()
                    .flat_map(|p| p.cmd().iter())
                    .filter_map(|arg| {
                        arg.to_str()
                            .and_then(|s| s.strip_prefix("--profile-directory="))
                            .map(|s| s.to_string())
                    })
                    .collect();

                for profile in browser.profiles.iter_mut() {
                    profile.is_running = active_profiles.contains(&profile.id);
                }
            }
        }
    }
}

#[cfg(unix)]
fn is_firefox_profile_locked(profile_path: &Path) -> bool {
    // Linux: `lock` symlink (points to a network address, not a real path).
    // macOS: `.parentlock` regular file. Check both for cross-distro safety.
    profile_path.join("lock").symlink_metadata().is_ok()
        || profile_path.join(".parentlock").exists()
}

#[cfg(windows)]
fn is_firefox_profile_locked(profile_path: &Path) -> bool {
    profile_path.join("parent.lock").exists()
}
