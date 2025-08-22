use actix_web::{web, HttpResponse, Result};
use serde_json::json;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/health", web::get().to(health_check))
        .route("/hello", web::get().to(hello_world));
}

async fn health_check() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

async fn hello_world() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(json!({
        "message": "Hello from Amplify Backend!",
        "service": "amplify-api",
        "version": "0.1.0"
    })))
}