use serde::Deserialize;
use tsplib_solver::{SolverOptions, enums::SolverAlgorithm};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartSolverRequest {
    pub algorithm: SolverAlgorithm,
    pub problem_id: String,
    pub start_node: Option<usize>,
    pub solver_options: Option<SolverOptions>,
}
