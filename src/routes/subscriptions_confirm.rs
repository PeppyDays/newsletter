use axum::extract::State;
use axum::response::IntoResponse;
use axum::{extract::Query, http::StatusCode};
use serde::Deserialize;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

#[derive(Deserialize, Debug)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(pool))]
pub async fn confirm(
    State(pool): State<Pool<Postgres>>,
    Query(parameters): Query<Parameters>,
) -> impl IntoResponse {
    let id = match get_subscriber_id_from_token(&pool, &parameters.subscription_token).await {
        Ok(id) => id,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };

    match id {
        None => StatusCode::UNAUTHORIZED,
        Some(subscriber_id) => match confirm_subscriber(&pool, subscriber_id).await {
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Ok(_) => StatusCode::OK,
        },
    }
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(pool, subscription_id))]
pub async fn confirm_subscriber(
    pool: &Pool<Postgres>,
    subscription_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE subscriptions SET status = 'confirmed' WHERE id = $1",
        subscription_id
    )
    .execute(pool)
    .await
    .map_err(|error| {
        tracing::error!("Failed to execute query: {:?}", error);
        error
    })?;

    Ok(())
}

#[tracing::instrument(name = "Get subscriber_id from token", skip(pool, subscription_token))]
pub async fn get_subscriber_id_from_token(
    pool: &Pool<Postgres>,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        "SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1",
        subscription_token,
    )
    .fetch_optional(pool)
    .await
    .map_err(|error| {
        tracing::error!("Failed to execute query: {:?}", error);
        error
    })?;

    Ok(result.map(|r| r.subscriber_id))
}
