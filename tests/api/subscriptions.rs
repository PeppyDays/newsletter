use reqwest::StatusCode;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::App;

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    let app = App::new().await;
    let parameter = [("name", "arine"), ("email", "peppydays@gmail.com")];

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    let response = app.post_subscriptions(&parameter).await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    let app = App::new().await;
    let parameter = [("name", "arine"), ("email", "peppydays@gmail.com")];

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    let _response = app.post_subscriptions(&parameter).await;

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&app.pool)
        .await
        .unwrap();

    assert_eq!(saved.email, "peppydays@gmail.com");
    assert_eq!(saved.name, "arine");
    assert_eq!(saved.status, "pending_confirmation");
}

#[tokio::test]
async fn subscribe_returns_422_when_some_attributes_in_request_are_missing() {
    let app = App::new().await;
    let test_cases = vec![[("name", "arine")], [("email", "peppydays@gmail.com")]];

    for test_case in test_cases {
        let response = app.post_subscriptions(&test_case).await;

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
        let response = app.post_subscriptions(&test_case).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
async fn subscribe_sends_confirmation_email_for_valid_data() {
    let app = App::new().await;
    let parameter = [("name", "arine"), ("email", "peppydays@gmail.com")];

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let _response = app.post_subscriptions(&parameter).await;
}

#[tokio::test]
async fn subscribe_sends_confirmation_email_with_link() {
    let app = App::new().await;
    let parameter = [("name", "arine"), ("email", "peppydays@gmail.com")];

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    let _response = app.post_subscriptions(&parameter).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let links = app.get_confirmation_links(email_request);

    assert_eq!(links.in_html, links.in_text);
}

#[tokio::test]
async fn subscribe_fails_if_there_is_fatal_database_error() {
    let app = App::new().await;
    let parameter = [("name", "arine"), ("email", "peppydays@gmail.com")];

    sqlx::query!("ALTER TABLE subscription_tokens DROP COLUMN subscription_token")
        .execute(&app.pool)
        .await
        .unwrap();

    let response = app.post_subscriptions(&parameter).await;

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
