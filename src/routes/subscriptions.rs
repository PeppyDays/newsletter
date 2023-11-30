use std::fmt::Debug;

use anyhow::Context;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use axum::{response::IntoResponse, Form};
use chrono::Utc;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::Deserialize;
use sqlx::{Executor, Pool, Postgres, Transaction};
use uuid::Uuid;

use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};
use crate::email_client::EmailClient;
use crate::startup::AccessUrl;

#[derive(Debug, Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = SubscribeError;

    fn try_from(data: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(data.name).map_err(SubscribeError::ValidationError)?;
        let email = SubscriberEmail::parse(data.email).map_err(SubscribeError::ValidationError)?;

        Ok(Self {
            id: Uuid::new_v4(),
            name,
            email,
        })
    }
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool, email_client, access_url),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    ),
)]
pub async fn subscribe(
    State(AccessUrl(access_url)): State<AccessUrl>,
    State(pool): State<Pool<Postgres>>,
    State(email_client): State<EmailClient>,
    Form(form): Form<FormData>,
) -> Result<StatusCode, SubscribeError> {
    let new_subscriber = form.try_into()?;
    let subscription_token = generate_subscription_token();

    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a PostgreSQL connection from the pool")?;
    insert_subscriber(&mut transaction, &new_subscriber)
        .await
        .context("Failed to insert new subscriber in the database")?;
    store_token(&mut transaction, &new_subscriber, &subscription_token)
        .await
        .context("Failed to store the confirmation token for a new subscriber")?;
    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new subscriber")?;

    send_confirmation_email(
        &email_client,
        &access_url,
        &new_subscriber,
        &subscription_token,
    )
    .await
    .context("Failed to send a confirmation email")?;

    Ok(StatusCode::OK)
}

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, new_subscriber)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    access_url: &str,
    new_subscriber: &NewSubscriber,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        access_url, subscription_token,
    );

    email_client
        .send_email(
            &new_subscriber.email,
            "Welcome!",
            &format!("Welcome to our newsletter!<br />Click <a href=\"{}\">here</a> to confirm your subscription.", confirmation_link),
            &format!("Welcome to our newsletter!\nVisit {} to confirm your subscription.", confirmation_link),
        )
        .await
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(transaction, new_subscriber)
)]
async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    let query = sqlx::query!(
        r#"
            INSERT INTO subscriptions (id, email, name, subscribed_at, status)
            VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        new_subscriber.id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    );

    transaction.execute(query).await?;

    Ok(())
}

#[tracing::instrument(
    name = "Storing new subscriber token in the database",
    skip(transaction, new_subscriber)
)]
async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
    subscription_token: &str,
) -> Result<(), sqlx::Error> {
    let query = sqlx::query!(
        r#"
            INSERT INTO subscription_tokens (subscription_token, subscriber_id)
            VALUES ($1, $2)
        "#,
        subscription_token,
        new_subscriber.id,
    );

    transaction.execute(query).await?;

    Ok(())
}

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for SubscribeError {
    fn into_response(self) -> axum::response::Response {
        match self {
            SubscribeError::ValidationError(message) => (StatusCode::BAD_REQUEST, Json(message)),
            SubscribeError::UnexpectedError(_) => {
                tracing::error!("{:?}", self);
                (StatusCode::INTERNAL_SERVER_ERROR, Json(self.to_string()))
            }
        }
        .into_response()
    }
}

fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}
