use std::fmt::Debug;

use anyhow::Context;
use axum::extract::State;
use axum::http::header::WWW_AUTHENTICATE;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use axum_extra::headers::authorization::Basic;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use secrecy::Secret;
use serde::Deserialize;
use sqlx::{Pool, Postgres};

use crate::authentication::validate_credentials;
use crate::authentication::{AuthError, Credentials};
use crate::{domain::SubscriberEmail, email_client::EmailClient};

#[derive(Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(Deserialize)]
pub struct Content {
    html: String,
}

impl From<Authorization<Basic>> for Credentials {
    fn from(auth: Authorization<Basic>) -> Self {
        let username = auth.username();
        let password = auth.password();

        Self {
            username: username.into(),
            password: Secret::new(password.into()),
        }
    }
}

#[tracing::instrument(
    name = "Sending newsletter to the subscribers",
    skip(pool, email_client, body, authorization)
)]
pub async fn publish_newsletter(
    State(pool): State<Pool<Postgres>>,
    State(email_client): State<EmailClient>,
    TypedHeader(authorization): TypedHeader<Authorization<Basic>>,
    Json(body): Json<BodyData>,
) -> Result<(), PublishError> {
    let credentials: Credentials = authorization.into();

    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));
    let user_id = validate_credentials(credentials, &pool)
        .await
        .map_err(|error| match error {
            AuthError::InvalidCredentials(_) => PublishError::AuthError(error.into()),
            AuthError::UnexpectedError(_) => PublishError::UnexpectedError(error.into()),
        })?;
    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    let confirmed_subscribers = get_confirmed_subscribers(&pool).await?;

    for subscriber in confirmed_subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &body.title,
                        &body.content.html,
                        &body.content.html,
                    )
                    .await
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })?;
            }
            Err(error) => {
                tracing::warn!(
                    error.cause_chain = ?error,
                    "Skipping a confirmed subscriber. There stored contact details are invalid",
                )
            }
        }
    }

    Ok(())
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &Pool<Postgres>,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let subscribers = sqlx::query!(
        r#"
            SELECT email FROM subscriptions WHERE status = 'confirmed'
        "#,
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| match SubscriberEmail::parse(r.email) {
        Ok(email) => Ok(ConfirmedSubscriber { email }),
        Err(error) => Err(anyhow::anyhow!(error)),
    })
    .collect();

    Ok(subscribers)
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for PublishError {
    fn into_response(self) -> axum::response::Response {
        match self {
            PublishError::AuthError(_) => {
                let mut headers = HeaderMap::new();
                headers.append(
                    WWW_AUTHENTICATE,
                    HeaderValue::from_static(r#"Basic realm="publish"#),
                );

                (StatusCode::UNAUTHORIZED, headers, Json(self.to_string()))
            }
            PublishError::UnexpectedError(_) => {
                tracing::error!("{:?}", self);

                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    HeaderMap::new(),
                    Json(self.to_string()),
                )
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
