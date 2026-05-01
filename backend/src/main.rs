use actix_cors::Cors;
use actix_web::{get, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use futures_util::StreamExt;
use shared_types::{
    BrowserStateSnapshot, ExtensionStatus, FromExtension, HeartbeatResponse, TabInfo,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{watch, Mutex, RwLock};

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
    /// The active extension WebSocket session, if any.
    extension_session: Arc<Mutex<Option<actix_ws::Session>>>,
    /// Most recent tab list pushed by the extension.
    extension_tabs: Arc<RwLock<Vec<TabInfo>>>,
}

// ---------------------------------------------------------------------------
// Browser detection endpoints (unchanged from before)
// ---------------------------------------------------------------------------

#[get("/api/heartbeat")]
async fn heartbeat() -> impl Responder {
    HttpResponse::Ok().json(HeartbeatResponse {
        status: "ok".to_string(),
        service: env!("CARGO_PKG_NAME").to_string(),
    })
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

// ---------------------------------------------------------------------------
// Extension WebSocket endpoint
// ---------------------------------------------------------------------------

#[get("/api/extension/ws")]
async fn extension_ws(
    req: HttpRequest,
    stream: web::Payload,
    state: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let (res, session, msg_stream) = actix_ws::handle(&req, stream)?;

    tracing::info!("Extension WebSocket connected");
    *state.extension_session.lock().await = Some(session.clone());

    actix_web::rt::spawn(handle_extension_messages(
        state.into_inner(),
        session,
        msg_stream,
    ));

    Ok(res)
}

async fn handle_extension_messages(
    state: Arc<AppState>,
    mut session: actix_ws::Session,
    mut stream: actix_ws::MessageStream,
) {
    while let Some(Ok(msg)) = stream.next().await {
        match msg {
            actix_ws::Message::Text(text) => {
                match serde_json::from_str::<FromExtension>(&text) {
                    Ok(FromExtension::Connected { browser }) => {
                        tracing::info!("Extension identified: browser={browser}");
                    }
                    Ok(FromExtension::TabList { tabs }) => {
                        tracing::info!("Extension pushed {} tabs", tabs.len());
                        *state.extension_tabs.write().await = tabs;
                    }
                    Ok(FromExtension::DomContent { tab_id, .. }) => {
                        tracing::info!("Received DOM for tab {tab_id}");
                        // TODO: route to waiting request or job
                    }
                    Ok(FromExtension::ScrollDone { tab_id, reached_bottom }) => {
                        tracing::info!(
                            "Scroll done for tab {tab_id}, reached_bottom={reached_bottom}"
                        );
                        // TODO: route to waiting request or job
                    }
                    Ok(FromExtension::Error { message }) => {
                        tracing::error!("Extension error: {message}");
                    }
                    Err(e) => {
                        tracing::warn!("Unparseable extension message: {e}");
                    }
                }
            }
            actix_ws::Message::Ping(bytes) => {
                if session.pong(&bytes).await.is_err() {
                    break;
                }
            }
            actix_ws::Message::Close(_) => break,
            _ => {}
        }
    }

    tracing::info!("Extension WebSocket disconnected");
    *state.extension_session.lock().await = None;
    state.extension_tabs.write().await.clear();
}

// ---------------------------------------------------------------------------
// Extension REST endpoints (polled by admin-gui)
// ---------------------------------------------------------------------------

#[get("/api/extension/status")]
async fn extension_status(state: web::Data<AppState>) -> impl Responder {
    let connected = state.extension_session.lock().await.is_some();
    HttpResponse::Ok().json(ExtensionStatus { connected })
}

#[get("/api/extension/tabs")]
async fn extension_tabs(state: web::Data<AppState>) -> impl Responder {
    let tabs = state.extension_tabs.read().await.clone();
    HttpResponse::Ok().json(tabs)
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

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
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(2));
        loop {
            interval.tick().await;
            let snapshot = {
                let c = cache_bg.read().await;
                c.browsers.clone()
            };

            let updated = match tokio::task::spawn_blocking(move || {
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
                let _ = notify_tx.send(c.version);
            }
        }
    });

    println!("Backend listening on http://{}:{}", backend_host, backend_port);

    let gui_origin_ip = format!("http://127.0.0.1:{gui_port}");
    let gui_origin_local = format!("http://localhost:{gui_port}");
    let admin_origin_ip = format!("http://127.0.0.1:{admin_gui_port}");
    let admin_origin_local = format!("http://localhost:{admin_gui_port}");
    let domain_origin_https = domain_name.as_deref().map(|d| format!("https://{d}"));
    let domain_origin_http = domain_name.as_deref().map(|d| format!("http://{d}"));

    let app_state = web::Data::new(AppState {
        cache: cache.clone(),
        notify: notify_rx,
        extension_session: Arc::new(Mutex::new(None)),
        extension_tabs: Arc::new(RwLock::new(Vec::new())),
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
            .service(extension_ws)
            .service(extension_status)
            .service(extension_tabs)
    })
    .bind((backend_host.as_str(), backend_port))?
    .run()
    .await
}

fn running_state_eq(
    a: &[shared_types::DetectedBrowser],
    b: &[shared_types::DetectedBrowser],
) -> bool {
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
