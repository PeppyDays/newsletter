use axum::{http::StatusCode, response::IntoResponse};

pub async fn home() -> impl IntoResponse {
    StatusCode::OK
}
