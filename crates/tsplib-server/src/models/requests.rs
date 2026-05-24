use serde::Deserialize;
use tsplib_solver::enums::SolverAlgorithm;

#[derive(Deserialize)]
pub struct StartSolverRequest {
    pub algorithm: SolverAlgorithm,
    pub problem_id: String,
    pub start_node: Option<usize>,
}
