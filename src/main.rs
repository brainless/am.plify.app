#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

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
    sender: Option<tokio::sync::mpsc::UnboundedSender<String>>,
    #[serde(skip)]
    receiver: Option<tokio::sync::mpsc::UnboundedReceiver<String>>,
}

impl Default for DesktopGuiApp {
    fn default() -> Self {
        Self {
            search_text: String::new(),
            status: String::new(),
            sender: None,
            receiver: None,
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
            while let Ok(msg) = rx.try_recv() {
                self.status = msg;
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
        });

        // Main content column, responsive
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(&self.status);
        });
    }
}

async fn check_chrome() -> String {
    log::info!("Checking Chrome CDP connection...");
    match headless_chrome::Browser::connect("http://127.0.0.1:9222".to_string()) {
        Ok(_) => {
            log::info!("Connected to Chrome CDP");
            "Connected to Chrome CDP".to_string()
        }
        Err(e) => {
            log::info!("Chrome not running, attempting to launch: {}", e);
            match std::process::Command::new(r"C:\Program Files\Google\Chrome\Application\chrome.exe")
                .arg("--remote-debugging-port=9222")
                .arg("--new-window")
                .spawn()
            {
                Ok(_) => {
                    log::info!("Launched Chrome, waiting...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                    match headless_chrome::Browser::connect("http://127.0.0.1:9222".to_string()) {
                        Ok(_) => {
                            log::info!("Connected after launch");
                            "Launched Chrome and connected to CDP".to_string()
                        }
                        Err(e) => {
                            log::info!("Failed to connect after launch: {}", e);
                            format!("Launched Chrome but failed to connect: {}", e)
                        }
                    }
                }
                Err(e) => {
                    log::info!("Failed to launch Chrome: {}", e);
                    format!("Failed to launch Chrome: {}", e)
                }
            }
        }
    }
}