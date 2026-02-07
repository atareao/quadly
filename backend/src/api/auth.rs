use std::sync::Arc;

use axum::{
    body,
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing, Json, Router,
};
use bcrypt::verify;
use tracing::{debug, error};

use axum_extra::extract::cookie::{Cookie, SameSite};
use jsonwebtoken::{encode, EncodingKey, Header};

use crate::models::{AppState, CustomResponse, NewUser, TokenClaims, User, UserPass};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/login", routing::post(login))
        .route("/logout", routing::get(logout))
        .route("/register", routing::post(register))
}

pub fn api_user_router() -> Router<Arc<AppState>> {
    Router::new().route("/", routing::get(read))
}

pub async fn login(
    State(app_state): State<Arc<AppState>>,
    Json(user_pass): Json<UserPass>,
) -> impl IntoResponse {
    //) -> Result<Json<serde_json::Value>,(StatusCode, Json<serde_json::Value>)>{
    tracing::info!("init login");
    tracing::info!("User pass: {:?}", user_pass);
    let user = User::read_by_username(&app_state.pool, &user_pass.username)
        .await
        .map_err(|e| {
            let message = &format!("Error: {}", e);
            CustomResponse::<()>::empty(StatusCode::FORBIDDEN, message)
        })?
        .ok_or_else(|| {
            let message = "Invalid name or password";
            CustomResponse::empty(StatusCode::FORBIDDEN, message)
        })?;
    if !verify(&user_pass.hashed_password, &user.hashed_password).unwrap() {
        let message = "Invalid name or password";
        return Err(CustomResponse::empty(StatusCode::FORBIDDEN, message));
    }

    let now = chrono::Utc::now();
    let iat = now.timestamp() as usize;
    let exp = (now + chrono::Duration::minutes(60)).timestamp() as usize;
    let claims: TokenClaims = TokenClaims {
        sub: user.username.to_string(),
        role: user.role,
        exp,
        iat,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(app_state.secret.as_bytes()),
    )
    .map_err(|e| {
        let message = format!("Encoding JWT error: {}", e);
        CustomResponse::empty(StatusCode::INTERNAL_SERVER_ERROR, &message)
    })
    .map(|token| {
        let value = serde_json::json!({"token": token});
        CustomResponse::api(StatusCode::OK, "Ok", Some(value))
    })
}

pub async fn register(
    State(app_state): State<Arc<AppState>>,
    Json(user): Json<NewUser>,
) -> impl IntoResponse {
    debug!("User data: {:?}", user);
    match User::create(&app_state.pool, user).await {
        Ok(user) => {
            debug!("User created: {:?}", user);
            CustomResponse::api(
                StatusCode::CREATED,
                "User created",
                Some(serde_json::to_value(user).unwrap()),
            )
        }
        Err(e) => {
            error!("Error creating user: {:?}", e);
            CustomResponse::empty(
                StatusCode::BAD_REQUEST,
                &format!("Error creating user: {}", e),
            )
        }
    }
}

pub async fn logout() -> impl IntoResponse {
    debug!("Logout");
    let cookie = Cookie::build(("token", ""))
        .path("/")
        .max_age(cookie::time::Duration::ZERO)
        .same_site(SameSite::Lax)
        .http_only(true)
        .build();

    tracing::info!("The cookie: {}", cookie.to_string());

    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, "/")
        .header(header::SET_COOKIE, cookie.to_string())
        .body(body::Body::empty())
        .unwrap()
}

pub async fn read(State(app_state): State<Arc<AppState>>) -> impl IntoResponse {
    match User::read_all(&app_state.pool).await {
        Ok(values) => {
            debug!("Users: {:?}", values);
            CustomResponse::api(
                StatusCode::OK,
                "Users",
                Some(serde_json::to_value(values).unwrap()),
            )
        }
        Err(e) => {
            error!("Error reading values: {:?}", e);
            CustomResponse::empty(
                StatusCode::BAD_REQUEST,
                &format!("Error reading values: {}", e),
            )
        }
    }
}
