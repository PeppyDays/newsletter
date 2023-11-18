use axum::http::StatusCode;
use axum::{response::IntoResponse, Form};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

pub async fn subscribe(Form(data): Form<FormData>) -> impl IntoResponse {
    println!("{:?}", data);
    StatusCode::OK
}
