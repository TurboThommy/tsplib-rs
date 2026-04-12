use std::fmt;

#[derive(Debug)]
pub enum EdgeWeightType {
    // EXPLICIT, Weights are listed explicitly in the corresponding section
    Explicit,

    // EUC_2D, Weights are Euclidean distances in 2D
    Euc2D,

    // EUC_3D, Weights are Euclidean distances in 3D
    Euc3D,

    // MAX_2D, Weights are maximum distances in 2D
    Max2D,

    // MAX_3D, Weights are maximum distances in 3D
    Max3D,

    // MAN_2D, Weights are Manhattan distances in 2D
    Man2D,

    // MAN_3D, Weights are Manhattan distances in 3D
    Man3D,

    // CEIL_2D, Weights are Euclidean distances in 2D rounded up
    Ceil2D,

    // GEO, Weights are geographical distances
    Geo,

    // ATT, Special distance function for problems att48 and att532
    Att,

    // XRAY1, Special distance function for crystallography problems (Version 1)
    Xray1,

    // XRAY2, Special distance function for crystallography problems (Version 2)
    Xray2,

    // SPECIAL, There is a special distance function documented elsewhere
    Special,
}

impl fmt::Display for EdgeWeightType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            EdgeWeightType::Explicit => "EXPLICIT",
            EdgeWeightType::Euc2D => "EUC_2D",
            EdgeWeightType::Euc3D => "EUC_3D",
            EdgeWeightType::Max2D => "MAX_2D",
            EdgeWeightType::Max3D => "MAX_3D",
            EdgeWeightType::Man2D => "MAN_2D",
            EdgeWeightType::Man3D => "MAN_3D",
            EdgeWeightType::Ceil2D => "CEIL_2D",
            EdgeWeightType::Geo => "GEO",
            EdgeWeightType::Att => "ATT",
            EdgeWeightType::Xray1 => "XRAY1",
            EdgeWeightType::Xray2 => "XRAY2",
            EdgeWeightType::Special => "SPECIAL",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug)]
pub enum ProblemType {
    // Symmetric TSP
    // distance between i and j is the same as between j and i
    TSP,

    // Asymmetric TSP
    // distance from i to j may differ from distance from j to i
    ATSP,

    // Sequential Ordering Problem
    // ATSP with precedence constraints, where certain vertices must be visited before others
    SOP,

    // Hammilton Cycle Problem
    // Test if the graph contains a hammilton cycle (a cycle that visits each vertex exactly once)
    HCP,

    // Capacitated Vehicle Routing Problem
    // TSP with multiple vehicles and capacity constraints
    CVRP,

    // Collectison of tours
    // TBD
    TOUR,
}

impl fmt::Display for ProblemType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ProblemType::TSP => "TSP",
            ProblemType::ATSP => "ATSP",
            ProblemType::SOP => "SOP",
            ProblemType::HCP => "HCP",
            ProblemType::CVRP => "CVRP",
            ProblemType::TOUR => "TOUR",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug)]
pub enum EdgeWeightFormat {
    // FUNCTION, Weights are given by a function (see EdgeWeightType)
    Function,

    // FULL_MATRIX, Weights are given by a full matrix
    FullMatrix,

    // UPPER_ROW, Upper triangular matrix (row-wise without diagonal entries)
    UpperRow,

    // LOWER_ROW, Lower triangular matrix (row-wise without diagonal entries)
    LowerRow,

    // UPPER_DIAG_ROW, Upper triangular matrix (row-wise including diagonal entries)
    UpperDiagRow,

    // LOWER_DIAG_ROW, Lower triangular matrix (row-wise including diagonal entries)
    LowerDiagRow,

    // UPPER_COL, Upper triangular matrix (column-wise without diagonal entries)
    UpperCol,

    // LOWER_COL, Lower triangular matrix (column-wise without diagonal entries)
    LowerCol,

    // UPPER_DIAG_COL, Upper triangular matrix (column-wise including diagonal entries)
    UpperDiagCol,

    // LOWER_DIAG_COL, Lower triangular matrix (column-wise including diagonal entries)
    LowerDiagCol,
}

impl fmt::Display for EdgeWeightFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            EdgeWeightFormat::Function => "FUNCTION",
            EdgeWeightFormat::FullMatrix => "FULL_MATRIX",
            EdgeWeightFormat::UpperRow => "UPPER_ROW",
            EdgeWeightFormat::LowerRow => "LOWER_ROW",
            EdgeWeightFormat::UpperDiagRow => "UPPER_DIAG_ROW",
            EdgeWeightFormat::LowerDiagRow => "LOWER_DIAG_ROW",
            EdgeWeightFormat::UpperCol => "UPPER_COL",
            EdgeWeightFormat::LowerCol => "LOWER_COL",
            EdgeWeightFormat::UpperDiagCol => "UPPER_DIAG_COL",
            EdgeWeightFormat::LowerDiagCol => "LOWER_DIAG_COL",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug)]
pub enum EdgeDataFormat {
    // EDGE_LIST, The graph is given by an edge list
    EdgeList,

    // ADJ_LIST, The graph is given as an adjacency list
    AdjList,
}

impl fmt::Display for EdgeDataFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            EdgeDataFormat::EdgeList => "EDGE_LIST",
            EdgeDataFormat::AdjList => "ADJ_LIST",
        };
        write!(f, "{}", s)
    }
}

#[derive(Clone, Debug)]
pub enum NodeCoordType {
    // TWOD_COORDS, Nodes are specified by coordinates in 2D
    TwoDCoords,

    // THREED_COORDS, Nodes are specified by coordinates in 3D
    ThreeDCoords,

    // NO_COORDS, The nodes do not have associated coordinates (this is the default value)
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

#[derive(Debug)]
pub enum DisplayDataType {
    // COORDS_DISPLAY, Display is generated from the node coordinates (default value if node coordinates are specified)
    CoordDisplay,

    // TWOD_DISPLAY, Explicit coordinates in 2D are given
    TwoDDisplay,

    // NO_DISPLAY, No graphical display is possible (default value if node coordinates are not specified)
    NoDisplay,
}

impl fmt::Display for DisplayDataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            DisplayDataType::CoordDisplay => "COORDS_DISPLAY",
            DisplayDataType::TwoDDisplay => "TWOD_DISPLAY",
            DisplayDataType::NoDisplay => "NO_DISPLAY",
        };
        write!(f, "{}", s)
    }
}

pub enum DataSectionType {
    // NODE_COORD_SECTION
    NodeCoordSection,

    // FIXED_EDGES_SECTION,
    FixedEdgesSection,

    // DISPLAY_DATA_SECTION,
    DisplayDataSection,

    // EDGE_WEIGHT_SECTION,
    EdgeWeightSection,

    // TOUR_SECTION, no examples given in the tsplib repo
    TourSection,

    // DEPOT_SECTION, no examples given in the tsplib repo
    DepotSection,

    // DEMAND_SECTION, no examples given in the tsplib repo
    DemandSection,

    // EDGE_DATA_SECTION, no examples given in the tsplib repo
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

#[derive(Debug)]
pub enum DataSection {
    // NODE_COORD_SECTION && NodeCoordType = TWOD_COORDS,
    NodeCoordSection2D(Vec<(usize, f64, f64)>),

    // NODE_COORD_SECTION && NodeCoordType = THREED_COORDS,
    NodeCoordSection3D(Vec<(usize, f64, f64, f64)>),

    // FIXED_EDGES_SECTION,
    FixedEdgesSection(Vec<(usize, usize)>),

    // DISPLAY_DATA_SECTION,
    DisplayDataSection(Vec<(usize, f64, f64)>),

    // EDGE_WEIGHT_SECTION,
    EdgeWeightSection(Vec<Vec<f64>>),

    // TOUR_SECTION, no examples given in the tsplib repo
    TourSection,

    // DEPOT_SECTION, no examples given in the tsplib repo
    DepotSection,

    // DEMAND_SECTION, no examples given in the tsplib repo
    DemandSection,

    // EDGE_DATA_SECTION, no examples given in the tsplib repo
    EdgeDataSection,
}

// TODO: add termination sequences for sections that require them (e.g. -1)
impl fmt::Display for DataSection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            DataSection::NodeCoordSection2D(coords) => format!(
                "NODE_COORD_SECTION\n{}",
                coords
                    .iter()
                    .map(|(id, x, y)| format!("{} {} {}", id, x, y))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            DataSection::NodeCoordSection3D(coords) => format!(
                "NODE_COORD_SECTION\n{}",
                coords
                    .iter()
                    .map(|(id, x, y, z)| format!("{} {} {} {}", id, x, y, z))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            DataSection::FixedEdgesSection(edges) => format!(
                "FIXED_EDGES_SECTION\n{}",
                edges
                    .iter()
                    .map(|(from, to)| format!("{} {}", from, to))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            DataSection::DisplayDataSection(display_data) => format!(
                "DISPLAY_DATA_SECTION\n{}",
                display_data
                    .iter()
                    .map(|(id, x, y)| format!("{} {} {}", id, x, y))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            DataSection::EdgeWeightSection(weights) => format!(
                "EDGE_WEIGHT_SECTION\n{}",
                weights
                    .iter()
                    .map(|row| row
                        .iter()
                        .map(|w| w.to_string())
                        .collect::<Vec<_>>()
                        .join(" "))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            DataSection::TourSection => "TOUR_SECTION".to_string(),
            DataSection::DepotSection => "DEPOT_SECTION".to_string(),
            DataSection::DemandSection => "DEMAND_SECTION".to_string(),
            DataSection::EdgeDataSection => "EDGE_DATA_SECTION".to_string(),
        };
        write!(f, "{}", s)
    }
}
