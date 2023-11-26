use newsletter::{
    configuration::get_configuration,
    startup::{get_app_state, get_listener, run},
    telemetry::{get_subscriber, initialize_subscriber},
};

#[tokio::main]
async fn main() {
    let subscriber = get_subscriber("newsletter".into(), "info".into());
    initialize_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration");

    let listener = get_listener(&configuration).await;
    let app_state = get_app_state(&configuration).await;

    run(listener, app_state).await
}
