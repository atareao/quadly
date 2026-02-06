use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub hashed_password: String,
    pub role: String, // "admin" o "viewer"
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct NewUser {
    pub username: String,
    pub hashed_password: String,
    pub role: String, // "admin" o "viewer"
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct UserPass {
    pub username: String,
    pub hashed_password: String,
}

impl User {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }

    pub async fn read_by_username(
        pool: &sqlx::SqlitePool,
        username: &str,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(pool)
            .await
    }

    pub async fn read_all(pool: &sqlx::SqlitePool) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM users")
            .fetch_all(pool)
            .await
    }

    pub async fn create(pool: &sqlx::SqlitePool, new_user: NewUser) -> Result<Self, sqlx::Error>{
        let sql = "INSERT INTO users (username, hashed_password, role) VALUES (?, ?, ?) RETURNING *";
        sqlx::query_as::<_, Self>(sql)
            .bind(&new_user.username)
            .bind(&new_user.hashed_password)
            .bind(&new_user.role)
            .fetch_one(pool)
            .await
    }
}
