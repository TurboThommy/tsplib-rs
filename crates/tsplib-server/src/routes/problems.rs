//! Handlers for the REST API routes related to problem instances.
use crate::{
    errors::ServerError,
    models::responses::ProblemDescriptionResponse,
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
    models::TsplibInstance,
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
    // read all tsp files from the "./data" directory
    // filter relevant metadata
    let problems = try_read_tsp_files("./data")?
        .iter()
        .map(|(problem_id, problem_data)| {
            let mut specification = SpecificationPart::new();

            let metadata_lines = problem_data
                .lines()
                .filter(|line| line.contains(':'))
                .collect::<Vec<_>>();

            for line in metadata_lines {
                let parts = line.split(':').map(|s| s.trim()).collect::<Vec<_>>();
                if parts.len() != 2 {
                    Err(ServerError::MetadataParseError(problem_id.to_string()))?;
                }
                try_parse_header_line(parts[0], parts[1], &mut specification)?;
            }

            ProblemDescriptionResponse::try_from_specification(problem_id.clone(), &specification)
        })
        .collect::<Result<Vec<_>, ServerError>>()?;

    // let problems = fs::read_dir("./data")?
    //     .filter_map(|entry| {
    //         let entry = entry.ok()?;
    //         let path = entry.path();
    //         if !path.is_file() {
    //             return None;
    //         }
    //         if path.extension()? != "tsp" {
    //             return None;
    //         }
    //         Some(path.file_stem()?.to_string_lossy().to_string())
    //     })
    //     .collect::<Vec<_>>();

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
) -> Result<Json<TsplibInstance>, ServerError> {
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
        let (problem_id, problem_data) = try_read_tsp_file(problem_path.as_ref())?;
        let problem = try_parse(problem_id, problem_data)?.try_into_problem_instance(ctx)?;

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
