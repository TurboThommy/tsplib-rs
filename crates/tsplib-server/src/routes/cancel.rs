use axum::{Router, extract::State, http::StatusCode, routing::post};

use crate::state::{AppState, ProcessingState};

/// Router for cancellation-related endpoints.
pub fn router() -> Router<AppState> {
    Router::new().route("/cancel", post(cancel_processing))
}

/// Cancels the currently running processing task, if any.
/// If a solver or parser is running, it aborts the task and resets the AppState to idle.
/// If no processingt task is running, it returns a bad request status code.
///
/// # Arguments
/// * `State(state)` - The shared application state containing the AppState.
///
/// # Returns
/// * `StatusCode` - HTTP status code indicating the result of the cancellation attempt.
async fn cancel_processing(State(state): State<AppState>) -> StatusCode {
    let solver_state = state.solver_state.lock().await;

    if let ProcessingState::Processing(ct) = &*solver_state {
        ct.cancel();
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    }
}
