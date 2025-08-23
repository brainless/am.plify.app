use crate::types::{HealthResponse, HelloResponse};
use actix_web::{web, HttpResponse, Result};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/health", web::get().to(health_check))
        .route("/hello", web::get().to(hello_world));
}

async fn health_check() -> Result<HttpResponse> {
    let response = HealthResponse {
        status: "ok".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    Ok(HttpResponse::Ok().json(response))
}

async fn hello_world() -> Result<HttpResponse> {
    let response = HelloResponse {
        message: "Hello from Amplify Backend!".to_string(),
        service: "amplify-api".to_string(),
        version: "0.1.0".to_string(),
    };
    Ok(HttpResponse::Ok().json(response))
}
