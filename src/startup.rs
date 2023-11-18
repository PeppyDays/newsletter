use std::net::TcpListener;

use axum::{
    routing::{get, post},
    Router, Server,
};
use sqlx::{Pool, Postgres};

use crate::routes::{check_health, subscribe};

pub async fn run(listener: TcpListener, pool: Pool<Postgres>) {
    let app = router(pool);

    Server::from_tcp(listener)
        .expect("Failed to start up the application")
        .serve(app.into_make_service())
        .await
        .expect("Failed to start up the application");
}

pub fn router(pool: Pool<Postgres>) -> Router {
    Router::new()
        .route("/subscriptions", post(subscribe))
        .with_state(pool)
        .route("/health_check", get(check_health))
}
