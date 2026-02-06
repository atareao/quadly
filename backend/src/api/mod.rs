mod auth;
mod quadlet;
mod health;

pub use quadlet::router as quadlet_router;
pub use health::router as health_router;
pub use auth::router as auth_router;
