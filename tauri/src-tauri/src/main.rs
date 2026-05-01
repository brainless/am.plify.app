use std::path::Path;
use std::process::{Child, Command};
use std::sync::Mutex;

use tauri::Manager;

struct ApiChild(Mutex<Option<Child>>);

fn get_project_root() -> Option<std::path::PathBuf> {
    let current_dir = std::env::current_dir().ok()?;
    let mut dir: &Path = current_dir.as_path();
    for _ in 0..10 {
        if dir.join("project.conf").exists() {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
    }
    None
}

fn start_api(app: &tauri::AppHandle) -> Result<Child, String> {
    let project_root = get_project_root();

    if let Ok(path) = std::env::var("AMPLIFY_BACKEND_PATH") {
        let mut cmd = Command::new(&path);
        if let Some(ref root) = project_root {
            cmd.current_dir(root);
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            unsafe {
                cmd.pre_exec(|| {
                    libc::setpgid(0, 0);
                    Ok(())
                });
            }
        }
        return cmd
            .spawn()
            .map_err(|e| format!("failed to spawn AMPLIFY_BACKEND_PATH: {e}"));
    }

    let mut candidates = Vec::new();

    if let Ok(exe_dir) = app.path().executable_dir() {
        candidates.push(exe_dir.join("amplify-backend"));
    }

    if let Ok(resource_dir) = app.path().resource_dir() {
        candidates.push(resource_dir.join("amplify-backend"));
    }

    #[cfg(debug_assertions)]
    if let Ok(exe_dir) = app.path().executable_dir() {
        let mut dir: &Path = exe_dir.as_path();
        for _ in 0..8 {
            match dir.parent() {
                Some(p) => dir = p,
                None => break,
            }
            candidates.push(dir.join("target").join("debug").join("amplify-backend"));
        }
    }

    if cfg!(target_os = "windows") {
        for candidate in &mut candidates {
            candidate.set_extension("exe");
        }
    }

    let candidate = candidates
        .into_iter()
        .filter(|path| path.exists())
        .max_by_key(|path| {
            path.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        })
        .ok_or_else(|| {
            "amplify-backend sidecar not found (set AMPLIFY_BACKEND_PATH to the backend binary path)"
                .to_string()
        })?;

    eprintln!("[amplify] starting backend from: {}", candidate.display());
    let mut cmd = Command::new(&candidate);
    if let Some(ref root) = project_root {
        cmd.current_dir(root);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                libc::setpgid(0, 0);
                Ok(())
            });
        }
    }
    cmd.spawn()
        .map_err(|e| format!("failed to spawn amplify-backend sidecar: {e}"))
}

fn stop_api(app: &tauri::AppHandle) {
    eprintln!("[amplify] stopping backend...");
    if let Some(state) = app.try_state::<ApiChild>() {
        if let Some(mut child) = state.0.lock().ok().and_then(|mut g| g.take()) {
            let pid = child.id() as i32;

            #[cfg(unix)]
            {
                unsafe {
                    libc::kill(-pid, libc::SIGTERM);
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
                unsafe {
                    libc::kill(-pid, libc::SIGKILL);
                }
            }

            #[cfg(not(unix))]
            {
                let _ = child.kill();
            }

            let _ = child.wait();
            eprintln!("[amplify] backend stopped");
        }
    }
}

struct ApiStartError(Mutex<Option<String>>);

#[tauri::command]
fn api_start_error(state: tauri::State<ApiStartError>) -> Option<String> {
    state.0.lock().ok().and_then(|g| g.clone())
}

/// Returns the path to the built extension directory.
/// In debug builds uses the local extension/dist folder.
/// In release builds uses the bundled resource directory.
#[tauri::command]
fn get_extension_path(app: tauri::AppHandle) -> Result<String, String> {
    #[cfg(debug_assertions)]
    {
        if let Some(root) = get_project_root() {
            let path = root.join("extension").join("dist");
            if path.exists() {
                return Ok(path.to_string_lossy().into_owned());
            }
            // Extension not built yet — return the path anyway so the UI can show it
            return Ok(path.to_string_lossy().into_owned());
        }
    }

    app.path()
        .resource_dir()
        .map(|p| p.join("extension").to_string_lossy().into_owned())
        .map_err(|e| e.to_string())
}

/// Opens the browser's extension management page by spawning the browser executable directly.
/// Using the OS URL handler (e.g. `open "about:..."` on macOS) does not work for about: URLs
/// because no app registers that scheme. Spawning the executable with the URL as an argument
/// works reliably for all browser-specific URL schemes.
#[tauri::command]
fn open_extensions_page(browser: String, executable: String) -> Result<(), String> {
    let url = match browser.as_str() {
        "brave" => "brave://extensions",
        "edge" => "edge://extensions",
        "firefox" | "firefox_developer_edition" | "firefox_nightly" => {
            "about:debugging#/runtime/this-firefox"
        }
        _ => "chrome://extensions",
    };

    Command::new(&executable)
        .arg(url)
        .spawn()
        .map_err(|e| format!("failed to launch browser: {e}"))?;
    Ok(())
}

/// Opens any URL in the given browser executable. Used for about:addons and similar
/// pages that are not covered by open_extensions_page's fixed URL mapping.
#[tauri::command]
fn open_url_in_browser(url: String, executable: String) -> Result<(), String> {
    Command::new(&executable)
        .arg(&url)
        .spawn()
        .map_err(|e| format!("failed to launch browser: {e}"))?;
    Ok(())
}

/// Packages extension/dist/ into a .xpi file (zip with .xpi extension).
/// Firefox accepts .xpi files via about:addons → Install Add-on From File.
/// Firefox Developer Edition does not require the extension to be signed.
#[tauri::command]
fn package_extension_xpi() -> Result<String, String> {
    let root = get_project_root().ok_or("could not find project root")?;
    let dist = root.join("extension").join("dist");
    let xpi = root.join("extension").join("amplify-extension.xpi");

    if !dist.is_dir() {
        return Err(format!(
            "extension/dist not found at {} — run `npm run build` in extension/ first",
            dist.display()
        ));
    }

    // Remove stale .xpi if present
    let _ = std::fs::remove_file(&xpi);

    let output = Command::new("zip")
        .args(["-r", &xpi.to_string_lossy().into_owned(), "."])
        .current_dir(&dist)
        .output()
        .map_err(|e| format!("zip command failed: {e}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    Ok(xpi.to_string_lossy().into_owned())
}

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let error_state = match start_api(app.handle()) {
                Ok(child) => {
                    app.manage(ApiChild(Mutex::new(Some(child))));
                    ApiStartError(Mutex::new(None))
                }
                Err(err) => {
                    eprintln!("[amplify] {err}");
                    ApiStartError(Mutex::new(Some(err)))
                }
            };
            app.manage(error_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            api_start_error,
            get_extension_path,
            open_extensions_page,
            open_url_in_browser,
            package_extension_xpi,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                stop_api(&window.app_handle());
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            stop_api(app_handle);
        }
    });
}
