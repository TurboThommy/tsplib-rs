use serde::Deserialize;
use tsplib_core::enums::AlgorithmType;

#[derive(Deserialize)]
pub struct StartSolverRequest {
    pub algorithm: AlgorithmType,
    pub problem_id: String,
    pub start_node: Option<usize>,
}
