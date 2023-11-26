use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener};

use reqwest::{Client, Response};

use newsletter::{
    configuration::{get_configuration, Settings},
    startup::{get_app_state, run},
};
use secrecy::ExposeSecret;
use serde::Serialize;
use sqlx::{Connection, Executor, PgConnection, Pool, Postgres};
use uuid::Uuid;

pub struct App {
    address: SocketAddr,
    client: Client,
    pub pool: Pool<Postgres>,
}

impl App {
    pub async fn new() -> Self {
        // configure listener
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
            .expect("Failed to start an test application");
        let address = listener.local_addr().unwrap();

        // get configuration and randomise database name
        let mut configuration = get_configuration().expect("Failed to read configuration");
        configuration.database.database = Uuid::new_v4().to_string();

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
