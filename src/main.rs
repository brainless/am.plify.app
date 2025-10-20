#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> eframe::Result {
    env_logger::Builder::from_default_env().filter_level(log::LevelFilter::Info).init(); // Log to stderr (defaults to info level).

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([600.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native(
        "am.plify.app",
        native_options,
        Box::new(|cc| Ok(Box::new(DesktopGuiApp::new(cc)))),
    )
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct DesktopGuiApp {
    search_text: String,
    status: String,
    #[serde(skip)]
    sender: Option<tokio::sync::mpsc::UnboundedSender<(String, Option<headless_chrome::Browser>)>>,
    #[serde(skip)]
    receiver: Option<tokio::sync::mpsc::UnboundedReceiver<(String, Option<headless_chrome::Browser>)>>,
    #[serde(skip)]
    browser: Option<headless_chrome::Browser>,
    current_page: Page,
    chrome_profile_path: String,
}

#[derive(serde::Deserialize, serde::Serialize, PartialEq)]
enum Page {
    Main,
    Settings,
}

impl Default for DesktopGuiApp {
    fn default() -> Self {
        Self {
            search_text: String::new(),
            status: String::new(),
            sender: None,
            receiver: None,
            browser: None,
            current_page: Page::Main,
            chrome_profile_path: String::new(),
        }
    }
}

impl DesktopGuiApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app: DesktopGuiApp = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        app.sender = Some(tx);
        app.receiver = Some(rx);
        app
    }
}

impl eframe::App for DesktopGuiApp {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll for messages
        if let Some(rx) = &mut self.receiver {
            while let Ok((msg, browser)) = rx.try_recv() {
                self.status = msg;
                if browser.is_some() {
                    self.browser = browser;
                }
            }
        }

        // Top navigation with search
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Search:");
                ui.text_edit_singleline(&mut self.search_text);
            });
        });

        // Left sidebar, 300px wide
        egui::SidePanel::left("sidebar").show(ctx, |ui| {
            ui.set_width(300.0);
            if ui.button("Twitter").clicked() {
                log::info!("Twitter button clicked");
                if let Some(tx) = &self.sender {
                    let tx = tx.clone();
                    tokio::spawn(async move {
                        let result = check_chrome().await;
                        let _ = tx.send(result);
                    });
                }
            }
            
            // Add spacer and Settings button at bottom
            ui.add_space(ui.available_height() - 30.0);
            if ui.button("Settings").clicked() {
                log::info!("Settings button clicked");
                self.current_page = Page::Settings;
            }
        });

        // Main content column, responsive
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_page {
                Page::Main => {
                    ui.label(&self.status);
                }
                Page::Settings => {
                    ui.heading("Settings");
                    
                    ui.separator();
                    ui.heading("Browser profile");
                    ui.label("Point to the path where your Chrome user data is stored:");
                    
                    // Detect OS and show appropriate Chrome path
                    let detected_path = detect_chrome_profile_path();
                    if let Some(path) = detected_path {
                        ui.label(format!("Detected Chrome path for your OS:"));
                        ui.monospace(path);
                    } else {
                        ui.label("Could not detect Chrome path. Here are the default locations:");
                        ui.monospace("Windows: C:\\Users\\YourUsername\\AppData\\Local\\Google\\Chrome\\User Data");
                        ui.monospace("Mac: ~/Library/Application Support/Google/Chrome");
                        ui.monospace("Linux: ~/.config/google-chrome");
                    }
                    
                    ui.add_space(10.0);
                    
if ui.button("Select Chrome Profile Folder").clicked() {
                        let mut dialog = rfd::FileDialog::new()
                            .set_title("Select Chrome Profile Directory");
                        
                        // Set the detected path as the starting directory
                        let detected_path = detect_chrome_profile_path();
                        if let Some(path_str) = detected_path {
                            let path = std::path::Path::new(&path_str);
                            if path.exists() {
                                dialog = dialog.set_directory(path);
                            }
                        }
                        
                        if let Some(path) = dialog.pick_folder() {
                            self.chrome_profile_path = path.display().to_string();
                        }
                    }

fn detect_chrome_profile_path() -> Option<String> {
    use std::path::PathBuf;
    
    if cfg!(target_os = "windows") {
        // Try to get the user's home directory and construct the path
        if let Some(home_dir) = std::env::var_os("USERPROFILE") {
            let path = PathBuf::from(home_dir)
                .join("AppData")
                .join("Local")
                .join("Google")
                .join("Chrome")
                .join("User Data");
            
            if path.exists() {
                return Some(path.to_string_lossy().to_string());
            }
        }
        Some("C:\\Users\\YourUsername\\AppData\\Local\\Google\\Chrome\\User Data".to_string())
    } else if cfg!(target_os = "macos") {
        if let Some(home_dir) = std::env::var_os("HOME") {
            let path = PathBuf::from(home_dir)
                .join("Library")
                .join("Application Support")
                .join("Google")
                .join("Chrome");
            
            if path.exists() {
                return Some(path.to_string_lossy().to_string());
            }
        }
        Some("~/Library/Application Support/Google/Chrome".to_string())
    } else if cfg!(target_os = "linux") {
        if let Some(home_dir) = std::env::var_os("HOME") {
            let path = PathBuf::from(home_dir)
                .join(".config")
                .join("google-chrome");
            
            if path.exists() {
                return Some(path.to_string_lossy().to_string());
            }
        }
        Some("~/.config/google-chrome".to_string())
    } else {
        None
    }
}
                    
                    if !self.chrome_profile_path.is_empty() {
                        ui.add_space(5.0);
                        ui.label("Selected path:");
                        ui.monospace(&self.chrome_profile_path);
                    }
                }
            }
        });
    }
}

async fn check_chrome() -> (String, Option<headless_chrome::Browser>) {
    log::info!("Launching Chrome with debugging enabled");
    match headless_chrome::Browser::new(headless_chrome::LaunchOptions {
        headless: false,
        args: vec![
            std::ffi::OsStr::new("--remote-debugging-port=9222"),
            std::ffi::OsStr::new("--remote-debugging-address=127.0.0.1"),
            std::ffi::OsStr::new("--disable-ipv6"),
            std::ffi::OsStr::new("--user-data-dir=./chrome-profile"),
        ],
        ..Default::default()
    }) {
        Ok(browser) => {
            log::info!("Connected to Chrome CDP");
            let tabs = browser.get_tabs().lock().unwrap();
            let current_url = if let Some(tab) = tabs.first() {
                let url = tab.get_url();
                log::info!("Current URL: {}", url);
                url
            } else {
                "No tabs found".to_string()
            };
            drop(tabs); // Explicitly drop the lock guard
            ("Launched Chrome and connected to CDP".to_string(), Some(browser))
        }
        Err(e) => {
            log::info!("Failed to launch Chrome: {:?}", e);
            (format!("Failed to launch Chrome: {:?}", e), None)
        }
    }
}