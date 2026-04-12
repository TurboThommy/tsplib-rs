use crate::enums::{
    DataSection, DisplayDataType, EdgeDataFormat, EdgeWeightFormat, EdgeWeightType, NodeCoordType,
    ProblemType,
};
use std::fmt;

#[derive(Debug)]
pub struct TSPInstance {
    // required
    pub name: String,
    pub problem_type: ProblemType,
    pub dimension: usize,
    pub edge_weight_type: EdgeWeightType,

    // optional
    pub comment: Option<Vec<String>>,
    pub capacity: Option<usize>,
    pub edge_weight_format: Option<EdgeWeightFormat>,
    pub edge_data_format: Option<EdgeDataFormat>,
    pub node_coord_type: Option<NodeCoordType>,
    pub display_data_type: Option<DisplayDataType>,

    // data sections
    pub data_sections: Vec<DataSection>,
}

impl fmt::Display for TSPInstance {
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
