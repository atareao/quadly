use axum::{
    body::Body,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum CustomResponse<T> {
    Api(ApiResponse<T>),
    Empty(EmptyResponse),
}


impl<T> CustomResponse<T>
where
    T: Serialize,
{
    pub fn empty(status: StatusCode, message: &str) -> Self {
        CustomResponse::Empty(EmptyResponse {
            status,
            message: message.to_string(),
        })
    }
    pub fn api(status: StatusCode, message: &str, data: T) -> Self {
        CustomResponse::Api(ApiResponse::new(status, message, data))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ApiResponse<T> {
    pub status: u16,
    pub message: String,
    pub data: T,
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    pub fn new(status: StatusCode, message: &str, data: T) -> Self {
        Self {
            status: status.as_u16(),
            message: message.to_string(),
            data,
        }
    }
}

impl<T> From<ApiResponse<T>> for CustomResponse<T>
where
    T: Serialize,
{
    fn from(api_response: ApiResponse<T>) -> Self {
        CustomResponse::Api(api_response)
    }
}

impl<T> From<EmptyResponse> for CustomResponse<T>
where
    T: Serialize,
{
    fn from(empty_response: EmptyResponse) -> Self {
        CustomResponse::Empty(empty_response)
    }
}

impl<T> IntoResponse for ApiResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(self)).into_response()
    }
}

impl<T> IntoResponse for CustomResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        match self {
            CustomResponse::Api(api_response) => api_response.into_response(),
            CustomResponse::Empty(empty_response) => empty_response.into_response(),
        }
    }
}

#[derive(Debug, Clone)]
struct EmptyResponse {
    pub status: StatusCode,
    pub message: String,
}
impl EmptyResponse {
    pub fn create(status: StatusCode, message: &str) -> Response<Body> {
        Response::builder()
            .status(status)
            .body(Body::from(message.to_string())) // Cuerpo de la respuesta
            .unwrap()
    }
}

impl IntoResponse for EmptyResponse {
    fn into_response(self) -> Response {
        EmptyResponse::create(self.status, self.message.as_str())
    }
}
