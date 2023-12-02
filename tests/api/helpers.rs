use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use once_cell::sync::Lazy;
use reqwest::{Client, Response};
use secrecy::ExposeSecret;
use serde::Serialize;
use sha3::Digest;
use sqlx::{Connection, Executor, PgConnection, Pool, Postgres};
use tokio::net::TcpListener;
use uuid::Uuid;
use wiremock::MockServer;

use newsletter::{
    configuration::{get_configuration, Settings},
    startup::{get_app_state, run},
    telemetry::{get_subscriber, initialize_subscriber},
};

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        initialize_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        initialize_subscriber(subscriber);
    };
});

pub struct App {
    pub address: SocketAddr,
    pub client: Client,
    pub pool: Pool<Postgres>,
    pub email_server: MockServer,
}

impl App {
    pub async fn new() -> Self {
        Lazy::force(&TRACING);

        // configure listener
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("Failed to start an test application");
        let address = listener.local_addr().unwrap();

        // run email server
        let email_server = MockServer::start().await;

        // get configuration and randomise database name
        let mut configuration = get_configuration().expect("Failed to read configuration");
        configuration.application.access_url = format!("http://{}", address);
        configuration.database.database = Uuid::new_v4().to_string();
        configuration.email_client.access_url = email_server.uri();

        // initialise randomise database
        App::initialise_database(&configuration).await;

        // configure app state
        let app_state = get_app_state(&configuration).await;

        // get database pool
        let pool = app_state.pool.clone();

        // migrate database
        sqlx::migrate!("./migrations")
            .run(&pool.clone())
            .await
            .expect("Failed to migrate the database");

        // start a server
        tokio::spawn(run(listener, app_state));

        // provide a reqwest client
        let client = Client::new();

        App {
            address,
            client,
            pool,
            email_server,
        }
    }

    async fn initialise_database(configuration: &Settings) {
        // create a connection to postgres database
        // and create randomised database
        let mut connection = PgConnection::connect(
            configuration
                .database
                .connection_string_without_database()
                .expose_secret(),
        )
        .await
        .expect("Failed to connect to Postgres");

        connection
            .execute(format!(r#"CREATE DATABASE "{}";"#, configuration.database.database).as_str())
            .await
            .expect("Failed to create database.");
    }
}

impl App {
    pub async fn get(&self, path: &str) -> Response {
        self.client
            .get(format!("http://{}{}", self.address, path))
            .send()
            .await
            .expect("Failed to send GET request")
    }

    // pub async fn post(&self, path: &str, body: &serde_json::Value) -> Response {
    //     self.client
    //         .post(format!("http://{}{}", self.address, path))
    //         .json(body)
    //         .send()
    //         .await
    //         .expect("Failed to send POST request")
    // }

    pub async fn form<T: Serialize + ?Sized>(&self, path: &str, parameter: &T) -> Response {
        self.client
            .post(format!("http://{}{}", self.address, path))
            .form(parameter)
            .send()
            .await
            .expect("Failed to send POST form request")
    }
}

impl App {
    pub async fn get_health_check(&self) -> Response {
        self.get("/health_check").await
    }

    pub async fn post_subscriptions<T: Serialize + ?Sized>(&self, parameter: &T) -> Response {
        self.form("/subscriptions", parameter).await
    }

    pub async fn post_newsletters(&self, body: &serde_json::Value) -> Response {
        let (username, password) = self.add_test_user().await;

        self.client
            .post(&format!("http://{}{}", self.address, "/newsletters"))
            .json(body)
            .basic_auth(username, Some(password))
            .send()
            .await
            .expect("Failed to execute request")
    }
}

pub struct ConfirmationLinks {
    pub in_html: reqwest::Url,
    pub in_text: reqwest::Url,
}

impl App {
    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            links[0].as_str().to_owned()
        };

        let link_in_html = &get_link(body["HtmlBody"].as_str().unwrap());
        let link_in_text = &get_link(body["TextBody"].as_str().unwrap());

        ConfirmationLinks {
            in_html: reqwest::Url::parse(link_in_html).unwrap(),
            in_text: reqwest::Url::parse(link_in_text).unwrap(),
        }
    }

    pub async fn add_test_user(&self) -> (String, String) {
        let username = Uuid::new_v4().to_string();
        let password = Uuid::new_v4().to_string();
        let password_hash = format!("{:x}", sha3::Sha3_256::digest(password.as_bytes()));

        sqlx::query!(
            "INSERT INTO users (user_id, username, password_hash) VALUES ($1, $2, $3)",
            Uuid::new_v4(),
            username,
            password_hash,
        )
        .execute(&self.pool)
        .await
        .expect("Failed to create test user");

        (username, password)
    }
}
