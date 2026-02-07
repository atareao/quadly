use axum::{
    routing::{get, post},
    Router,
};
use sqlx::{
    migrate::{
        Migrator,
        MigrateDatabase,
    },
    sqlite::SqlitePool
};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use std::{env::var, str::FromStr, sync::Arc, path::Path};
use tracing::{debug, error};
use crate::models::AppState;

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
    let secret = std::env::var("SECRET").expect("SECRET environment variable must be set");
    info!("DB url: {}", db_url);
    let port: u16 = var("PORT")
        .unwrap_or("3000".to_string())
        .parse()
        .unwrap_or(3000);
    info!("Port: {}", port);


    if !sqlx::Sqlite::database_exists(&db_url)
            .await
            .map_err(|e| {
                error!("Failed to check if database exists: {}", e);
                e
            })? {
        info!("Database does not exist, creating...");
        sqlx::Sqlite::create_database(&db_url)
        .await
        .map_err(|e| {
            error!("Failed to create database: {}", e);
            e
        })?;
    } else {
        info!("Database already exists");
    }

    let migrations = if var("RUST_ENV") == Ok("production".to_string()){
        std::env::current_exe().unwrap().parent().unwrap().join("migrations")
    }else{
        let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        Path::new(&crate_dir).join("migrations")
    };
    info!("{}", &migrations.display());

    let pool = SqlitePool::connect(&db_url).await.expect("Failed to connect to database");

    Migrator::new(migrations)
        .await?
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    // Configuración de CORS para permitir al frontend de React comunicarse
    let cors = CorsLayer::permissive(); // En producción deberías restringirlo

    let routes = Router::new()
        .nest("/health",api::health_router())
        .nest("/quadlets",api::quadlet_router())
        .nest("/auth",api::auth_router())
        .fallback(api::fallback_404)
        .with_state(Arc::new(AppState {
            pool,
            secret,
            static_dir: "static".to_string(),
        }));

    // Definición de las rutas de Quadly
    let app = Router::new().nest("/api/v1", routes).layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("🚀 Quadly Backend arrancando en http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
