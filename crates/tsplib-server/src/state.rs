//! Application state management for the TSP solver server.
use std::{collections::HashMap, fs, sync::Arc};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// Represents the current state of the TSP solver,which can either be idle or processing a problem instance.
#[derive(Debug)]
pub enum ProcessingState {
    Idle,
    Processing(CancellationToken),
}

/// The shared application state for the TSP solver server, containing the current solver state.
#[derive(Clone, Debug)]
pub struct AppState {
    pub solver_state: Arc<Mutex<ProcessingState>>,
    pub solutions: Arc<HashMap<String, i64>>,
}

impl AppState {
    /// Creates a new instance of the application state with the solver state initialized to idle.
    pub fn new() -> Self {
        AppState {
            solver_state: Arc::new(Mutex::new(ProcessingState::Idle)),
            solutions: Arc::new(parse_solutions()),
        }
    }
}

fn parse_solutions() -> HashMap<String, i64> {
    tracing::info!("Parsing solutions file from ./data directory");

    let content = fs::read_to_string("./data/solutions")
        .expect("Failed to read ./data/solutions (run from workspace root?)");

    let solutions: HashMap<String, i64> = content
        .lines()
        .filter_map(|line| {
            let (name, rest) = line.split_once(':')?;
            let value = rest.split_whitespace().next()?.parse().ok()?;
            Some((name.trim().to_string(), value))
        })
        .collect();

    tracing::info!(
        solutions = solutions.len(),
        "Successfully parsed solution file"
    );

    solutions
}
