use axum::extract::State;
use axum::http::StatusCode;
use axum::{response::IntoResponse, Form};
use chrono::Utc;
use serde::Deserialize;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

pub async fn subscribe(
    State(pool): State<Pool<Postgres>>,
    Form(form): Form<FormData>,
) -> impl IntoResponse {
    sqlx::query!(
        r#"
            INSERT INTO subscriptions (id, email, name, subscribed_at)
            VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(&pool)
    .await
    .map(|_| StatusCode::OK)
    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}
