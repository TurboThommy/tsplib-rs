//! This module defines the DataSectionType enum, which specifies the type of data section in the problem instance file.
use std::fmt;

/// DataSectionType specifies the type of data section in the problem instance file.
pub enum DataSectionType {
    /// NODE_COORD_SECTION
    NodeCoordSection,

    /// FIXED_EDGES_SECTION
    FixedEdgesSection,

    /// DISPLAY_DATA_SECTION
    DisplayDataSection,

    /// EDGE_WEIGHT_SECTION
    EdgeWeightSection,

    /// TOUR_SECTION, no examples given in the tsplib repo
    TourSection,

    /// DEPOT_SECTION, no examples given in the tsplib repo
    DepotSection,

    /// DEMAND_SECTION, no examples given in the tsplib repo
    DemandSection,

    /// EDGE_DATA_SECTION, no examples given in the tsplib repo
    EdgeDataSection,
}

impl fmt::Display for DataSectionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            DataSectionType::NodeCoordSection => "NODE_COORD_SECTION",
            DataSectionType::FixedEdgesSection => "FIXED_EDGES_SECTION",
            DataSectionType::DisplayDataSection => "DISPLAY_DATA_SECTION",
            DataSectionType::EdgeWeightSection => "EDGE_WEIGHT_SECTION",
            DataSectionType::TourSection => "TOUR_SECTION",
            DataSectionType::DepotSection => "DEPOT_SECTION",
            DataSectionType::DemandSection => "DEMAND_SECTION",
            DataSectionType::EdgeDataSection => "EDGE_DATA_SECTION",
        };
        write!(f, "{}", s)
    }
}
