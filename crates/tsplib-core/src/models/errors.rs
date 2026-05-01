use thiserror::Error;

use crate::enums::{EdgeWeightFormat, EdgeWeightType};

#[derive(Error, Debug)]
pub enum ConversionError {
    #[error(
        "EDGE_WEIGHT_TYPE {0} requires NODE_COORD_SECTION, but it is missing in the instance data"
    )]
    MissingNodeCoordSection(EdgeWeightType),

    #[error(
        "EDGE_WEIGHT_TYPE {0} requires EDGE_WEIGHT_SECTION, but it is missing in the instance data"
    )]
    MissingEdgeWeightSection(EdgeWeightType),

    #[error("Unsupported EDGE_WEIGHT_FORMAT {0:?} for EDGE_WEIGHT_TYPE {1}")]
    UnsupportedEdgeWeightFormat(Option<EdgeWeightFormat>, EdgeWeightType),

    #[error(
        "Length of DISPLAY_DATA_SECTION does not match the number of nodes in the instance data. Found {0}, expected {1}"
    )]
    InvalidDisplayDataSectionLength(usize, usize),

    #[error("Unsupported EDGE_WEIGHT_TYPE {0:?}")]
    UnsupportedEdgeWeightType(EdgeWeightType),

    #[error(
        "Invalid EDGE_WEIGHT_TYPE {0:?} for 2D coordinates. Expected EUC_2D, MAX_2D, MAN_2D, CEIL_2D, GEO or ATT"
    )]
    InvalidEdgeWeightType2D(EdgeWeightType),
}
