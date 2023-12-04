use axum::{
    extract::State,
    http::{header::LOCATION, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    Form,
};
use secrecy::Secret;
use serde::Deserialize;
use sqlx::{Pool, Postgres};

use crate::authentication::{validate_credentials, AuthError, Credentials};

#[derive(Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(skip(form, pool), fields(username=tracing::field::Empty, user_id=tracing::field::Empty))]
pub async fn login(
    State(pool): State<Pool<Postgres>>,
    Form(form): Form<FormData>,
) -> Result<impl IntoResponse, LoginError> {
    let credentials = Credentials {
        username: form.username,
        password: form.password,
    };
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));

    let user_id = validate_credentials(credentials, &pool)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
            AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
        })?;
    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    let mut headers = HeaderMap::new();
    headers.append(LOCATION, HeaderValue::from_static("/"));

    Ok(headers)
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
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

impl IntoResponse for LoginError {
    fn into_response(self) -> axum::response::Response {
        match self {
            LoginError::AuthError(_) => StatusCode::UNAUTHORIZED,
            LoginError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
        .into_response()
    }
}
