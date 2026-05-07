//! Module containing the specific error types that can occur during parsing, using the `thiserror` crate for convenient error definitions and formatting.
use thiserror::Error;

/// Defines the `ParseError` enum, which represents various errors that can occur during the parsing of TSPLIB files.
/// Each variant includes a descriptive error message and relevant data to help identify the issue.
#[derive(Error, Debug)]
pub enum ParseError {
    /// Represents an error that occurs when the file format is invalid.
    #[error("Invalid line format: {0}")]
    InvalidLineFormat(String),

    /// Represents an error that occurs when a required header field is missing.
    #[error("Empty line in header is not allowed")]
    EmptyLineInHeader,

    /// Represents an error that occurs when an unknown header field is encountered.
    #[error("Unknown header field: {0}")]
    UnknownHeaderField(String),

    /// Represents an error that occurs when an unknown problem type is encountered.
    #[error("Unknown problem type: {0}")]
    UnknownProblemType(String),

    /// Represents an error that occurs when an unknown edge weight type is encountered.
    #[error("Unknown edge weight type: {0}")]
    UnknownEdgeWeightType(String),

    /// Represents an error that occurs when an unknown edge weight format is encountered.
    #[error("Unknown edge weight format: {0}")]
    UnknownEdgeWeightFormat(String),

    /// Represents an error that occurs when an unknown edge data format is encountered.
    #[error("Unknown edge data format: {0}")]
    UnknownEdgeDataFormat(String),

    /// Represents an error that occurs when an unknown node coord type is encountered.
    #[error("Unknown node coord type: {0}")]
    UnknownNodeCoordType(String),

    /// Represents an error that occurs when an unknown display data type is encountered.
    #[error("Unknown display data type: {0}")]
    UnknownDisplayDataType(String),

    /// Represents an error that occurs when an unknown section type is encountered.
    #[error("Unknown section type: {0}")]
    UnknownSectionType(String),

    /// Represents an error that occurs when an invalid dimension value is encountered.
    #[error("Invalid dimension value: {0}")]
    InvalidDimensionValue(String),

    /// Represents an error that occurs when an invalid capacity value is encountered.
    #[error("Invalid capacity value: {0}")]
    InvalidCapacityValue(String),

    /// Represents an error that occurs when an invalid demand value is encountered.
    #[error("Invalid node index value: {0}")]
    InvalidNodeIndexValue(String),

    /// Represents an error that occurs when an invalid demand value is encountered.
    #[error("Invalid coordinate value: {0}")]
    InvalidCoordinateValue(String),

    /// Represents an error that occurs when an invalid line format is encountered in the NODE_COORD_SECTION.
    #[error("Invalid line format in NODE_COORD_SECTION: {0}")]
    InvalidNodeCoordLineFormat(String),

    /// Represents an error that occurs when an invalid line format is encountered in the FIXED_EDGES_SECTION.
    #[error("Invalid line format in FIXED_EDGES_SECTION: {0}")]
    InvalidFixedEdgesLineFormat(String),

    /// Represents an error that occurs when an invalid line format is encountered in the DISPLAY_DATA_SECTION.
    #[error("Invalid line format in DISPLAY_DATA_SECTION: {0}")]
    InvalidDisplayDataLineFormat(String),

    /// Represents an error that occurs when an invalid line format is encountered in the EDGE_WEIGHT_SECTION.
    #[error("Invalid line format in EDGE_WEIGHT_SECTION: {0}")]
    InvalidEdgeWeightLineFormat(String),

    /// Represents an error that occurs when a required field is missing in the header.
    #[error("Missing required field: {0}")]
    MissingRequiredField(String),

    /// Represents an error that occurs when a required section is missing in the file.
    #[error("Unimplemented section type: {0}")]
    UnimplementedSectionType(String),
}
