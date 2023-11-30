use std::{net::TcpListener, time::Duration};

use axum::{
    body::BoxBody,
    extract::{FromRef, MatchedPath},
    http::Request,
    response::Response,
    routing::{get, post},
    Router, Server,
};
use secrecy::ExposeSecret;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use tower_http::{classify::ServerErrorsFailureClass, trace::TraceLayer};
use tracing::Span;
use uuid::Uuid;

use crate::{
    configuration::Settings,
    email_client::EmailClient,
    routes::{check_health, confirm, subscribe},
};

#[derive(Clone)]
pub struct AccessUrl(pub String);

#[derive(Clone)]
pub struct AppState {
    pub pool: Pool<Postgres>,
    pub email_client: EmailClient,
    pub access_url: AccessUrl,
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

impl FromRef<AppState> for AccessUrl {
    fn from_ref(state: &AppState) -> Self {
        state.access_url.clone()
    }
}

pub async fn run(listener: TcpListener, app_state: AppState) {
    let app = Router::new()
        .route("/subscriptions/confirm", get(confirm))
        .route("/subscriptions", post(subscribe))
        .with_state(app_state)
        .route("/health_check", get(check_health))
        .layer(
            // Refer to https://github.com/tokio-rs/axum/blob/main/examples/tracing-aka-logging/Cargo.toml
            TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
                let path = request
                    .extensions()
                    .get::<MatchedPath>()
                    .map(MatchedPath::as_str);

                tracing::info_span!(
                    "Processing HTTP request",
                    method = ?request.method(),
                    path,
                    request_id = %Uuid::new_v4(),
                )
            }),
        );

    Server::from_tcp(listener)
        .expect("Failed to start up the application")
        .serve(app.into_make_service())
        .await
        .expect("Failed to start up the application");
}

pub async fn get_listener(configuration: &Settings) -> TcpListener {
    TcpListener::bind(format!(
        "{}:{}",
        configuration.application.host, configuration.application.port,
    ))
    .expect("Failed to bind a port for application")
}

pub async fn get_app_state(configuration: &Settings) -> AppState {
    AppState {
        pool: db_connection_pool(configuration).await,
        email_client: email_client(configuration).await,
        access_url: AccessUrl(configuration.application.access_url.clone()),
    }
}

async fn db_connection_pool(configuration: &Settings) -> Pool<Postgres> {
    PgPoolOptions::new()
        .min_connections(5)
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(configuration.database.connection_string().expose_secret())
        .await
        .expect("Failed to create database connection pool")
}

async fn email_client(configuration: &Settings) -> EmailClient {
    let sender_email = configuration
        .email_client
        .sender()
        .expect("Invalid sender email address");
    let timeout = configuration.email_client.timeout();

    EmailClient::new(
        configuration.email_client.access_url.to_owned(),
        sender_email,
        configuration.email_client.authorization_token.to_owned(),
        timeout,
    )
}
