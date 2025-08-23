use std::env;

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub upload_path: String,
    pub port: u16,
    pub cors_allowed_origins: Vec<String>,
}

impl Config {
    pub fn from_env() -> Self {
        dotenv::dotenv().ok();

        let cors_origins = env::var("CORS_ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:3000,https://am.plify.app".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        Config {
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:./amplify.db".to_string()),
            redis_url: env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            jwt_secret: env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret-key".to_string()),
            upload_path: env::var("UPLOAD_PATH").unwrap_or_else(|_| "./uploads".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .expect("PORT must be a valid number"),
            cors_allowed_origins: cors_origins,
        }
    }
}