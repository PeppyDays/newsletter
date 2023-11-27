use reqwest::StatusCode;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::App;

#[tokio::test]
async fn confirmations_without_toekn_are_rejected_with_400() {
    let app = App::new().await;

    let response = app.get("/subscriptions/confirm").await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn link_returned_by_subscribe_returns_200_if_called() {
    let app = App::new().await;
    let parameter = [("name", "arine"), ("email", "peppydays@gmail.com")];

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(&parameter).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let links = app.get_confirmation_links(email_request);

    assert_eq!(links.in_html.host_str().unwrap(), "127.0.0.1");
    assert_eq!(links.in_text.host_str().unwrap(), "127.0.0.1");

    let response = reqwest::get(links.in_html).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn clicking_on_confirmation_link_confirms_a_subscriber() {
    let app = App::new().await;
    let parameter = [("name", "arine"), ("email", "peppydays@gmail.com")];

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(&parameter).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let links = app.get_confirmation_links(email_request);

    reqwest::get(links.in_html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&app.pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.email, "peppydays@gmail.com");
    assert_eq!(saved.name, "arine");
    assert_eq!(saved.status, "confirmed");
}
