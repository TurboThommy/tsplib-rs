//! Module containing the specific error types that can occur during the operation of the TSPLIB server.
use axum::{http::StatusCode, response::IntoResponse};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("I/O error: {0}")]
    IoError(String),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            ServerError::IoError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };
        (status, error_message).into_response()
    }
}

impl From<std::io::Error> for ServerError {
    fn from(value: std::io::Error) -> Self {
        ServerError::IoError(value.to_string())
    }
}
