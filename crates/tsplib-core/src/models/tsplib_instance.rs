//! This module defines the `TSPLIBInstance` struct, which represents a TSP instance as defined in the TSPLIB format.
//! The struct contains all required and optional fields, as well as the data sections of the instance.
//! It also provides methods for extracting nodes and calculating the adjacency matrix based on the available data sections and edge weight type.
use crate::context::ExecutionContext;
use crate::distances::{
    distance_att, distance_ceil_2d, distance_euc_2d, distance_geo, distance_man_2d, distance_max_2d,
};
use crate::enums::{
    DataSection, DisplayDataType, EdgeDataFormat, EdgeWeightFormat, EdgeWeightType, NodeCoordType,
    ProblemType,
};

use crate::enums::ConversionError;
use crate::models::{Node, ProblemInstance};

use std::{fmt, vec};

/// A struct representing a TSP instance, containing all required and optional fields, as well as the data sections.
/// This struct can be used to represent any TSP instance defined in the TSPLIB format.
#[derive(Debug)]
pub struct TSPLIBInstance {
    // required
    /// The ID of the TSP instance, which corresponds to the filename (without extension) of the TSPLIB file from which the instance was parsed.
    /// This field is required and must be a non-empty string.
    pub problem_id: String,
    /// The name of the TSP instance, as specified in the TSPLIB file. This field is required and must be a non-empty string.
    pub name: String,
    /// The type of the TSP instance, as specified in the TSPLIB file. This field is required and must be one of the variants of the `ProblemType` enum.
    pub problem_type: ProblemType,
    /// The dimension of the TSP instance, as specified in the TSPLIB file. This field is required and must be a positive integer.
    pub dimension: usize,
    /// The edge weight type of the TSP instance, as specified in the TSPLIB file. This field is required and must be one of the variants of the `EdgeWeightType` enum.
    pub edge_weight_type: EdgeWeightType,

    // optional
    /// The comment lines of the TSP instance, as specified in the TSPLIB file. This field is optional and can be `None` if no comment lines are present. If present, it must be a vector of strings, where each string represents a line of comment.
    pub comment: Option<Vec<String>>,
    /// The capacity of the TSP instance, as specified in the TSPLIB file. This field is optional and can be `None` if no capacity is specified.
    pub capacity: Option<usize>,
    /// The edge weight format of the TSP instance, as specified in the TSPLIB file. This field is optional and can be `None` if no edge weight format is specified.
    pub edge_weight_format: Option<EdgeWeightFormat>,
    /// The edge data format of the TSP instance, as specified in the TSPLIB file. This field is optional and can be `None` if no edge data format is specified.
    pub edge_data_format: Option<EdgeDataFormat>,
    /// The node coordinate type of the TSP instance, as specified in the TSPLIB file. This field is optional and can be `None` if no node coordinate type is specified.
    pub node_coord_type: Option<NodeCoordType>,
    /// The display data type of the TSP instance, as specified in the TSPLIB file. This field is optional and can be `None` if no display data type is specified.
    pub display_data_type: Option<DisplayDataType>,

    // data sections
    /// The data sections of the TSP instance, as specified in the TSPLIB file.
    /// This field is required and must be a vector of `DataSection` enums, where each enum variant represents a different type of data section (e.g., `NodeCoordSection`, `EdgeWeightSection`, etc.).
    /// The order of the data sections in the vector should match the order in which they appear in the TS
    pub data_sections: Vec<DataSection>,
}

impl fmt::Display for TSPLIBInstance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = Vec::new();

        s.push(format!("NAME: {}", self.name));
        s.push(format!("TYPE: {:?}", self.problem_type));
        s.push(format!("DIMENSION: {}", self.dimension));
        s.push(format!("EDGE_WEIGHT_TYPE: {}", self.edge_weight_type));

        if let Some(comment) = &self.comment {
            comment
                .iter()
                .for_each(|line| s.push(format!("COMMENT: {}", line)));
        }

        if let Some(capacity) = &self.capacity {
            s.push(format!("CAPACITY: {}", capacity));
        }

        if let Some(edge_weight_format) = &self.edge_weight_format {
            s.push(format!("EDGE_WEIGHT_FORMAT: {}", edge_weight_format));
        }

        if let Some(edge_data_format) = &self.edge_data_format {
            s.push(format!("EDGE_DATA_FORMAT: {}", edge_data_format));
        }

        if let Some(node_coord_type) = &self.node_coord_type {
            s.push(format!("NODE_COORD_TYPE: {}", node_coord_type));
        }

        if let Some(display_data_type) = &self.display_data_type {
            s.push(format!("DISPLAY_DATA_TYPE: {}", display_data_type));
        }

        s.push(
            self.data_sections
                .iter()
                .map(|section| format!("{}", section))
                .collect::<Vec<_>>()
                .join("\n"),
        );

        write!(f, "{}", s.join("\n"))
    }
}

impl TSPLIBInstance {
    /// Converts the `TSPLIBInstance` into a `ProblemInstance` by extracting the nodes and calculating the adjacency matrix based on the available data sections and edge weight type.
    /// The conversion process may involve checking for cancellation through the provided execution context, allowing for responsive cancellation of long-running operations.
    ///
    /// # Arguments
    /// * `ctx` - The execution context, which can be used to check for cancellation during the conversion process.
    ///   If the execution is cancelled, the function will return a `ConversionError::Cancelled` error.
    ///
    /// # Returns
    /// * `Result<ProblemInstance, ConversionError>` - A result containing the converted `ProblemInstance` if successful, or a `ConversionError`
    ///   if the conversion fails due to missing required data sections, unsupported edge weight types, or if the execution was cancelled.
    pub fn try_into_problem_instance(
        &self,
        ctx: ExecutionContext,
    ) -> Result<ProblemInstance, ConversionError> {
        let nodes = self.try_extract_nodes()?;
        let adjacency_matrix = self.try_calculate_adjacency_matrix(ctx)?;

        // check for fixed edges
        let fixed_edges = self.data_sections.iter().find_map(|s| match s {
            DataSection::FixedEdgesSection(section) => Some(section.clone()),
            _ => None,
        });

        Ok(ProblemInstance {
            problem_id: self.problem_id.clone(),
            name: self.name.clone(),
            problem_type: self.problem_type.clone(),
            nodes,
            adjacency_matrix,
            fixed_edges,
        })
    }

    /// Extracts the nodes from the TSP instance based on the available data sections and edge weight type.
    /// The extraction logic depends on the edge weight type and the presence of specific data sections
    /// (e.g., NODE_COORD_SECTION, EDGE_WEIGHT_SECTION, DISPLAY_DATA_SECTION).
    ///
    /// # Returns
    /// * `Result<Vec<Node>, ConversionError>` - A result containing the vector of `Node` structs if successful,
    ///   or a `ConversionError` if the required data sections are missing or if the edge weight type is unsupported for node extraction.
    pub(super) fn try_extract_nodes(&self) -> Result<Vec<Node>, ConversionError> {
        match self.edge_weight_type {
            // EDGE_WEIGHT_SECTION
            EdgeWeightType::Explicit => Ok(self.try_extract_nodes_from_edge_weight_section()?),

            // NODE_COORD_SECTION with 2D coordinates
            EdgeWeightType::Euc2D
            | EdgeWeightType::Max2D
            | EdgeWeightType::Man2D
            | EdgeWeightType::Ceil2D
            | EdgeWeightType::Geo
            | EdgeWeightType::Att => Ok(self.try_extract_nodes_from_node_coord_section_2d()?),

            _ => Err(ConversionError::UnsupportedEdgeWeightType(
                self.edge_weight_type,
            )),
        }
    }

    /// Calculates the adjacency matrix of edge weights for the TSP instance based on the available data sections and edge weight type.
    ///
    /// # Arguments
    /// * `ctx` - The execution context, which can be used to check for cancellation during the calculation of the adjacency matrix.
    ///   If the execution is cancelled, the function will return a `ConversionError::Cancelled` error.
    ///
    /// # Returns
    /// * `Result<Vec<Vec<i32>>, ConversionError>` - A result containing the adjacency matrix if successful,
    ///   or a `ConversionError` if the required data sections are missing
    pub(super) fn try_calculate_adjacency_matrix(
        &self,
        ctx: ExecutionContext<'_>,
    ) -> Result<Vec<Vec<i32>>, ConversionError> {
        match self.edge_weight_type {
            // EDGE_WEIGHT_SECTION
            EdgeWeightType::Explicit => Ok(self.try_calculate_adjacency_matrix_edge_weights(ctx)?),

            // NODE_COORD_SECTION with 2D coordinates
            EdgeWeightType::Euc2D
            | EdgeWeightType::Max2D
            | EdgeWeightType::Man2D
            | EdgeWeightType::Ceil2D
            | EdgeWeightType::Geo
            | EdgeWeightType::Att => Ok(self.try_calculate_adjacency_matrix_2d(ctx)?),

            _ => Err(ConversionError::UnsupportedEdgeWeightType(
                self.edge_weight_type,
            )),
        }
    }

    /// Calculates the adjacency matrix of edge weights for the TSP instance based on the NODE_COORD_SECTION data, assuming 2D coordinates.
    ///
    /// # Arguments
    /// * `ctx` - The execution context, which can be used to check for cancellation during the calculation of the adjacency matrix.
    ///   If the execution is cancelled, the function will return a `ConversionError::Cancelled` error.
    ///
    /// # Returns
    /// * `Result<Vec<Vec<f64>>, ConversionError>` - A result containing the adjacency matrix if successful,
    ///   or a `ConversionError` if the required NODE_COORD_SECTION is missing in the instance data.
    fn try_calculate_adjacency_matrix_2d(
        &self,
        ctx: ExecutionContext<'_>,
    ) -> Result<Vec<Vec<i32>>, ConversionError> {
        // check for NODE_COORD_SECTION
        let node_coord_section = self.try_get_node_coord_section_2d()?;

        // set the distance function based on the edge weight type
        let distance_fn = match self.edge_weight_type {
            EdgeWeightType::Euc2D => distance_euc_2d,
            EdgeWeightType::Max2D => distance_max_2d,
            EdgeWeightType::Man2D => distance_man_2d,
            EdgeWeightType::Ceil2D => distance_ceil_2d,
            EdgeWeightType::Geo => distance_geo,
            EdgeWeightType::Att => distance_att,
            _ => {
                return Err(ConversionError::InvalidEdgeWeightType2D(
                    self.edge_weight_type,
                ));
            }
        };

        // construct adjacency matrix by calculating the distance
        let mut matrix = vec![vec![0; self.dimension]; self.dimension];
        for i in 0..node_coord_section.len() {
            // check for cancellation every 100 iterations to avoid excessive overhead while still allowing for responsive cancellation
            if i % 100 == 0 && ctx.is_cancelled() {
                return Err(ConversionError::Cancelled);
            }

            for j in i..node_coord_section.len() {
                let node_i = node_coord_section[i];
                let node_j = node_coord_section[j];

                let distance = distance_fn(node_i, node_j);
                matrix[i][j] = distance;
                matrix[j][i] = distance;
            }
        }

        Ok(matrix)
    }

    /// Calculates the adjacency matrix of edge weights for the TSP instance based on the EDGE_WEIGHT_SECTION data.
    ///
    /// # Arguments
    /// * `ctx` - The execution context, which can be used to check for cancellation during the calculation of the adjacency matrix.
    ///   If the execution is cancelled, the function will return a `ConversionError::Cancelled` error.
    ///
    /// # Returns
    /// * `Result<Vec<Vec<f64>>, ConversionError>` - A result containing the adjacency matrix if successful,
    ///   or a `ConversionError` if the required data section is missing or if the edge weight format is unsupported for the given edge weight type.
    pub fn try_calculate_adjacency_matrix_edge_weights(
        &self,
        ctx: ExecutionContext<'_>,
    ) -> Result<Vec<Vec<i32>>, ConversionError> {
        // check for EDGE_WEIGHT_SECTION
        let edge_weight_section = self.try_get_edge_weight_section()?;

        match self.edge_weight_format {
            // Weights are given by a full matrix
            Some(EdgeWeightFormat::FullMatrix) => {
                self.try_get_adjacency_matrix_full_matrix(&edge_weight_section, ctx)
            }

            // Upper triangular matrix (row-wise without diagonal entries)
            Some(EdgeWeightFormat::UpperRow) => {
                self.try_get_adjacency_matrix_upper_row(&edge_weight_section, ctx)
            }

            // Lower triangular matrix (row-wise without diagonal entries)
            Some(EdgeWeightFormat::LowerRow) => {
                self.try_get_adjacency_matrix_lower_row(&edge_weight_section, ctx)
            }

            // Upper triangular matrix (row-wise including diagonal entries)
            Some(EdgeWeightFormat::UpperDiagRow) => {
                self.try_get_adjacency_matrix_upper_diag_row(&edge_weight_section, ctx)
            }

            // Lower triangular matrix (row-wise including diagonal entries)
            Some(EdgeWeightFormat::LowerDiagRow) => {
                self.try_get_adjacency_matrix_lower_diag_row(&edge_weight_section, ctx)
            }

            // Upper triangular matrix (column-wise without diagonal entries)
            Some(EdgeWeightFormat::UpperCol) => {
                self.try_get_adjacenty_matrix_upper_col(&edge_weight_section, ctx)
            }

            // Lower triangular matrix (column-wise without diagonal entries)
            Some(EdgeWeightFormat::LowerCol) => {
                self.try_get_adjacency_matrix_lower_col(&edge_weight_section, ctx)
            }

            // Upper triangular matrix (column-wise including diagonal entries)
            Some(EdgeWeightFormat::UpperDiagCol) => {
                self.try_get_adjacency_matrix_upper_diag_col(&edge_weight_section, ctx)
            }

            // Lower triangular matrix (column-wise including diagonal entries)
            Some(EdgeWeightFormat::LowerDiagCol) => {
                self.try_get_adjacency_matrix_lower_diag_col(&edge_weight_section, ctx)
            }

            _ => Err(ConversionError::UnsupportedEdgeWeightFormat(
                self.edge_weight_format,
                self.edge_weight_type,
            )),
        }
    }

    /// Extracts the nodes from the NODE_COORD_SECTION of the TSP instance and returns them as a vector of `Node` structs.
    /// This function assumes that the NODE_COORD_SECTION contains 2D coordinates (i.e., x and y) and that the z-coordinate is not provided (or is set to None).
    ///
    /// # Returns
    /// * `Result<Vec<Node>, ConversionError>` - A result containing the vector of `Node` structs if successful,
    ///   or a `ConversionError` if the required NODE_COORD_SECTION is missing in the instance data.
    fn try_extract_nodes_from_node_coord_section_2d(&self) -> Result<Vec<Node>, ConversionError> {
        // check for NODE_COORD_SECTION
        let node_coord_section = self.try_get_node_coord_section_2d()?;

        let nodes = node_coord_section.iter().map(|(id, x, y)| Node {
            id: *id,
            x: *x,
            y: *y,
            z: None,
        });
        Ok(nodes.collect())
    }

    /// Extracts the nodes from the EDGE_WEIGHT_SECTION of the TSP instance. If the instance contains a DISPLAY_DATA_SECTION,
    /// the nodes are extracted from there. Otherwise, the nodes are distributed on a circle.
    ///
    /// # Returns
    /// * `Result<Vec<Node>, ConversionError>` - A result containing the vector of `Node` structs if successful,
    ///   or a `ConversionError` if the required EDGE_WEIGHT_SECTION is missing in the instance data,
    ///   if the length of DISPLAY_DATA_SECTION does not match the number of nodes in the instance data,
    ///   or if the edge weight type is unsupported for node extraction.
    fn try_extract_nodes_from_edge_weight_section(&self) -> Result<Vec<Node>, ConversionError> {
        // check if the instance contains DISPLAY_DATA_SECTION
        let display_data_section = self.data_sections.iter().find_map(|s| match s {
            DataSection::DisplayDataSection(section) => Some(section),
            _ => None,
        });

        match display_data_section {
            // DISPLAY_DATA_SECTION exists
            Some(section) => {
                // check if the length of DISPLAY_DATA_SECTION matches the number of nodes in the instance data
                if section.len() != self.dimension {
                    // if not return an error
                    return Err(ConversionError::InvalidDisplayDataSectionLength(
                        section.len(),
                        self.dimension,
                    ));
                }

                // else extract nodes from DISPLAY_DATA_SECTION
                Ok(section
                    .iter()
                    .map(|(id, x, y)| Node {
                        id: *id,
                        x: *x,
                        y: *y,
                        z: None,
                    })
                    .collect::<Vec<_>>())
            }

            // DISPLAY_DATA_SECTION does not exist, create nodes with coordinates distributed on a circle
            None => {
                let n = self.dimension;

                // set radius such that the average distance between nodes is around 1.0
                let radius = n as f64 / (2.0 * std::f64::consts::PI);
                let angle = (2.0 * std::f64::consts::PI) / (n as f64);

                // construct nodes with coordinates distributed on a circle
                // add radius to x and y coordinates to ensure that all coordinates are positive
                // then round coordinates to 2 decimal places
                let nodes = (0..n)
                    .map(|i| Node {
                        id: i + 1,
                        x: ((radius * (i as f64 * angle).cos() + radius) * 100.0).round() / 100.0,
                        y: ((radius * (i as f64 * angle).sin() + radius) * 100.0).round() / 100.0,
                        z: None,
                    })
                    .collect::<Vec<_>>();

                Ok(nodes)
            }
        }
    }

    /// Extracts the node coordinates from the NODE_COORD_SECTION of the TSP instance, assuming 2D coordinates,
    /// and returns them as a reference to a vector of tuples containing the node ID and its x and y coordinates.
    ///
    /// # Returns
    /// * `Result<&Vec<(usize, f64, f64)>, ConversionError>` - A result containing a reference to the vector of tuples with node coordinates if successful,
    ///   or a `ConversionError` if the required NODE_COORD_SECTION is missing in the instance data.
    fn try_get_node_coord_section_2d(&self) -> Result<&Vec<(usize, f64, f64)>, ConversionError> {
        self.data_sections
            .iter()
            .find_map(|s| match s {
                DataSection::NodeCoordSection2D(section) => Some(section),
                _ => None,
            })
            .ok_or(ConversionError::MissingNodeCoordSection(
                self.edge_weight_type,
            ))
    }

    /// Extracts the edge weights from the EDGE_WEIGHT_SECTION of the TSP instance and returns them as a reference to a vector of vectors of integers.
    ///
    /// # Returns
    /// * `Result<&Vec<Vec<i32>>, ConversionError>` - A result containing a reference to the vector of vectors of edge weights if successful,
    ///   or a `ConversionError` if the required EDGE_WEIGHT_SECTION is missing in the instance data.
    fn try_get_edge_weight_section(&self) -> Result<Vec<i32>, ConversionError> {
        self.data_sections
            .iter()
            .find_map(|s| match s {
                DataSection::EdgeWeightSection(section) => Some(
                    section
                        .iter()
                        .flat_map(|v| v.iter().copied())
                        .collect::<Vec<_>>(),
                ),
                _ => None,
            })
            .ok_or(ConversionError::MissingEdgeWeightSection(
                self.edge_weight_type,
            ))
    }

    /// Constructs the adjacency matrix from the EDGE_WEIGHT_SECTION when the edge weights are given by a full matrix.
    ///
    /// # Arguments
    /// * `edge_weight_section` - A slice containing the full adjacency matrix.
    /// * `ctx` - The execution context, which can be used to check for cancellation during the construction of the adjacency matrix.
    ///
    /// # Returns
    /// * `Vec<Vec<i32>>` - The adjacency matrix representing the graph.
    fn try_get_adjacency_matrix_full_matrix(
        &self,
        edge_weight_section: &[i32],
        ctx: ExecutionContext,
    ) -> Result<Vec<Vec<i32>>, ConversionError> {
        let mut matrix = vec![vec![0; self.dimension]; self.dimension];

        for i in 0..self.dimension {
            // check for cancellation every 100 iterations to avoid excessive overhead while still allowing for responsive cancellation
            if i % 100 == 0 && ctx.is_cancelled() {
                return Err(ConversionError::Cancelled);
            }

            for j in 0..self.dimension {
                matrix[i][j] = edge_weight_section[i * self.dimension + j];
            }
        }
        Ok(matrix)
    }

    /// Constructs the adjacency matrix from the EDGE_WEIGHT_SECTION when the edge weights are given by an upper
    /// triangular matrix (row-wise without diagonal entries).
    ///
    /// # Arguments
    /// * `edge_weight_section` - A slice containing the upper triangular part of the adjacency matrix
    ///   (without diagonal entries) with row major ordering.
    /// * `ctx` - The execution context, which can be used to check for cancellation during the construction of the adjacency matrix.
    ///
    /// # Returns
    /// * `Vec<Vec<i32>>` - The adjacency matrix representing the graph.
    #[allow(clippy::needless_range_loop)]
    fn try_get_adjacency_matrix_upper_row(
        &self,
        edge_weight_section: &[i32],
        ctx: ExecutionContext,
    ) -> Result<Vec<Vec<i32>>, ConversionError> {
        let mut matrix = vec![vec![0; self.dimension]; self.dimension];

        for i in 0..self.dimension - 1 {
            // check for cancellation every 100 iterations to avoid excessive overhead while still allowing for responsive cancellation
            if i % 100 == 0 && ctx.is_cancelled() {
                return Err(ConversionError::Cancelled);
            }

            for j in i + 1..self.dimension {
                let idx = (i * self.dimension + j) - (i + 1) * (i + 2) / 2;
                matrix[i][j] = edge_weight_section[idx];
                matrix[j][i] = edge_weight_section[idx];
            }
        }

        Ok(matrix)
    }

    /// Constructs the adjacency matrix from the EDGE_WEIGHT_SECTION when the edge weights are given by a lower
    /// triangular matrix (row-wise without diagonal entries).
    ///
    /// # Arguments
    /// * `edge_weight_section` - A slice containing the lower triangular part of the adjacency matrix
    ///   (without diagonal entries) with row major ordering.
    /// * `ctx` - The execution context, which can be used to check for cancellation during the construction of the adjacency matrix.
    /// # Returns
    /// * `Vec<Vec<i32>>` - The adjacency matrix representing the graph.
    #[allow(clippy::needless_range_loop)]
    fn try_get_adjacency_matrix_lower_row(
        &self,
        edge_weight_section: &[i32],
        ctx: ExecutionContext,
    ) -> Result<Vec<Vec<i32>>, ConversionError> {
        let mut matrix = vec![vec![0; self.dimension]; self.dimension];

        for i in 1..self.dimension {
            // check for cancellation every 100 iterations to avoid excessive overhead while still allowing for responsive cancellation
            if i % 100 == 0 && ctx.is_cancelled() {
                return Err(ConversionError::Cancelled);
            }

            for j in 0..i {
                let idx = (i * (i - 1)) / 2 + j;
                matrix[i][j] = edge_weight_section[idx];
                matrix[j][i] = edge_weight_section[idx];
            }
        }

        Ok(matrix)
    }

    /// Constructs the adjacency matrix from the EDGE_WEIGHT_SECTION when the edge weights are given by an upper
    /// triangular matrix (row-wise including diagonal entries).
    ///
    /// # Arguments
    /// * `edge_weight_section` - A slice containing the upper triangular part of the adjacency matrix
    ///   (including diagonal entries) with row major ordering.
    /// * `ctx` - The execution context, which can be used to check for cancellation during the construction of the adjacency matrix.
    ///
    /// # Returns
    /// * `Vec<Vec<i32>>` - The adjacency matrix representing the graph.
    #[allow(clippy::needless_range_loop)]
    fn try_get_adjacency_matrix_upper_diag_row(
        &self,
        edge_weight_section: &[i32],
        ctx: ExecutionContext,
    ) -> Result<Vec<Vec<i32>>, ConversionError> {
        let mut matrix = vec![vec![0; self.dimension]; self.dimension];

        for i in 0..self.dimension {
            // check for cancellation every 100 iterations to avoid excessive overhead while still allowing for responsive cancellation
            if i % 100 == 0 && ctx.is_cancelled() {
                return Err(ConversionError::Cancelled);
            }

            for j in i..self.dimension {
                let idx = (i * self.dimension + j) - (i * (i + 1)) / 2;
                matrix[i][j] = edge_weight_section[idx];
                matrix[j][i] = edge_weight_section[idx];
            }
        }
        Ok(matrix)
    }

    /// Constructs the adjacency matrix from the EDGE_WEIGHT_SECTION when the edge weights are given by a lower
    /// triangular matrix (row-wise including diagonal entries).
    ///
    /// # Arguments
    /// * `edge_weight_section` - A slice containing the lower triangular part of the adjacency matrix
    ///   (including diagonal entries) with row major ordering.
    /// * `ctx` - The execution context, which can be used to check for cancellation during the construction of the adjacency matrix.
    ///
    /// # Returns
    /// * `Vec<Vec<i32>>` - The adjacency matrix representing the graph.
    #[allow(clippy::needless_range_loop)]
    fn try_get_adjacency_matrix_lower_diag_row(
        &self,
        edge_weight_section: &[i32],
        ctx: ExecutionContext,
    ) -> Result<Vec<Vec<i32>>, ConversionError> {
        let mut matrix = vec![vec![0; self.dimension]; self.dimension];

        for i in 0..self.dimension {
            // check for cancellation every 100 iterations to avoid excessive overhead while still allowing for responsive cancellation
            if i % 100 == 0 && ctx.is_cancelled() {
                return Err(ConversionError::Cancelled);
            }

            for j in 0..=i {
                let idx = (i * (i + 1)) / 2 + j;
                matrix[i][j] = edge_weight_section[idx];
                matrix[j][i] = edge_weight_section[idx];
            }
        }

        Ok(matrix)
    }

    /// Constructs the adjacency matrix from the EDGE_WEIGHT_SECTION when the edge weights are given by an upper
    /// triangular matrix (column-wise without diagonal entries).
    ///
    /// # Arguments
    /// * `edge_weight_section` - A slice containing the upper triangular part of the adjacency matrix
    ///   (without diagonal entries) with column major ordering.
    /// * `ctx` - The execution context, which can be used to check for cancellation during the construction of the adjacency matrix.
    ///
    /// # Returns
    /// * `Vec<Vec<i32>>` - The adjacency matrix representing the graph.
    #[allow(clippy::needless_range_loop)]
    fn try_get_adjacenty_matrix_upper_col(
        &self,
        edge_weight_section: &[i32],
        ctx: ExecutionContext,
    ) -> Result<Vec<Vec<i32>>, ConversionError> {
        let mut matrix = vec![vec![0; self.dimension]; self.dimension];

        for j in 1..self.dimension {
            // check for cancellation every 100 iterations to avoid excessive overhead while still allowing for responsive cancellation
            if j % 100 == 0 && ctx.is_cancelled() {
                return Err(ConversionError::Cancelled);
            }

            for i in 0..j {
                let idx = (j * (j - 1)) / 2 + i;
                matrix[i][j] = edge_weight_section[idx];
                matrix[j][i] = edge_weight_section[idx];
            }
        }

        Ok(matrix)
    }

    /// Constructs the adjacency matrix from the EDGE_WEIGHT_SECTION when the edge weights are given by a lower
    /// triangular matrix (column-wise without diagonal entries).
    ///
    /// # Arguments
    /// * `edge_weight_section` - A slice containing the lower triangular part of the adjacency matrix
    ///   (without diagonal entries) with column major ordering.
    /// * `ctx` - The execution context, which can be used to check for cancellation during the construction of the adjacency matrix.
    ///
    /// # Returns
    /// * `Vec<Vec<i32>>` - The adjacency matrix representing the graph.
    #[allow(clippy::needless_range_loop)]
    fn try_get_adjacency_matrix_lower_col(
        &self,
        edge_weight_section: &[i32],
        ctx: ExecutionContext,
    ) -> Result<Vec<Vec<i32>>, ConversionError> {
        let mut matrix = vec![vec![0; self.dimension]; self.dimension];

        for j in 0..self.dimension - 1 {
            // check for cancellation every 100 iterations to avoid excessive overhead while still allowing for responsive cancellation
            if j % 100 == 0 && ctx.is_cancelled() {
                return Err(ConversionError::Cancelled);
            }

            for i in j + 1..self.dimension {
                let idx = j * (2 * self.dimension - j - 1) / 2 + (i - j - 1);
                matrix[i][j] = edge_weight_section[idx];
                matrix[j][i] = edge_weight_section[idx];
            }
        }

        Ok(matrix)
    }

    /// Constructs the adjacency matrix from the EDGE_WEIGHT_SECTION when the edge weights are given by an upper
    /// triangular matrix (column-wise including diagonal entries).
    ///
    /// # Arguments
    /// * `edge_weight_section` - A slice containing the upper triangular part of the adjacency matrix
    ///   (including diagonal entries) with column major ordering.
    /// * `ctx` - The execution context, which can be used to check for cancellation during the construction of the adjacency matrix.
    ///
    /// # Returns
    /// * `Vec<Vec<i32>>` - The adjacency matrix representing the graph.
    #[allow(clippy::needless_range_loop)]
    fn try_get_adjacency_matrix_upper_diag_col(
        &self,
        edge_weight_section: &[i32],
        ctx: ExecutionContext,
    ) -> Result<Vec<Vec<i32>>, ConversionError> {
        let mut matrix = vec![vec![0; self.dimension]; self.dimension];

        for j in 0..self.dimension {
            // check for cancellation every 100 iterations to avoid excessive overhead while still allowing for responsive cancellation
            if j % 100 == 0 && ctx.is_cancelled() {
                return Err(ConversionError::Cancelled);
            }

            for i in 0..=j {
                let idx = (j * (j + 1)) / 2 + i;
                matrix[i][j] = edge_weight_section[idx];
                matrix[j][i] = edge_weight_section[idx];
            }
        }

        Ok(matrix)
    }

    /// Constructs the adjacency matrix from the EDGE_WEIGHT_SECTION when the edge weights are given by a lower
    /// triangular matrix (column-wise including diagonal entries).
    ///
    /// # Arguments
    /// * `edge_weight_section` - A slice containing the lower triangular part of the adjacency matrix
    ///   (including diagonal entries) with column major ordering.
    /// * `ctx` - The execution context, which can be used to check for cancellation during the construction of the adjacency matrix.
    ///
    /// # Returns
    /// * `Vec<Vec<i32>>` - The adjacency matrix representing the graph.
    #[allow(clippy::needless_range_loop)]
    fn try_get_adjacency_matrix_lower_diag_col(
        &self,
        edge_weight_section: &[i32],
        ctx: ExecutionContext,
    ) -> Result<Vec<Vec<i32>>, ConversionError> {
        let mut matrix = vec![vec![0; self.dimension]; self.dimension];

        for j in 0..self.dimension {
            // check for cancellation every 100 iterations to avoid excessive overhead while still allowing for responsive cancellation
            if j % 100 == 0 && ctx.is_cancelled() {
                return Err(ConversionError::Cancelled);
            }

            for i in j..self.dimension {
                let idx = j * (2 * self.dimension - j + 1) / 2 + (i - j);
                matrix[i][j] = edge_weight_section[idx];
                matrix[j][i] = edge_weight_section[idx];
            }
        }

        Ok(matrix)
    }
}
