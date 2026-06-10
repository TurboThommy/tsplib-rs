use serde::Serialize;

use crate::enums::EdgeWeightType;

/// Represents the source of distance information in a TSP problem instance.
#[derive(Serialize)]
pub enum DistanceSource {
    /// The distance information are explicitly provided as an adjacency matrix in the problem instance.
    Explicit(Vec<Vec<i32>>),

    /// The distance information are derived from the node coordinates using a specific edge weight type.
    Geometric(EdgeWeightType),
}

impl DistanceSource {
    pub fn heap_size(&self) -> usize {
        match self {
            // Geometric distance source does only store the edge weight type, no heap allocations
            DistanceSource::Geometric(_) => 0,

            // Explicit distance source stores a full adjacency matrix
            DistanceSource::Explicit(matrix) => {
                matrix.len() * matrix.first().map_or(0, |r| r.len()) * std::mem::size_of::<i32>()
            }
        }
    }
}
