use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener};

use reqwest::{Client, Response, StatusCode};
use secrecy::ExposeSecret;
use serde::Serialize;
use sqlx::{postgres::PgPoolOptions, Connection, Executor, PgConnection, Pool, Postgres};
use uuid::Uuid;

use newsletter::{configuration::get_configuration, startup::run};

#[tokio::test]
async fn health_check_works() {
    let app = App::new().await;

    let response = app.get("/health_check").await;

    assert!(response.status().is_success());
    assert_eq!(response.content_length(), Some(0));
}

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    let app = App::new().await;
    let parameter = [("name", "arine"), ("email", "peppydays@gmail.com")];

    let response = app.form("/subscriptions", &parameter).await;

    assert_eq!(response.status(), StatusCode::OK);

    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&app.pool)
        .await
        .unwrap();

    assert_eq!(saved.email, "peppydays@gmail.com");
    assert_eq!(saved.name, "arine");
}

#[tokio::test]
async fn subscribe_returns_422_when_some_attributes_in_request_are_missing() {
    let app = App::new().await;
    let test_cases = vec![[("name", "arine")], [("email", "peppydays@gmail.com")]];

    for test_case in test_cases {
        let response = app.form("/subscriptions", &test_case).await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}

#[tokio::test]
async fn subscribe_returns_400_when_fields_are_present_but_empty() {
    let app = App::new().await;
    let test_cases = [
        [("name", "arine"), ("email", "")],
        [("name", ""), ("email", "pepppydays@gmail.com")],
        [("name", "arine"), ("email", "definitely-not-an-email")],
    ];

    for test_case in test_cases {
        let response = app.form("/subscriptions", &test_case).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}

struct App {
    address: SocketAddr,
    client: Client,
    pool: Pool<Postgres>,
}

impl App {
    async fn new() -> Self {
        // get configuration and randomise database name
        let mut configuration = get_configuration().expect("Failed to read configuration");
        configuration.database.database = Uuid::new_v4().to_string();

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

        // create a database connection pool pointing the new randomised database
        let pool = PgPoolOptions::new()
            .min_connections(5)
            .max_connections(5)
            .connect(configuration.database.connection_string().expose_secret())
            .await
            .expect("Failed to create database connection pool");

        // migrate database
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to migrate the database");

        // start an application
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
            .expect("Failed to start an app in test");
        let address = listener.local_addr().unwrap();

        tokio::spawn(run(listener, pool.clone()));

        // provide a reqwest client
        let client = Client::new();

        App {
            address,
            client,
            pool,
        }
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
