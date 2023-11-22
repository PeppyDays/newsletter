use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};

use sqlx::postgres::PgPoolOptions;

use newsletter::{
    configuration::get_configuration,
    startup::run,
    telemetry::{get_subscriber, initialize_subscriber},
};

#[tokio::main]
async fn main() {
    let subscriber = get_subscriber("newsletter".into(), "info".into());
    initialize_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration");

    let listener = TcpListener::bind(SocketAddrV4::new(
        Ipv4Addr::LOCALHOST,
        configuration.application.port,
    ))
    .expect("Failed to bind a port for application");

    let pool = PgPoolOptions::new()
        .min_connections(5)
        .max_connections(5)
        .connect(&configuration.database.connection_string())
        .await
        .expect("Failed to create database connection pool");

    run(listener, pool).await;
}
