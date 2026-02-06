use anyhow::Result;
use sqlx::SqlitePool;

pub async fn init_db(pool: &SqlitePool) -> Result<()> {
    // 1. Crear tabla si no existe
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            role TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    // 2. Crear usuario inicial si la tabla está vacía
    let count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;

    if count == 0 {
        let admin_user = std::env::var("QUADLY_ADMIN_USER").unwrap_or_else(|_| "admin".into());
        let admin_pass =
            std::env::var("QUADLY_ADMIN_PASS").expect("QUADLY_ADMIN_PASS es obligatoria");
        let hash = bcrypt::hash(admin_pass, bcrypt::DEFAULT_COST)?;

        sqlx::query("INSERT INTO users (username, password_hash, role) VALUES (?, ?, ?)")
            .bind(admin_user)
            .bind(hash)
            .bind("admin")
            .execute(pool)
            .await?;

        println!("👤 Usuario administrador inicial creado.");
    }
    Ok(())
}

// En el shutdown_signal de main.rs
async fn shutdown_signal(pool: SqlitePool) {
    // ... lógica de señales (Ctrl+C, SIGTERM) ...
    println!("Cerrando conexiones de base de datos...");
    pool.close().await;
}
