//! Module containing the specific error types that can occur during parsing, using the `thiserror` crate for convenient error definitions and formatting.
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Invalid line format: {0}")]
    InvalidLineFormat(String),
    #[error("Empty line in header is not allowed")]
    EmptyLineInHeader,
    #[error("Unknown header field: {0}")]
    UnknownHeaderField(String),
    #[error("Unknown problem type: {0}")]
    UnknownProblemType(String),
    #[error("Unknown edge weight type: {0}")]
    UnknownEdgeWeightType(String),
    #[error("Unknown edge weight format: {0}")]
    UnknownEdgeWeightFormat(String),
    #[error("Unknown edge data format: {0}")]
    UnknownEdgeDataFormat(String),
    #[error("Unknown node coord type: {0}")]
    UnknownNodeCoordType(String),
    #[error("Unknown display data type: {0}")]
    UnknownDisplayDataType(String),
    #[error("Unknown section type: {0}")]
    UnknownSectionType(String),
    #[error("Invalid dimension value: {0}")]
    InvalidDimensionValue(String),
    #[error("Invalid capacity value: {0}")]
    InvalidCapacityValue(String),
    #[error("Invalid node index value: {0}")]
    InvalidNodeIndexValue(String),
    #[error("Invalid coordinate value: {0}")]
    InvalidCoordinateValue(String),
    #[error("Invalid line format in NODE_COORD_SECTION: {0}")]
    InvalidNodeCoordLineFormat(String),
    #[error("Invalid line format in FIXED_EDGES_SECTION: {0}")]
    InvalidFixedEdgesLineFormat(String),
    #[error("Invalid line format in DISPLAY_DATA_SECTION: {0}")]
    InvalidDisplayDataLineFormat(String),
    #[error("Invalid line format in EDGE_WEIGHT_SECTION: {0}")]
    InvalidEdgeWeightLineFormat(String),
    #[error("Missing required field: {0}")]
    MissingRequiredField(String),
    #[error("Unimplemented section type: {0}")]
    UnimplementedSectionType(String),
}
