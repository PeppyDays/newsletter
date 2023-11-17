use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener};

use newsletter::run;

#[tokio::test]
async fn health_check_works() {
    let app = App::new();

    let response = app
        .get("/health_check")
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(response.content_length(), Some(0));
}

struct App {
    address: SocketAddr,
    client: reqwest::Client,
}

impl App {
    fn new() -> Self {
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
            .expect("Failed to start an app in test");
        let address = listener.local_addr().unwrap();
        tokio::spawn(run(listener));

        let client = reqwest::Client::new();

        App { address, client }
    }
}

impl App {
    async fn get(&self, path: &str) -> Result<reqwest::Response, reqwest::Error> {
        self.client
            .get(format!("http://{}{}", self.address, path))
            .send()
            .await
    }
}
