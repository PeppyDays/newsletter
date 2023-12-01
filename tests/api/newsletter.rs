use reqwest::StatusCode;
use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::{App, ConfirmationLinks};

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    let app = App::new().await;
    create_unconfirmed_subscriber(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Title",
        "content": {
            "text": "Text",
            "html": "Html",
        }
    });

    let response = app.post_newsletters(&newsletter_request_body).await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let app = App::new().await;
    create_confirmed_subscriber(&app).await;

    Mock::given(any())
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>"
        }
    });

    let response = app.post_newsletters(&newsletter_request_body).await;

    assert_eq!(response.status(), StatusCode::OK)
}

#[tokio::test]
async fn newsletters_returns_422_for_invalid_data() {
    let app = App::new().await;

    let invalid_cases = [
        serde_json::json!({
            "content": {
                "text": "Hi",
                "html": "There",
            }
        }),
        serde_json::json!({
            "title": "hi",
        }),
    ];

    for case in invalid_cases {
        let response = app.post_newsletters(&case).await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY)
    }
}

async fn create_unconfirmed_subscriber(app: &App) -> ConfirmationLinks {
    let parameter = [("name", "arine"), ("email", "peppydays@gmail.com")];

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    app.post_subscriptions(&parameter)
        .await
        .error_for_status()
        .unwrap();

    let email_request = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();

    app.get_confirmation_links(email_request)
}

async fn create_confirmed_subscriber(app: &App) {
    let confirmation_link = create_unconfirmed_subscriber(app).await;

    reqwest::get(confirmation_link.in_html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}
