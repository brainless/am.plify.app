use actix_cors::Cors;
use actix_web::{web, App, HttpServer, middleware::Logger};
use amplify_backend::config::Config;
use amplify_backend::routes;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    
    let config = Config::from_env();
    let port = config.port;
    
    println!("Starting Amplify Backend on port {}", port);
    
    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin("http://localhost:3000")
            .allowed_origin("https://am.plify.app")
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
            .allowed_headers(vec!["Content-Type", "Authorization"])
            .max_age(3600);
        
        App::new()
            .app_data(web::Data::new(config.clone()))
            .wrap(cors)
            .wrap(Logger::default())
            .service(
                web::scope("/api")
                    .configure(routes::configure)
            )
    })
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await
}
