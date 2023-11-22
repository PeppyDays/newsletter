use std::net::TcpListener;

use axum::{
    extract::MatchedPath,
    http::Request,
    routing::{get, post},
    Router, Server,
};
use sqlx::{Pool, Postgres};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

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
        .layer(
            // Refer to https://github.com/tokio-rs/axum/blob/main/examples/tracing-aka-logging/Cargo.toml
            TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
                let path = request
                    .extensions()
                    .get::<MatchedPath>()
                    .map(MatchedPath::as_str);
                tracing::info_span!(
                    "Starting HTTP request",
                    method = ?request.method(),
                    path,
                    request_id = %Uuid::new_v4(),
                )
            }),
        )
}
