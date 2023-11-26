use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener};

use reqwest::{Client, Response};
use secrecy::ExposeSecret;
use serde::Serialize;
use sqlx::{postgres::PgPoolOptions, Connection, Executor, PgConnection, Pool, Postgres};

use newsletter::{configuration::get_configuration, email_client::EmailClient, startup::run};
use uuid::Uuid;

pub struct App {
    address: SocketAddr,
    client: Client,
    pub pool: Pool<Postgres>,
}

impl App {
    pub async fn new() -> Self {
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

        // create a email client
        let sender_email = configuration.email_client.sender().unwrap();
        let timeout = configuration.email_client.timeout();
        let email_client = EmailClient::new(
            configuration.email_client.base_url,
            sender_email,
            configuration.email_client.authorization_token,
            timeout,
        );

        // migrate database
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to migrate the database");

        // start an application
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
            .expect("Failed to start an app in test");
        let address = listener.local_addr().unwrap();

        tokio::spawn(run(listener, pool.clone(), email_client.clone()));

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
    pub async fn get(&self, path: &str) -> Response {
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

    pub async fn form<T: Serialize + ?Sized>(&self, path: &str, parameter: &T) -> Response {
        self.client
            .post(format!("http://{}{}", self.address, path))
            .form(parameter)
            .send()
            .await
            .expect("Failed to send POST form request")
    }
}
