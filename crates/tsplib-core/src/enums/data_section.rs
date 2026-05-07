//! This module defines the DataSection enum, which represents the different types of data sections that can be present in a problem instance file.
//! Each variant of the enum corresponds to a specific section type and contains the relevant data for that section.
use std::fmt;

/// DataSection represents the actual data for a given section in the problem instance file.
/// The variants of this enum correspond to the different types of sections that can be present in the file,
/// and each variant contains the relevant data for that section.
#[derive(Debug)]
pub enum DataSection {
    /// NODE_COORD_SECTION && NodeCoordType = TWOD_COORDS,
    NodeCoordSection2D(Vec<(usize, f64, f64)>),

    /// NODE_COORD_SECTION && NodeCoordType = THREED_COORDS,
    NodeCoordSection3D(Vec<(usize, f64, f64, f64)>),

    /// FIXED_EDGES_SECTION,
    FixedEdgesSection(Vec<(usize, usize)>),

    /// DISPLAY_DATA_SECTION,
    DisplayDataSection(Vec<(usize, f64, f64)>),

    /// EDGE_WEIGHT_SECTION,
    EdgeWeightSection(Vec<Vec<i32>>),

    /// TOUR_SECTION, no examples given in the tsplib repo
    TourSection,

    /// DEPOT_SECTION, no examples given in the tsplib repo
    DepotSection,

    /// DEMAND_SECTION, no examples given in the tsplib repo
    DemandSection,

    /// EDGE_DATA_SECTION, no examples given in the tsplib repo
    EdgeDataSection,
}

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
                "FIXED_EDGES_SECTION\n{}\n-1",
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
            DataSection::TourSection => {
                unimplemented!("fmt is not implemented for TOUR_SECTION yet")
            }
            DataSection::DepotSection => {
                unimplemented!("fmt is not implemented for DEPOT_SECTION yet")
            }
            DataSection::DemandSection => {
                unimplemented!("fmt is not implemented for DEMAND_SECTION yet")
            }
            DataSection::EdgeDataSection => {
                unimplemented!("fmt is not implemented for EDGE_DATA_SECTION yet")
            }
        };
        write!(f, "{}", s)
    }
}
