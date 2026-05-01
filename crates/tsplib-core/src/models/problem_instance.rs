use crate::models::{ConversionError, Node, TSPLIBInstance};

pub struct ProblemInstance {
    pub name: String,
    pub nodes: Vec<Node>,
    pub adjacency_matrix: Vec<Vec<i32>>,
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
            * std::mem::size_of::<f64>();

        nodes_size + matrix_size
    }
}

impl TryFrom<TSPLIBInstance> for ProblemInstance {
    type Error = ConversionError;

    fn try_from(tsp_instance: TSPLIBInstance) -> Result<Self, ConversionError> {
        let nodes = tsp_instance.try_extract_nodes()?;
        let adjacency_matrix = tsp_instance.try_calculate_adjacency_matrix()?;

        Ok(ProblemInstance {
            name: tsp_instance.name.clone(),
            nodes,
            adjacency_matrix,
        })
    }
}

impl TryFrom<&TSPLIBInstance> for ProblemInstance {
    type Error = ConversionError;

    fn try_from(tsp_instance: &TSPLIBInstance) -> Result<Self, ConversionError> {
        let nodes = tsp_instance.try_extract_nodes()?;
        let adjacency_matrix = tsp_instance.try_calculate_adjacency_matrix()?;

        Ok(ProblemInstance {
            name: tsp_instance.name.clone(),
            nodes,
            adjacency_matrix,
        })
    }
}
