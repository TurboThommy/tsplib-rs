//! Defines the `ProblemInstance` struct, which represents a TSP problem instance with its name, type, nodes, adjacency matrix, and optional fixed edges.
//! It also includes methods for estimating heap size and converting from a `TSPLIBInstance`.
use serde::Serialize;

use crate::{
    context::ExecutionContext,
    enums::{ConversionError, DistanceSource, InstanceError, MstComputationError, ProblemType},
    minimum_spanning_tree::{try_get_mst_boruvka, try_get_mst_kruskal, try_get_mst_prim},
    models::{Graph, Node, TsplibDefinition},
};

/// Represents a TSP problem instance as graph (collection of nodes and an adjacency matrix).
#[derive(Debug, Serialize)]
pub struct TsplibInstance {
    /// The ID of the problem instance, typically derived from the filename without extension.
    pub problem_id: String,

    /// The name of the problem instance.
    pub name: String,

    /// The TYPE of the problem instance.
    pub problem_type: ProblemType,

    /// The nodes in the problem instance.
    pub nodes: Vec<Node>,

    /// The distance source for the problem instance, which can be used
    /// to compute distances on demand instead of storing a full adjacency matrix.
    pub distance_source: DistanceSource,

    /// Optional fixed edges that must be included in the solution.
    pub fixed_edges: Option<Vec<(usize, usize)>>,
}

impl TsplibInstance {
    /// Estimates the heap size of the `TsplibInstance` by calculating the size of its nodes and adjacency matrix.
    /// This is a rough estimation and may not be exact due to Rust's memory management and optimizations.
    ///
    /// # Returns
    /// * `usize` - The estimated heap size in bytes.
    pub fn heap_size(&self) -> usize {
        let nodes_size = self.nodes.len() * std::mem::size_of::<Node>();
        nodes_size + self.distance_source.heap_size()
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
        if from == 0 || to == 0 || from > self.nodes.len() || to > self.nodes.len() {
            return Err(InstanceError::DistanceInvalidNodeId(
                from,
                to,
                self.nodes.len(),
            ));
        }

        match &self.distance_source {
            DistanceSource::Explicit(matrix) => Ok(matrix[from - 1][to - 1]),
            DistanceSource::Geometric(edge_weight_type) => {
                let node_from = &self.nodes[from - 1];
                let node_to = &self.nodes[to - 1];
                node_from
                    .try_get_distance(node_to, edge_weight_type)
                    .map_err(|e| InstanceError::GetDistanceError(from, to, e.to_string()))
            }
        }
    }

    /// Tries to compute the minimum spanning tree (MST) of the TSP instance using Kruskal's algorithm.
    ///
    /// # Returns
    /// * `Result<Graph, ConversionError>` - Graph struct containing the edges in the MST, or an error if the MST cannot be computed.
    pub fn try_get_mst_kruskal(&self, ctx: ExecutionContext) -> Result<Graph, MstComputationError> {
        try_get_mst_kruskal(self, ctx)
    }

    /// Tries to compute the minimum spanning tree (MST) of the TSP instance using Prim's algorithm starting from a specified node.
    ///
    /// # Arguments
    /// * `start_node` - The ID of the starting node for Prim's algorithm (1-based index).
    ///
    /// # Returns
    /// * `Result<Graph, ConversionError>` - Graph struct containing the edges in the MST,
    ///   or an error if the adjacency matrix is empty or the start node is invalid.
    pub fn try_get_mst_prim(
        &self,
        start_node: usize,
        ctx: ExecutionContext,
    ) -> Result<Graph, MstComputationError> {
        try_get_mst_prim(self, start_node, ctx)
    }

    /// Tries to compute the minimum spanning tree (MST) of the TSP instance using Borůvka's algorithm.
    ///
    /// # Returns
    /// * `Result<Graph, ConversionError>` - Graph struct containing the edges in the MST, or an error if the MST cannot be computed.
    pub fn try_get_mst_boruvka(&self, ctx: ExecutionContext) -> Result<Graph, MstComputationError> {
        try_get_mst_boruvka(self, ctx)
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
