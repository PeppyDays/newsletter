use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};

use newsletter::startup::run;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        .expect("Failed to bind a port for application");

    run(listener).await
}
