use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use ts_rs::TS;

#[derive(Debug, FromRow, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/Config.ts")]
pub struct Config {
    pub id: i32,
    pub key: String,
    pub value: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/NewConfig.ts")]
pub struct NewConfig {
    pub key: String,
    pub value: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/UpdateConfig.ts")]
pub struct UpdateConfig {
    pub value: String,
    pub description: Option<String>,
}

impl Config {
    /// Obtiene un valor de configuración por su clave
    pub async fn get_by_key(pool: &sqlx::SqlitePool, key: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM config WHERE key = ?")
            .bind(key)
            .fetch_optional(pool)
            .await
    }

    /// Obtiene todas las configuraciones
    pub async fn get_all(pool: &sqlx::SqlitePool) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM config ORDER BY key")
            .fetch_all(pool)
            .await
    }

    /// Crea una nueva configuración
    pub async fn create(pool: &sqlx::SqlitePool, new_config: NewConfig) -> sqlx::Result<Self> {
        let sql = "INSERT INTO config (key, value, description) VALUES (?, ?, ?) RETURNING *";
        sqlx::query_as::<_, Self>(sql)
            .bind(&new_config.key)
            .bind(&new_config.value)
            .bind(new_config.description)
            .fetch_one(pool)
            .await
    }

    /// Actualiza una configuración existente por su clave
    pub async fn update_by_key(
        pool: &sqlx::SqlitePool,
        key: &str,
        update_config: UpdateConfig,
    ) -> sqlx::Result<Option<Self>> {
        let sql = "UPDATE config SET value = ?, description = ?, updated_at = CURRENT_TIMESTAMP 
                   WHERE key = ? RETURNING *";
        sqlx::query_as::<_, Self>(sql)
            .bind(&update_config.value)
            .bind(update_config.description)
            .bind(key)
            .fetch_optional(pool)
            .await
    }

    /// Elimina una configuración por su clave
    pub async fn delete_by_key(pool: &sqlx::SqlitePool, key: &str) -> sqlx::Result<u64> {
        let result = sqlx::query("DELETE FROM config WHERE key = ?")
            .bind(key)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Obtiene la URL de templates de quadlets
    pub async fn get_quadly_templates_url(pool: &sqlx::SqlitePool) -> sqlx::Result<Option<String>> {
        if let Some(config) = Self::get_by_key(pool, "url_quadly_templates").await? {
            Ok(Some(config.value))
        } else {
            Ok(None)
        }
    }

    /// Establece/actualiza la URL de templates de quadlets
    pub async fn set_quadly_templates_url(
        pool: &sqlx::SqlitePool,
        url: &str,
    ) -> sqlx::Result<Self> {
        let update_config = UpdateConfig {
            value: url.to_string(),
            description: Some("URL del repositorio de templates para quadlets".to_string()),
        };

        if let Some(existing) =
            Self::update_by_key(pool, "url_quadly_templates", update_config).await?
        {
            Ok(existing)
        } else {
            // Si no existe, créala
            let new_config = NewConfig {
                key: "url_quadly_templates".to_string(),
                value: url.to_string(),
                description: Some("URL del repositorio de templates para quadlets".to_string()),
            };
            Self::create(pool, new_config).await
        }
    }
}
