use reqwest::StatusCode;

use crate::helpers::App;

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
