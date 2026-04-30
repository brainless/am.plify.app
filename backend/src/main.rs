use actix_cors::Cors;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use rustenium::browsers::{BidiBrowser, ChromeBrowser, ChromeConfig, ChromeLaunchMode, FirefoxBrowser, FirefoxConfig, FirefoxLaunchMode};
use rustenium_bidi_definitions::browsing_context::commands::{GetTree, GetTreeMethod, GetTreeParams, BrowsingContextCommand};
use rustenium_bidi_definitions::browsing_context::results::GetTreeResult;
use rustenium_bidi_definitions::browsing_context::types::BrowsingContext;
use rustenium_bidi_definitions::storage::commands::{GetCookies, GetCookiesMethod, GetCookiesParams, StorageCommand};
use rustenium_bidi_definitions::storage::results::GetCookiesResult;
use rustenium_bidi_definitions::storage::types::{BrowsingContextPartitionDescriptor, BrowsingContextPartitionDescriptorType, PartitionDescriptor};
use rustenium_bidi_definitions::Command;
use serde::{Deserialize, Serialize};
use shared_types::{BrowserStateSnapshot, ConnectBrowserRequest, ConnectBrowserResponse, HeartbeatResponse, LaunchBrowserRequest, RedditLoginStatus, TabInfo};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{watch, RwLock};
use uuid::Uuid;

mod auth;
mod browsers;
mod config;

struct BrowserCache {
    browsers: Vec<shared_types::DetectedBrowser>,
    version: u64,
}

struct AppState {
    cache: Arc<RwLock<BrowserCache>>,
    notify: watch::Receiver<u64>,
    connections: Arc<RwLock<HashMap<String, BrowserConnection>>>,
}

enum BrowserConnection {
    Chrome(ChromeBrowser),
    Firefox(FirefoxBrowser),
}

impl BrowserConnection {
    async fn connect(kind: &shared_types::BrowserKind, debug_port: u16) -> Result<Self, String> {
        match kind {
            shared_types::BrowserKind::Chrome
            | shared_types::BrowserKind::Chromium
            | shared_types::BrowserKind::Brave
            | shared_types::BrowserKind::Edge => {
                let mut config = ChromeConfig {
                    launch_mode: ChromeLaunchMode::Remote(debug_port),
                    enable_bidi: true,
                    enable_cdp: false,
                    ..Default::default()
                };
                if config.driver_executable_path.is_empty() {
                    config.driver_executable_path = "chromedriver".to_string();
                }
                let browser = ChromeBrowser::new(config).await;
                Ok(BrowserConnection::Chrome(browser))
            }
            shared_types::BrowserKind::Firefox
            | shared_types::BrowserKind::FirefoxDeveloperEdition
            | shared_types::BrowserKind::FirefoxNightly => {
                let config = FirefoxConfig {
                    launch_mode: FirefoxLaunchMode::Remote(debug_port),
                    ..Default::default()
                };
                let browser = FirefoxBrowser::new(config).await;
                Ok(BrowserConnection::Firefox(browser))
            }
            shared_types::BrowserKind::Safari => {
                Err("Safari does not support WebDriver BiDi".into())
            }
        }
    }

    async fn list_tabs(&mut self) -> Result<Vec<TabInfo>, String> {
        tracing::info!("Listing tabs via BiDi getTree");
        let command = Command::BrowsingContext(BrowsingContextCommand::GetTree(GetTree {
            method: GetTreeMethod::GetTree,
            params: GetTreeParams {
                max_depth: Some(1),
                root: None,
            },
        }));

        let response = match self {
            BrowserConnection::Chrome(b) => b.send_command(command).await,
            BrowserConnection::Firefox(b) => b.send_command(command).await,
        };

        let response = response.map_err(|e| {
            tracing::error!("Failed to get tab tree: {e:?}");
            format!("Failed to get tab tree: {e:?}")
        })?;

        tracing::info!("getTree raw result: {}", serde_json::to_string(&response.result).unwrap_or_default());

        let result: GetTreeResult = serde_json::from_value(response.result)
            .map_err(|e| format!("Failed to parse tab tree: {e}"))?;

        tracing::info!("getTree returned {} contexts", result.contexts.inner().len());

        let mut tabs = Vec::new();
        for info in result.contexts.inner() {
            if info.parent.is_none() {
                let url = info.url.clone();
                let is_reddit = url.contains("reddit.com");
                tracing::info!("Tab: url={url}, is_reddit={is_reddit}");
                tabs.push(TabInfo {
                    context_id: info.context.as_ref().to_string(),
                    url,
                    title: String::new(),
                    is_reddit,
                });
            }
        }

        Ok(tabs)
    }

    async fn check_reddit_login(&mut self, context_id: &str) -> Result<RedditLoginStatus, String> {
        let bc = BrowsingContext::new(context_id.to_string());
        let partition = PartitionDescriptor::BrowsingContextPartitionDescriptor(
            BrowsingContextPartitionDescriptor::new(
                BrowsingContextPartitionDescriptorType::Context,
                bc,
            ),
        );

        let command = Command::Storage(StorageCommand::GetCookies(GetCookies {
            method: GetCookiesMethod::GetCookies,
            params: GetCookiesParams {
                filter: None,
                partition: Some(partition),
            },
        }));

        let response = match self {
            BrowserConnection::Chrome(b) => b.send_command(command).await,
            BrowserConnection::Firefox(b) => b.send_command(command).await,
        };

        let response = response.map_err(|e| format!("Failed to get cookies: {e:?}"))?;
        let result: GetCookiesResult = serde_json::from_value(response.result)
            .map_err(|e| format!("Failed to parse cookies: {e}"))?;

        let reddit_cookie = result.cookies.iter().find(|c| c.name == "reddit_session");
        let is_logged_in = reddit_cookie.is_some();

        let username = if is_logged_in {
            result.cookies.iter().find(|c| c.name == "session_tracker").and_then(|c| {
                if let rustenium_bidi_definitions::network::types::BytesValue::StringValue(sv) = &c.value {
                    Some(sv.value.clone())
                } else {
                    None
                }
            })
        } else {
            None
        };

        Ok(RedditLoginStatus {
            context_id: context_id.to_string(),
            is_logged_in,
            username,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InspectTabRequest {
    pub context_id: String,
}

#[get("/api/heartbeat")]
async fn heartbeat() -> impl Responder {
    let payload = HeartbeatResponse {
        status: "ok".to_string(),
        service: env!("CARGO_PKG_NAME").to_string(),
    };

    HttpResponse::Ok().json(payload)
}

#[get("/api/browsers")]
async fn list_browsers(state: web::Data<AppState>) -> impl Responder {
    let cache = state.cache.read().await;
    HttpResponse::Ok().json(&cache.browsers)
}

#[get("/api/browsers/running")]
async fn browsers_running(
    state: web::Data<AppState>,
    query: web::Query<HashMap<String, u64>>,
) -> impl Responder {
    let since = query.get("since").copied().unwrap_or(0);
    let mut rx = state.notify.clone();
    let _ = rx.borrow_and_update();

    {
        let cache = state.cache.read().await;
        if cache.version > since {
            return HttpResponse::Ok().json(BrowserStateSnapshot {
                browsers: cache.browsers.clone(),
                version: cache.version,
            });
        }
    }

    let timeout = tokio::time::sleep(Duration::from_secs(30));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            _ = rx.changed() => {
                let cache = state.cache.read().await;
                if cache.version > since {
                    return HttpResponse::Ok().json(BrowserStateSnapshot {
                        browsers: cache.browsers.clone(),
                        version: cache.version,
                    });
                }
            }
            _ = &mut timeout => {
                let cache = state.cache.read().await;
                return HttpResponse::Ok().json(BrowserStateSnapshot {
                    browsers: cache.browsers.clone(),
                    version: cache.version,
                });
            }
        }
    }
}

#[post("/api/browsers/launch")]
async fn launch_browser(body: web::Json<LaunchBrowserRequest>) -> impl Responder {
    match web::block(move || browsers::launch_browser(&body)).await {
        Ok(Ok(resp)) => HttpResponse::Ok().json(resp),
        Ok(Err(msg)) => HttpResponse::BadRequest().body(msg),
        Err(_) => HttpResponse::InternalServerError().body("launch task panicked"),
    }
}

#[post("/api/browsers/connect")]
async fn connect_browser(
    body: web::Json<ConnectBrowserRequest>,
    state: web::Data<AppState>,
) -> impl Responder {
    let req = body.into_inner();
    let debug_port = req.debug_port;
    let kind = req.kind.clone();

    tracing::info!("Connecting to browser: kind={:?}, port={}", kind, debug_port);

    let connection = match BrowserConnection::connect(&kind, debug_port).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to connect: {}", e);
            return HttpResponse::BadRequest().body(e);
        }
    };

    let connection_id = Uuid::new_v4().to_string();
    state.connections.write().await.insert(connection_id.clone(), connection);
    tracing::info!("Connected, connection_id={}", connection_id);

    HttpResponse::Ok().json(ConnectBrowserResponse { connection_id })
}

#[get("/api/browsers/{connection_id}/tabs")]
async fn list_tabs(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> impl Responder {
    let connection_id = path.into_inner();

    let mut connections = state.connections.write().await;
    let connection = match connections.get_mut(&connection_id) {
        Some(c) => c,
        None => return HttpResponse::NotFound().body("Connection not found"),
    };

    match connection.list_tabs().await {
        Ok(tabs) => HttpResponse::Ok().json(tabs),
        Err(e) => HttpResponse::InternalServerError().body(e),
    }
}

#[post("/api/browsers/{connection_id}/tabs/inspect")]
async fn inspect_tab(
    path: web::Path<String>,
    body: web::Json<InspectTabRequest>,
    state: web::Data<AppState>,
) -> impl Responder {
    let connection_id = path.into_inner();
    let context_id = body.context_id.clone();

    let mut connections = state.connections.write().await;
    let connection = match connections.get_mut(&connection_id) {
        Some(c) => c,
        None => return HttpResponse::NotFound().body("Connection not found"),
    };

    match connection.check_reddit_login(&context_id).await {
        Ok(status) => HttpResponse::Ok().json(status),
        Err(e) => HttpResponse::InternalServerError().body(e),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let backend_host = std::env::var("BACKEND_HOST")
        .ok()
        .or_else(|| config::read_project_conf("BACKEND_HOST"))
        .unwrap_or_else(|| "127.0.0.1".to_string());

    let backend_port: u16 = std::env::var("BACKEND_PORT")
        .ok()
        .or_else(|| config::read_project_conf("BACKEND_PORT"))
        .and_then(|v| v.parse().ok())
        .unwrap_or(8080);

    let gui_port: u16 = std::env::var("GUI_PORT")
        .ok()
        .or_else(|| config::read_project_conf("GUI_PORT"))
        .and_then(|v| v.parse().ok())
        .unwrap_or(3030);

    let admin_gui_port: u16 = std::env::var("ADMIN_GUI_PORT")
        .ok()
        .or_else(|| config::read_project_conf("ADMIN_GUI_PORT"))
        .and_then(|v| v.parse().ok())
        .unwrap_or(3031);

    let domain_name = std::env::var("DOMAIN_NAME")
        .ok()
        .or_else(|| config::read_project_conf("DOMAIN_NAME"));

    let initial_browsers = web::block(browsers::detect_browsers)
        .await
        .unwrap_or_default();

    let (notify_tx, notify_rx) = watch::channel(0u64);
    let cache = Arc::new(RwLock::new(BrowserCache {
        browsers: initial_browsers,
        version: 1,
    }));

    let cache_bg = cache.clone();
    let notify_tx_bg = notify_tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(2));
        loop {
            interval.tick().await;
            let snapshot = {
                let c = cache_bg.read().await;
                c.browsers.clone()
            };

            let mut updated = match tokio::task::spawn_blocking(move || {
                let mut browsers = snapshot;
                browsers::annotate_running_state(&mut browsers);
                browsers
            })
            .await
            {
                Ok(v) => v,
                Err(_) => continue,
            };

            let mut c = cache_bg.write().await;
            if !running_state_eq(&c.browsers, &updated) {
                c.browsers = updated;
                c.version += 1;
                let _ = notify_tx_bg.send(c.version);
            } else {
                let _ = std::mem::take(&mut updated);
            }
        }
    });

    println!(
        "Backend listening on http://{}:{}",
        backend_host, backend_port
    );

    let gui_origin_ip = format!("http://127.0.0.1:{gui_port}");
    let gui_origin_local = format!("http://localhost:{gui_port}");
    let admin_origin_ip = format!("http://127.0.0.1:{admin_gui_port}");
    let admin_origin_local = format!("http://localhost:{admin_gui_port}");
    let domain_origin_https = domain_name.as_deref().map(|d| format!("https://{d}"));
    let domain_origin_http = domain_name.as_deref().map(|d| format!("http://{d}"));

    let app_state = web::Data::new(AppState {
        cache: cache.clone(),
        notify: notify_rx,
        connections: Arc::new(RwLock::new(HashMap::new())),
    });

    HttpServer::new(move || {
        let mut cors = Cors::default()
            .allowed_origin(&gui_origin_ip)
            .allowed_origin(&gui_origin_local)
            .allowed_origin(&admin_origin_ip)
            .allowed_origin(&admin_origin_local);

        if let Some(ref origin) = domain_origin_https {
            cors = cors.allowed_origin(origin);
        }
        if let Some(ref origin) = domain_origin_http {
            cors = cors.allowed_origin(origin);
        }

        let cors = cors
            .allowed_methods(vec!["GET", "POST"])
            .allow_any_header();

        App::new()
            .wrap(cors)
            .app_data(app_state.clone())
            .app_data(web::JsonConfig::default())
            .service(heartbeat)
            .service(list_browsers)
            .service(browsers_running)
            .service(launch_browser)
            .service(connect_browser)
            .service(list_tabs)
            .service(inspect_tab)
    })
    .bind((backend_host.as_str(), backend_port))?
    .run()
    .await
}

fn running_state_eq(a: &[shared_types::DetectedBrowser], b: &[shared_types::DetectedBrowser]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).all(|(a, b)| {
        if a.is_running != b.is_running || a.profiles.len() != b.profiles.len() {
            return false;
        }
        a.profiles
            .iter()
            .zip(b.profiles.iter())
            .all(|(pa, pb)| pa.is_running == pb.is_running)
    })
}
