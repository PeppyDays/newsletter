use std::net::TcpListener;

use axum::{
    extract::{FromRef, MatchedPath},
    http::Request,
    routing::{get, post},
    Router, Server,
};
use sqlx::{Pool, Postgres};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::{
    email_client::EmailClient,
    routes::{check_health, subscribe},
};

pub async fn run(listener: TcpListener, pool: Pool<Postgres>, email_client: EmailClient) {
    let app = router(pool, email_client);

    Server::from_tcp(listener)
        .expect("Failed to start up the application")
        .serve(app.into_make_service())
        .await
        .expect("Failed to start up the application");
}

#[derive(Clone)]
struct AppState {
    pool: Pool<Postgres>,
    email_client: EmailClient,
}

impl FromRef<AppState> for Pool<Postgres> {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

impl FromRef<AppState> for EmailClient {
    fn from_ref(state: &AppState) -> Self {
        state.email_client.clone()
    }
}

pub fn router(pool: Pool<Postgres>, email_client: EmailClient) -> Router {
    let state = AppState { pool, email_client };

    Router::new()
        .route("/subscriptions", post(subscribe))
        .with_state(state)
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
