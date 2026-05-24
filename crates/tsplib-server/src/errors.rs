//! Module containing the specific error types that can occur during the operation of the TSPLIB server.
use axum::{http::StatusCode, response::IntoResponse};
use thiserror::Error;
use tokio::task::JoinError;
use tsplib_core::enums::{ConversionError, IoError};
use tsplib_parser::ParseError;
use tsplib_solver::errors::SolverError;

/// This module defines the `ServerError` enum, which represents the various errors that can occur during the operation of the TSPLIB server.
/// Each variant of the enum corresponds to a specific type
#[derive(Error, Debug)]
pub enum ServerError {
    #[error("I/O error: {0}")]
    IoError(String),
    #[error("Solver error: {0}")]
    SolverError(String),
    #[error("A Processing task is already running")]
    ProcessingAlreadyRunning,
    #[error("Failed to parse problem instance: {0}")]
    ProblemParseError(String),
    #[error("Processing task was cancelled")]
    ProcessingCancelled,
    #[error("Failed to parse problem metadata for: {0}")]
    MetadataParseError(String),
    #[error("Unsupported edge weight type for problem: {0}")]
    UnsupportedEdgeWeightType(String),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            ServerError::ProcessingCancelled => (StatusCode::OK, self.to_string()),

            ServerError::ProcessingAlreadyRunning => (StatusCode::CONFLICT, self.to_string()),

            ServerError::SolverError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ServerError::IoError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ServerError::ProblemParseError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            ServerError::MetadataParseError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            ServerError::UnsupportedEdgeWeightType(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
        };
        (status, error_message).into_response()
    }
}

impl From<std::io::Error> for ServerError {
    fn from(value: std::io::Error) -> Self {
        ServerError::IoError(value.to_string())
    }
}

impl From<SolverError> for ServerError {
    fn from(value: SolverError) -> Self {
        ServerError::SolverError(value.to_string())
    }
}

impl From<ParseError> for ServerError {
    fn from(value: ParseError) -> Self {
        ServerError::ProblemParseError(value.to_string())
    }
}

impl From<ConversionError> for ServerError {
    fn from(value: ConversionError) -> Self {
        ServerError::ProblemParseError(value.to_string())
    }
}

impl From<JoinError> for ServerError {
    fn from(value: JoinError) -> Self {
        ServerError::SolverError(value.to_string())
    }
}

impl From<IoError> for ServerError {
    fn from(value: IoError) -> Self {
        ServerError::IoError(value.to_string())
    }
}
