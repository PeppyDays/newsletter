use std::net::TcpListener;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Form, Router, Server};
use serde::Deserialize;

async fn check_health() -> impl IntoResponse {
    StatusCode::OK
}

#[derive(Debug, Deserialize)]
struct FormData {
    email: String,
    name: String,
}

async fn subscribe(Form(data): Form<FormData>) -> impl IntoResponse {
    println!("{:?}", data);
    StatusCode::OK
}

pub async fn run(listener: TcpListener) {
    let app = router();

    Server::from_tcp(listener)
        .expect("Failed to start up the application")
        .serve(app.into_make_service())
        .await
        .expect("Failed to start up the application");
}

pub fn router() -> Router {
    Router::new()
        .route("/health_check", get(check_health))
        .route("/subscriptions", post(subscribe))
}
