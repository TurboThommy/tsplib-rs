use crate::errors::ServerError;

use axum::{Json, Router, routing::get};
use std::fs;

/// Router for problem-related endpoints.
pub fn router() -> Router {
    Router::new().route("/problems", get(get_problems))
}

/// Get the list of available TSP problem instances from the "./data" directory.
pub async fn get_problems() -> Result<Json<Vec<String>>, ServerError> {
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
