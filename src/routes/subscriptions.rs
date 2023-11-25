use axum::extract::State;
use axum::http::StatusCode;
use axum::{response::IntoResponse, Form};
use chrono::Utc;
use serde::Deserialize;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::domain::{NewSubscriber, SubscriberName};

#[derive(Debug, Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    State(pool): State<Pool<Postgres>>,
    Form(form): Form<FormData>,
) -> impl IntoResponse {
    let name = match SubscriberName::parse(form.name) {
        Ok(n) => n,
        Err(_) => return StatusCode::BAD_REQUEST,
    };

    let new_subscriber = NewSubscriber {
        email: form.email,
        name,
    };

    match insert_subscriber(&pool, &new_subscriber).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(pool, new_subscriber)
)]
async fn insert_subscriber(
    pool: &Pool<Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
            INSERT INTO subscriptions (id, email, name, subscribed_at)
            VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        &new_subscriber.email,
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(pool)
    .await
    .map_err(|error| {
        tracing::error!("Failed to execute query: {:?}", error);
        error
    })?;

    Ok(())
}
