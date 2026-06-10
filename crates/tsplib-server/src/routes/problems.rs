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
use tsplib_core::{
    context::ExecutionContext,
    reader::{try_read_tsp_file, try_read_tsp_files},
};
use tsplib_parser::{SpecificationPart, try_parse, try_parse_header_line};

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
async fn get_problems() -> Result<Json<Vec<ProblemDescriptionResponse>>, ServerError> {
    tracing::info!("Received request to get list of problems");

    // read all tsp files from the "./data" directory
    // filter relevant metadata
    tracing::debug!("Reading TSP files from ./data directory");
    let problems = try_read_tsp_files("./data")?
        .iter()
        .map(|(problem_id, problem_data)| {
            // create a new specification part to hold the parsed metadata
            let mut specification = SpecificationPart::new();

            // extract lines containing metadata (lines with a colon)
            let metadata_lines = problem_data
                .lines()
                .filter(|line| line.contains(':'))
                .collect::<Vec<_>>();

            // parse metadata lines and populate the specification
            for line in metadata_lines {
                let parts = line.split(':').map(|s| s.trim()).collect::<Vec<_>>();
                if parts.len() != 2 {
                    Err(ServerError::MetadataParseError(problem_id.to_string()))?;
                }
                try_parse_header_line(parts[0], parts[1], &mut specification)?;
            }

            // convert the specification to a problem description response
            ProblemDescriptionResponse::try_from_specification(problem_id.clone(), &specification)
        })
        .collect::<Result<Vec<_>, ServerError>>()?;

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

    tracing::debug!("Starting processing task");

    let handle = tokio::task::spawn_blocking(move || {
        let cancellation = || task_token.is_cancelled();
        let ctx = ExecutionContext::new(&cancellation);

        // define file path to the problem instance
        let problem_path = format!("./data/{}.tsp", problem_id);

        // try to read and parse the problem
        tracing::debug!(problem_path = %problem_path, "Attempting to read and parse problem instance");
        let (problem_id, problem_data) = try_read_tsp_file(problem_path.as_ref())?;

        let problem = try_parse(problem_id, problem_data)?.try_into_problem_instance(ctx)?;
        tracing::debug!(problem_id = %problem.problem_id, "Successfully parsed problem instance.");

        if ctx.is_cancelled() {
            return Err(ServerError::ProcessingCancelled);
        }

        tracing::debug!(problem_id = %problem.problem_id, "Attempting to create response with full adjacency matrix");
        let response = TsplibInstanceWithMatrixResponse::try_from_instance(&problem)?;

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
            tracing::info!(problem_count = %response.problem_id, "Successfully processed problem instance");
            Ok(Json(response))
        }
        Ok(Err(e)) => Err(e),
        Err(e) if e.is_cancelled() => Err(ServerError::ProcessingCancelled),
        Err(e) => Err(ServerError::from(e)),
    }
}
