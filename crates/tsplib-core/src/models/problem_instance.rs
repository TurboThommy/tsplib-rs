//! Defines the `ProblemInstance` struct, which represents a TSP problem instance with its name, type, nodes, adjacency matrix, and optional fixed edges.
//! It also includes methods for estimating heap size and converting from a `TSPLIBInstance`.
use crate::{
    enums::ConversionError,
    enums::{DataSection, ProblemType},
    models::{Node, TSPLIBInstance},
};

/// Represents a TSP problem instance as graph (collection of nodes and an adjacency matrix).
pub struct ProblemInstance {
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

impl ProblemInstance {
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
}

impl TryFrom<TSPLIBInstance> for ProblemInstance {
    type Error = ConversionError;

    fn try_from(tsp_instance: TSPLIBInstance) -> Result<Self, ConversionError> {
        let nodes = tsp_instance.try_extract_nodes()?;
        let adjacency_matrix = tsp_instance.try_calculate_adjacency_matrix()?;

        // check for fixed edges
        let fixed_edges = tsp_instance.data_sections.iter().find_map(|s| match s {
            DataSection::FixedEdgesSection(section) => Some(section.clone()),
            _ => None,
        });

        Ok(ProblemInstance {
            name: tsp_instance.name.clone(),
            problem_type: tsp_instance.problem_type,
            nodes,
            adjacency_matrix,
            fixed_edges,
        })
    }
}

impl TryFrom<&TSPLIBInstance> for ProblemInstance {
    type Error = ConversionError;

    fn try_from(tsp_instance: &TSPLIBInstance) -> Result<Self, ConversionError> {
        let nodes = tsp_instance.try_extract_nodes()?;
        let adjacency_matrix = tsp_instance.try_calculate_adjacency_matrix()?;

        // check for fixed edges
        let fixed_edges = tsp_instance.data_sections.iter().find_map(|s| match s {
            DataSection::FixedEdgesSection(section) => Some(section.clone()),
            _ => None,
        });

        Ok(ProblemInstance {
            name: tsp_instance.name.clone(),
            problem_type: tsp_instance.problem_type.clone(),
            nodes,
            adjacency_matrix,
            fixed_edges,
        })
    }
}
