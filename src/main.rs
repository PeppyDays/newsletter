use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};

use newsletter::{configuration::get_configuration, startup::run};

#[tokio::main]
async fn main() {
    let configuration = get_configuration().expect("Failed to read configuration");

    let listener = TcpListener::bind(SocketAddrV4::new(
        Ipv4Addr::LOCALHOST,
        configuration.application.port,
    ))
    .expect("Failed to bind a port for application");

    run(listener).await;
}
