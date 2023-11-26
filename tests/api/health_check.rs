use crate::helpers::App;

#[tokio::test]
async fn health_check_works() {
    let app = App::new().await;

    let response = app.get_health_check().await;

    assert!(response.status().is_success());
    assert_eq!(response.content_length(), Some(0));
}
