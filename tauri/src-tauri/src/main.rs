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
        .invoke_handler(tauri::generate_handler![api_start_error])
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
