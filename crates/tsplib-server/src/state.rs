//! Application state management for the TSP solver server.
use std::sync::Arc;
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
}

impl AppState {
    /// Creates a new instance of the application state with the solver state initialized to idle.
    pub fn new() -> Self {
        AppState {
            solver_state: Arc::new(Mutex::new(ProcessingState::Idle)),
        }
    }
}
