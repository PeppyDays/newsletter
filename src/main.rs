use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};

use newsletter::{configuration::get_configuration, startup::run};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() {
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
