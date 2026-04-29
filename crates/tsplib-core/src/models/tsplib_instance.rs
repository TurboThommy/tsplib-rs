use crate::enums::{
    DataSection, DisplayDataType, EdgeDataFormat, EdgeWeightFormat, EdgeWeightType, NodeCoordType,
    ProblemType,
};

use std::fmt;

/// A struct representing a TSP instance, containing all required and optional fields, as well as the data sections.
/// This struct can be used to represent any TSP instance defined in the TSPLIB format.
#[derive(Debug)]
pub struct TSPLIBInstance {
    // required
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
