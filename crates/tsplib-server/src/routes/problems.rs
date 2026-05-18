//! Handlers for the REST API routes related to problem instances.
use crate::{
    errors::ServerError,
    state::{AppState, ProcessingState},
};

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use std::fs;
use tokio_util::sync::CancellationToken;
use tsplib_core::{context::ExecutionContext, models::ProblemInstance};
use tsplib_parser::try_parse;

/// Router for problem-related endpoints.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/problems", get(get_problems))
        .route("/problems/{problemId}", get(get_problem))
}

/// Get the list of available TSP problem instances from the "./data" directory.
///
/// # Returns
/// * `Json<Vec<String>>` - A list of problem IDs (filenames without extension) in JSON format
///   or an error if the directory cannot be read.
async fn get_problems() -> Result<Json<Vec<String>>, ServerError> {
    let problems = fs::read_dir("./data")?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            if path.extension()? != "tsp" {
                return None;
            }
            Some(path.file_stem()?.to_string_lossy().to_string())
        })
        .collect::<Vec<_>>();

    Ok(Json(problems))
}

/// Get a specific TSP problem instance by its ID.
///
/// # Arguments
/// * `problem_id` - The ID of the problem instance to retrieve (corresponds to the filename without extension).
///
/// # Returns
/// * `Json<ProblemInstance>` - The problem instance data in JSON format or an error
///   if the problem is not found or if another processing task is currently running.
async fn get_problem(
    State(state): State<AppState>,
    Path(problem_id): Path<String>,
) -> Result<Json<ProblemInstance>, ServerError> {
    // check if any processing task is currently running
    let mut solver_state = state.solver_state.lock().await;

    if let ProcessingState::Processing(_) = *solver_state {
        return Err(ServerError::ProcessingAlreadyRunning);
    }

    // create cancellation token for the new processing task
    let token = CancellationToken::new();
    let task_token = token.clone();

    let handle = tokio::task::spawn_blocking(move || {
        let cancellation = || task_token.is_cancelled();
        let ctx = ExecutionContext::new(&cancellation);

        // define file path to the problem instance
        let problem_path = format!("./data/{}.tsp", problem_id);

        // try to read and parse the problem
        let problem_data = fs::read_to_string(problem_path);
        let problem = match problem_data {
            Ok(data) => try_parse(data)?.try_into_problem_instance(ctx)?,
            Err(_) => return Err(ServerError::ProblemNotFound(problem_id)),
        };

        Ok(problem)
    });

    *solver_state = ProcessingState::Processing(token);
    drop(solver_state);

    // wait for the processing task to complete and get the result
    let result = handle.await;

    // reset solver state to idle after completion
    *state.solver_state.lock().await = ProcessingState::Idle;

    match result {
        Ok(Ok(problem)) => Ok(Json(problem)),
        Ok(Err(e)) => Err(e),
        Err(e) if e.is_cancelled() => Err(ServerError::ProcessingCancelled),
        Err(e) => Err(ServerError::from(e)),
    }
}
