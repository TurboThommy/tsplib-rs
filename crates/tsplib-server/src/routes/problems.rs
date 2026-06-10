//! Handlers for the REST API routes related to problem instances.

use crate::{
    errors::ServerError,
    models::responses::{ProblemDescriptionResponse, TsplibInstanceWithMatrixResponse},
    state::{AppState, ProcessingState},
};

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use tokio_util::sync::CancellationToken;
use tsplib_core::context::ExecutionContext;

/// Router for problem-related endpoints.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/problems", get(get_problems))
        .route("/problems/{problemId}", get(get_problem))
        .route(
            "/problems/{problemId}/adjacency_matrix",
            get(get_adjacency_matrix),
        )
}

/// Get the list of available TSP problem instances from the "./data" directory.
///
/// # Arguments
/// * `State(state): State<AppState>` - The shared application state containing the preloaded problem instances and their metadata.
///
/// # Returns
/// * `Json<Vec<String>>` - A list of problem IDs (filenames without extension) in JSON format
///   or an error if the directory cannot be read.
async fn get_problems(
    State(state): State<AppState>,
) -> Result<Json<Vec<ProblemDescriptionResponse>>, ServerError> {
    tracing::info!("Received request to get list of problems");

    // read all tsp files from the "./data" directory
    // filter relevant metadata
    tracing::debug!("Reading TSP files from ./data directory");

    // use the preloaded instances from the app state to avoid reading and parsing the files again
    let problems = state
        .instances
        .values()
        .map(|i| i.try_into())
        .collect::<Result<Vec<_>, _>>()?;

    tracing::info!(
        problem_count = problems.len(),
        "Successfully read problems from ./data directory"
    );

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
) -> Result<Json<TsplibInstanceWithMatrixResponse>, ServerError> {
    tracing::info!(problem_id = %problem_id, state = ?state.solver_state, "Received request to get problem instance");

    // check if any processing task is currently running
    let mut solver_state = state.solver_state.lock().await;

    if let ProcessingState::Processing(_) = *solver_state {
        return Err(ServerError::ProcessingAlreadyRunning);
    }

    // create cancellation token for the new processing task
    let token = CancellationToken::new();
    let task_token = token.clone();

    let instance = state
        .get_instance(&problem_id)
        .ok_or(ServerError::ProblemInstanceNotFound(problem_id.clone()))?;

    tracing::debug!("Starting processing task");

    let handle = tokio::task::spawn_blocking(move || {
        let cancellation = || task_token.is_cancelled();
        let ctx = ExecutionContext::new(&cancellation);

        if ctx.is_cancelled() {
            return Err(ServerError::ProcessingCancelled);
        }

        tracing::debug!(problem_id = %instance.problem_id, "Attempting to create response with full adjacency matrix");
        let response = TsplibInstanceWithMatrixResponse::try_from_instance(&instance, ctx)?;

        if ctx.is_cancelled() {
            return Err(ServerError::ProcessingCancelled);
        }

        Ok(response)
    });

    tracing::debug!("Processing task started, updating app state");
    *solver_state = ProcessingState::Processing(token);
    drop(solver_state);
    tracing::debug!(state = ?state.solver_state, "App state updated");

    // wait for the processing task to complete and get the result
    tracing::debug!("Waiting for processing task to complete");
    let result = handle.await;

    // reset solver state to idle after completion
    tracing::debug!("Processing task completed, resetting solver state to idle");
    *state.solver_state.lock().await = ProcessingState::Idle;
    tracing::debug!(state = ?state.solver_state, "App state updated, returning result");

    match result {
        Ok(Ok(response)) => {
            tracing::info!(problem_id = %response.problem_id, "Successfully processed problem instance");
            Ok(Json(response))
        }
        Ok(Err(e)) => Err(e),
        Err(e) if e.is_cancelled() => Err(ServerError::ProcessingCancelled),
        Err(e) => Err(ServerError::from(e)),
    }
}

/// Get the full adjacency matrix for a specific TSP problem instance by its ID.
///
/// # Arguments
/// * `problem_id` - The ID of the problem instance to retrieve the adjacency matrix for (corresponds to the filename without extension).
///
/// # Returns
/// * `Json<Vec<Vec<i32>>>` - The adjacency matrix of the problem instance in JSON format or an error if the problem is not found,
///   if another processing task is currently running, or if the processing task was cancelled.
#[allow(clippy::needless_range_loop)]
async fn get_adjacency_matrix(
    State(state): State<AppState>,
    Path(problem_id): Path<String>,
) -> Result<Json<Vec<Vec<i32>>>, ServerError> {
    tracing::info!(problem_id = %problem_id, state = ?state.solver_state, "Received request to get full adjacency matrix for problem instance");

    // check if any processing task is currently running
    let mut solver_state = state.solver_state.lock().await;

    if let ProcessingState::Processing(_) = *solver_state {
        return Err(ServerError::ProcessingAlreadyRunning);
    }

    // create cancellation token for the new processing task
    let token = CancellationToken::new();
    let task_token = token.clone();

    let instance = state
        .get_instance(&problem_id)
        .ok_or(ServerError::ProblemInstanceNotFound(problem_id.clone()))?;

    tracing::debug!("Starting processing task");

    let handle = tokio::task::spawn_blocking(move || {
        let cancellation = || task_token.is_cancelled();
        let ctx = ExecutionContext::new(&cancellation);

        if ctx.is_cancelled() {
            return Err(ServerError::ProcessingCancelled);
        }

        tracing::debug!(problem_id = %instance.problem_id, "Attempting to create full adjacency matrix");
        let n = instance.nodes.len();
        let mut adjacency_matrix = vec![vec![0; n]; n];

        for i in 0..n {
            if ctx.is_cancelled() {
                return Err(ServerError::ProcessingCancelled);
            }

            for j in i + 1..n {
                if i == j {
                    continue;
                }

                let distance = instance.try_get_distance(i + 1, j + 1)?;
                adjacency_matrix[i][j] = distance;
                adjacency_matrix[j][i] = distance;
            }
        }

        if ctx.is_cancelled() {
            return Err(ServerError::ProcessingCancelled);
        }

        Ok(adjacency_matrix)
    });

    tracing::debug!("Processing task started, updating app state");
    *solver_state = ProcessingState::Processing(token);
    drop(solver_state);
    tracing::debug!(state = ?state.solver_state, "App state updated");

    // wait for the processing task to complete and get the result
    tracing::debug!("Waiting for processing task to complete");
    let result = handle.await;

    // reset solver state to idle after completion
    tracing::debug!("Processing task completed, resetting solver state to idle");
    *state.solver_state.lock().await = ProcessingState::Idle;
    tracing::debug!(state = ?state.solver_state, "App state updated, returning result");

    match result {
        Ok(Ok(response)) => {
            tracing::info!(
                problem_id = problem_id,
                "Successfully processed problem instance"
            );
            Ok(Json(response))
        }
        Ok(Err(e)) => Err(e),
        Err(e) if e.is_cancelled() => Err(ServerError::ProcessingCancelled),
        Err(e) => Err(ServerError::from(e)),
    }
}

// TODO: add endpoint to get the full problem without adjacency matrix

// TODO: add endpoint to get a specific edge

// TODO: add endpoint to get all edges for a specific node
