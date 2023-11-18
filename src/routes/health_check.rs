use axum::http::StatusCode;
use axum::response::IntoResponse;

pub async fn check_health() -> impl IntoResponse {
    StatusCode::OK
}
