use std::{net::TcpListener, time::Duration};

use secrecy::ExposeSecret;
use sqlx::postgres::PgPoolOptions;

use newsletter::{
    configuration::get_configuration,
    email_client::EmailClient,
    startup::run,
    telemetry::{get_subscriber, initialize_subscriber},
};

#[tokio::main]
async fn main() {
    let subscriber = get_subscriber("newsletter".into(), "info".into());
    initialize_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration");

    let listener = TcpListener::bind(format!(
        "{}:{}",
        configuration.application.host, configuration.application.port,
    ))
    .expect("Failed to bind a port for application");

    let pool = PgPoolOptions::new()
        .min_connections(5)
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(configuration.database.connection_string().expose_secret())
        .await
        .expect("Failed to create database connection pool");

    let sender_email = configuration
        .email_client
        .sender()
        .expect("Invalid sender email address");
    let email_client = EmailClient::new(
        configuration.email_client.base_url,
        sender_email,
        configuration.email_client.authorization_token,
    );

    run(listener, pool, email_client).await;
}
