use std::net::TcpListener;

use axum::{
    routing::{get, post},
    Router, Server,
};

use crate::routes::{check_health, subscribe};

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
