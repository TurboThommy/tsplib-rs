//! This module defines the NodeCoordType enum, which specifies how the node coordinates are defined in the problem instance.
use std::fmt;

/// NodeCoordType specifies how the node coordinates are defined in the problem instance.
#[derive(Clone, Debug)]
pub enum NodeCoordType {
    /// TWOD_COORDS, Nodes are specified by coordinates in 2D
    TwoDCoords,

    /// THREED_COORDS, Nodes are specified by coordinates in 3D
    ThreeDCoords,

    /// NO_COORDS, The nodes do not have associated coordinates (this is the default value)
    NoCoords,
}

impl fmt::Display for NodeCoordType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            NodeCoordType::TwoDCoords => "TWOD_COORDS",
            NodeCoordType::ThreeDCoords => "THREED_COORDS",
            NodeCoordType::NoCoords => "NO_COORDS",
        };
        write!(f, "{}", s)
    }
}
