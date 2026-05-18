use serde::Serialize;

#[derive(Serialize)]
pub struct TspSolution {
    pub tour: Vec<usize>,
    pub cost: i64,
}
