//! This module defines the ConversionError enum, which represents errors that can occur during the conversion between different model types.
use thiserror::Error;

use crate::enums::{EdgeWeightFormat, EdgeWeightType};

/// InstanceError represents errors that can occur during the validation of problem instances.
#[derive(Error, Debug)]
pub enum InstanceError {
    #[error(
        "One of the node IDs is not between 1 and the dimension of the problem instance. Found {0} and {1}, expected IDs between 1 and {2}"
    )]
    DistanceInvalidNodeId(usize, usize, usize),
}

#[derive(Error, Debug)]
pub enum IoError {
    #[error("Invalid file path: {0}")]
    InvalidFilePath(String),

    #[error("Unable to read file: {0}")]
    FileReadError(String),

    #[error("File does not contain a valid file stem: {0}")]
    InvalidFileStem(String),

    #[error("Unable to read directory: {0}")]
    DirectoryReadError(String),

    #[error("Unable to read directory entry: {0}")]
    DirectoryEntryReadError(String),
}

/// ConversionError represents errors that can occur during the conversion between different model types.
#[derive(Error, Debug)]
pub enum ConversionError {
    /// Required NODE_COORD_SECTION is missing for the specified EDGE_WEIGHT_TYPE
    #[error(
        "EDGE_WEIGHT_TYPE {0} requires NODE_COORD_SECTION, but it is missing in the instance data"
    )]
    MissingNodeCoordSection(EdgeWeightType),

    /// Required EDGE_WEIGHT_SECTION is missing for the specified EDGE_WEIGHT_TYPE
    #[error(
        "EDGE_WEIGHT_TYPE {0} requires EDGE_WEIGHT_SECTION, but it is missing in the instance data"
    )]
    MissingEdgeWeightSection(EdgeWeightType),

    /// Unsupported EDGE_WEIGHT_FORMAT for the specified EDGE_WEIGHT_TYPE
    #[error("Unsupported EDGE_WEIGHT_FORMAT {0:?} for EDGE_WEIGHT_TYPE {1}")]
    UnsupportedEdgeWeightFormat(Option<EdgeWeightFormat>, EdgeWeightType),

    /// Length of DISPLAY_DATA_SECTION does not match the number of nodes in the instance data
    #[error(
        "Length of DISPLAY_DATA_SECTION does not match the number of nodes in the instance data. Found {0}, expected {1}"
    )]
    InvalidDisplayDataSectionLength(usize, usize),

    /// Unsupported EDGE_WEIGHT_TYPE encountered.
    #[error("Unsupported EDGE_WEIGHT_TYPE {0:?}")]
    UnsupportedEdgeWeightType(EdgeWeightType),

    /// Invalid EDGE_WEIGHT_TYPE for 2D coordinates encountered.
    #[error(
        "Invalid EDGE_WEIGHT_TYPE {0:?} for 2D coordinates. Expected EUC_2D, MAX_2D, MAN_2D, CEIL_2D, GEO or ATT"
    )]
    InvalidEdgeWeightType2D(EdgeWeightType),

    #[error("The execution was cancelled before the algorithm could complete.")]
    Cancelled,

    #[error("The adjacency matrix is empty")]
    EmptyAdjacencyMatrix,

    #[error("Invalid start node {0}. Valid node IDs are between 1 and {1}")]
    InvalidStartNode(usize, usize),

    #[error("Prim's algorithm failed: {0}")]
    PrimMstError(String),
}
