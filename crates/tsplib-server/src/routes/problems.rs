//! Handlers for the REST API routes related to problem instances.
use crate::{errors::ServerError, state::AppState};

use axum::{Json, Router, extract::Path, routing::get};
use std::fs;
use tsplib_core::models::ProblemInstance;
use tsplib_parser::try_parse;

/// Router for problem-related endpoints.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/problems", get(get_problems))
        .route("/problems/{problemId}", get(get_problem))
}

/// Get the list of available TSP problem instances from the "./data" directory.
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

async fn get_problem(Path(problem_id): Path<String>) -> Result<Json<ProblemInstance>, ServerError> {
    // define file path to the problem instance
    let problem_path = format!("./data/{}.tsp", problem_id);

    // try to read and parse the problem
    let problem_data = fs::read_to_string(problem_path);
    let problem: ProblemInstance = match problem_data {
        Ok(data) => try_parse(data)?.try_into()?,
        Err(_) => return Err(ServerError::ProblemNotFound(problem_id)),
    };

    Ok(Json(problem))
}
