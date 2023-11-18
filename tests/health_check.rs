use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener};

use newsletter::startup::run;
use reqwest::{Client, Response, StatusCode};
use serde::Serialize;

#[tokio::test]
async fn health_check_works() {
    let app = App::new();

    let response = app.get("/health_check").await;

    assert!(response.status().is_success());
    assert_eq!(response.content_length(), Some(0));
}

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    let app = App::new();
    let parameter = [("name", "arine"), ("email", "peppydays@gmail.com")];

    let response = app.form("/subscriptions", &parameter).await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn subscribe_returns_422_when_some_attributes_in_request_are_missing() {
    let app = App::new();
    let parameters = vec![[("name", "arine")], [("email", "peppydays@gmail.com")]];

    for parameter in parameters {
        let response = app.form("/subscriptions", &parameter).await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}

struct App {
    address: SocketAddr,
    client: Client,
}

impl App {
    fn new() -> Self {
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
            .expect("Failed to start an app in test");
        let address = listener.local_addr().unwrap();
        tokio::spawn(run(listener));

        let client = Client::new();

        App { address, client }
    }
}

impl App {
    async fn get(&self, path: &str) -> Response {
        self.client
            .get(format!("http://{}{}", self.address, path))
            .send()
            .await
            .expect("Failed to send GET request")
    }

    // async fn post(&self, path: &str, headers: HeaderMap, body: Body) -> Response {
    //     self.client
    //         .post(format!("http://{}{}", self.address, path))
    //         .headers(headers)
    //         .body(body)
    //         .send()
    //         .await
    //         .expect("Failed to send POST request")
    // }

    async fn form<T: Serialize + ?Sized>(&self, path: &str, parameter: &T) -> Response {
        self.client
            .post(format!("http://{}{}", self.address, path))
            .form(parameter)
            .send()
            .await
            .expect("Failed to send POST form request")
    }
}
