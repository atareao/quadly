use axum::{
    routing::{get, post},
    Router,
};
use sqlx::sqlite::SqlitePool;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use std::{env::var, str::FromStr};

mod api;
mod core;
mod models;
mod system;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Inicializar el trazado de logs (útil para depurar)
    let log_level = var("RUST_LOG").unwrap_or("info".to_string());
    tracing_subscriber::registry()
        .with(EnvFilter::from_str(&log_level).unwrap())
        .with(tracing_subscriber::fmt::layer())
        .init();
    info!("Log level: {log_level}");

    // Configurar base de datos SQLite
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_string());
    info!("DB url: {}", db_url);
    let port: u16 = var("PORT")
        .unwrap_or("3000".to_string())
        .parse()
        .unwrap_or(3000);
    info!("Port: {}", port);

    let pool = SqlitePool::connect(&db_url).await?;

    // Inicializar DB si es primera vez
    if let Err(e) = system::init_db(&pool).await {
        eprintln!("Error inicializando base de datos: {}", e);
    }

    // Configuración de CORS para permitir al frontend de React comunicarse
    let cors = CorsLayer::permissive(); // En producción deberías restringirlo

    let routes = Router::new()
        .route("/health", get(api::health_router))
        .route("/login", post(api::login))
        .route("/quadlets", get(api::list_quadlets))
        .route("/quadlets/{name}", get(api::get_quadlet))
        .route("/quadlets/{name}/action", post(api::run_action))
        .route("/quadlets/{name}/save", post(api::save_quadlet))
        .route("/quadlets/{name}/logs", post(api::get_quadlet_logs))
        .with_state(pool);

    // Definición de las rutas de Quadly
    let app = Router::new().nest("/api/v1", routes).layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("🚀 Quadly Backend arrancando en http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
