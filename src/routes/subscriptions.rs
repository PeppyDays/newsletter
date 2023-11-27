use axum::extract::State;
use axum::http::StatusCode;
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
    type Error = String;

    fn try_from(data: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(data.name)?;
        let email = SubscriberEmail::parse(data.email)?;

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
) -> impl IntoResponse {
    let new_subscriber = match form.try_into() {
        Ok(subscriber) => subscriber,
        Err(_) => return StatusCode::BAD_REQUEST,
    };

    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };

    if let Err(_error) = insert_subscriber(&mut transaction, &new_subscriber).await {
        return StatusCode::INTERNAL_SERVER_ERROR;
    };

    let subscription_token = generate_subscription_token();

    if let Err(_error) = store_token(&mut transaction, &new_subscriber, &subscription_token).await {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    if let Err(_error) = transaction.commit().await {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    if let Err(_error) = send_confirmation_email(
        &email_client,
        &access_url,
        &new_subscriber,
        &subscription_token,
    )
    .await
    {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    StatusCode::OK
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

    transaction.execute(query).await.map_err(|error| {
        tracing::error!("Failed to execute query: {:?}", error);
        error
    })?;

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

    transaction.execute(query).await.map_err(|error| {
        tracing::error!("Failed to execute query: {:?}", error);
        error
    })?;

    Ok(())
}
