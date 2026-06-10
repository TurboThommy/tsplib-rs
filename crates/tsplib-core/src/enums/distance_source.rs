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
