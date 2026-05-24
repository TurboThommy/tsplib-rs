//! Defines the TspSolution struct, which represents a solution to the Traveling Salesman Problem (TSP).
use serde::Serialize;

/// Represents a solution to the Traveling Salesman Problem (TSP).
#[derive(Serialize)]
pub struct TspSolution {
    /// The tour of cities in the order they are visited.
    pub tour: Vec<usize>,
    /// The total cost of the tour.
    pub cost: i64,
}
