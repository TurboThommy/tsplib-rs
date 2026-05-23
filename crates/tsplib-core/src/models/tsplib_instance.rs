//! Defines the `ProblemInstance` struct, which represents a TSP problem instance with its name, type, nodes, adjacency matrix, and optional fixed edges.
//! It also includes methods for estimating heap size and converting from a `TSPLIBInstance`.
use serde::Serialize;

use crate::{
    context::ExecutionContext,
    enums::{ConversionError, InstanceError, ProblemType},
    minimum_spanning_tree::try_get_mst_kruskal,
    models::{Node, TsplibDefinition},
};

/// Represents a TSP problem instance as graph (collection of nodes and an adjacency matrix).
#[derive(Serialize)]
pub struct TsplibInstance {
    /// The ID of the problem instance, typically derived from the filename without extension.
    pub problem_id: String,

    /// The name of the problem instance.
    pub name: String,

    /// The TYPE of the problem instance.
    pub problem_type: ProblemType,

    /// The nodes in the problem instance.
    pub nodes: Vec<Node>,

    /// The adjacency matrix representing the distances between nodes.
    pub adjacency_matrix: Vec<Vec<i32>>,

    /// Optional fixed edges that must be included in the solution.
    pub fixed_edges: Option<Vec<(usize, usize)>>,
}

impl TsplibInstance {
    /// Estimates the heap size of the `ProblemInstance` by calculating the size of its nodes and adjacency matrix.
    /// This is a rough estimation and may not be exact due to Rust's memory management and optimizations.
    ///
    /// # Returns
    /// * `usize` - The estimated heap size in bytes.
    pub fn heap_size(&self) -> usize {
        let nodes_size = self.nodes.len() * std::mem::size_of::<Node>();
        let matrix_size = self.adjacency_matrix.len()
            * self.adjacency_matrix.first().map_or(0, |r| r.len())
            * std::mem::size_of::<i32>();

        nodes_size + matrix_size
    }

    /// Tries to get the distance between two nodes from the adjacency matrix.
    ///
    /// # Arguments
    /// * `from` - The ID of the starting node (1-based index).
    /// * `to` - The ID of the destination node (1-based index).
    ///
    /// # Returns
    /// * `Result<i32, InstanceError>` - The distance between the nodes if valid, or an error if the node IDs are invalid.
    pub fn try_get_distance(&self, from: usize, to: usize) -> Result<i32, InstanceError> {
        if from == 0
            || to == 0
            || from > self.adjacency_matrix.len()
            || to > self.adjacency_matrix.len()
        {
            return Err(InstanceError::DistanceInvalidNodeId(
                from,
                to,
                self.adjacency_matrix.len(),
            ));
        }
        Ok(self.adjacency_matrix[from - 1][to - 1])
    }

    /// Tries to compute the minimum spanning tree (MST) of the TSP instance using Kruskal's algorithm.
    ///
    /// # Returns
    /// * `Result<Vec<(usize, usize, i32)>, ConversionError>` - A vector of edges in the MST,
    ///   where each edge is represented as a tuple of (node1_id, node2_id, distance). Returns an error if the MST cannot be computed.
    pub fn try_get_mst_kruskal(&self) -> Result<Vec<(usize, usize, i32)>, ConversionError> {
        try_get_mst_kruskal(self)
    }
}

impl TryFrom<TsplibDefinition> for TsplibInstance {
    type Error = ConversionError;

    fn try_from(tsp_instance: TsplibDefinition) -> Result<Self, ConversionError> {
        (&tsp_instance).try_into()
    }
}

impl TryFrom<&TsplibDefinition> for TsplibInstance {
    type Error = ConversionError;

    fn try_from(tsp_instance: &TsplibDefinition) -> Result<Self, ConversionError> {
        tsp_instance.try_into_problem_instance(ExecutionContext::default())
    }
}
