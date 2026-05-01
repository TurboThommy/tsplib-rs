//! Helper functions to parse individual data sections
use super::errors::ParseError;
use tsplib_core::enums::{DataSection, DataSectionType};

/// Helper function to parse a data section based on its type, which dispatches to the appropriate parsing function for that section type.
///
/// # Arguments
/// * `section_type` - The type of the data section being parsed, which determines which parsing function will be called to parse the lines of the section.
/// * `lines` - The lines of the data section being parsed, which will be passed to the appropriate parsing function based on the section type.
///
/// # Returns
/// * `Result<DataSection, ParseError>` - The parsed data section, or an error if parsing fails.
///
/// Errors
/// * `Err(ParseError::UnknownDataSectionType)` - An error indicating that the section type does not correspond to a known data section type, with the error containing the unknown data section type.
/// * `Err(ParseError::_LineFormat)` - An error indicating that the lines of the data section do not conform to the expected format for that section type, with the error containing details about the specific issue with the line format.
/// * `Err(ParseError::UnimplementedSectionType)` - An error indicating that the section type does not correspond to a known data section type for which parsing is implemented, with the error containing the unimplemented section type.
pub(super) fn try_to_data_section(
    section_type: &DataSectionType,
    lines: Vec<&str>,
) -> Result<DataSection, ParseError> {
    let data_section = match section_type {
        // example given in the tsplib repository
        DataSectionType::NodeCoordSection => try_parse_node_coord_section(lines)?,
        DataSectionType::FixedEdgesSection => try_parse_fixed_edges_section(lines)?,
        DataSectionType::DisplayDataSection => try_parse_display_data_section(lines)?,
        DataSectionType::EdgeWeightSection => try_parse_edge_weight_section(lines)?,

        // other section types for which no examples exist in the tsplib repository
        DataSectionType::TourSection => {
            return Err(ParseError::UnimplementedSectionType("TOUR_SECTION".into()));
        }
        DataSectionType::DepotSection => {
            return Err(ParseError::UnimplementedSectionType("DEPOT_SECTION".into()));
        }
        DataSectionType::DemandSection => {
            return Err(ParseError::UnimplementedSectionType(
                "DEMAND_SECTION".into(),
            ));
        }
        DataSectionType::EdgeDataSection => {
            return Err(ParseError::UnimplementedSectionType(
                "EDGE_DATA_SECTION".into(),
            ));
        }
    };
    Ok(data_section)
}

/// Helper function to parse the NODE_COORD_SECTION data section, which determines whether the coordinates are 2D or 3D based on the number of values in the first line and dispatches to the appropriate parsing function for that coordinate type.
///
/// # Arguments
/// * `lines` - The lines of the NODE_COORD_SECTION data section being parsed, which will be analyzed to determine whether the coordinates are 2D or 3D and then passed to the appropriate parsing function for that coordinate type.
///
/// # Returns
/// * `Result<DataSection, ParseError>` - The parsed NODE_COORD_SECTION data section, containing either 2D or 3D coordinates based on the format of the lines, or an error if parsing fails.
///
/// # Errors
/// * `Err(ParseError::InvalidLineFormat)` - An error indicating that the first line of the NODE_COORD_SECTION is empty, with the error containing a message that the section cannot be empty.
/// * `Err(ParseError::InvalidNodeCoordLineFormat)` - An error indicating that the first line of the NODE_COORD_SECTION does not contain either 3 or 4 values, with the error containing the invalid line format in the NODE_COORD_SECTION.
fn try_parse_node_coord_section(lines: Vec<&str>) -> Result<DataSection, ParseError> {
    let first_line = lines.first().ok_or_else(|| {
        ParseError::InvalidLineFormat("NODE_COORD_SECTION cannot be empty".to_string())
    })?;
    match first_line.split_whitespace().count() {
        3 => Ok(try_parse_node_coord_section_2d(lines)?),
        4 => Ok(try_parse_node_coord_section_3d(lines)?),
        _ => Err(ParseError::InvalidNodeCoordLineFormat(
            first_line.to_string(),
        )),
    }
}

/// Helper function to parse the NODE_COORD_SECTION data section when the coordinates are in 2D format, which expects each line to contain a node index followed by x and y coordinates, and collects these into a vector of tuples.
///
/// # Arguments
/// * `lines` - The lines of the NODE_COORD_SECTION data section being parsed, which are expected to contain 2D coordinates in the format of a node index followed by x and y coordinates.
///
/// # Returns
/// * `Result<DataSection, ParseError>` - The parsed NODE_COORD_SECTION data section, containing a vector of tuples with node indices and their corresponding x and y coordinates, or an error if parsing fails.
///
/// # Errors
/// * `Err(ParseError::InvalidNodeCoordLineFormat)` if any line in the NODE_COORD_SECTION does not contain exactly 3 values, with the error containing the invalid line format in the NODE_COORD_SECTION.
/// * `Err(ParseError::InvalidNodeIndexValue)` if any node index value in the lines cannot be parsed as a usize, with the error containing the invalid node index value.
/// * `Err(ParseError::InvalidCoordinateValue)` if any coordinate value in the lines cannot be parsed as an f64, with the error containing the invalid coordinate value.
fn try_parse_node_coord_section_2d(lines: Vec<&str>) -> Result<DataSection, ParseError> {
    let coords = lines
        .into_iter()
        .map(|line| {
            let parts = line.split_whitespace().collect::<Vec<_>>();
            if parts.len() != 3 {
                Err(ParseError::InvalidNodeCoordLineFormat(line.to_string()))?;
            }
            Ok((
                parts[0]
                    .parse()
                    .map_err(|_| ParseError::InvalidNodeIndexValue(parts[0].to_string()))?,
                parts[1]
                    .parse()
                    .map_err(|_| ParseError::InvalidCoordinateValue(parts[1].to_string()))?,
                parts[2]
                    .parse()
                    .map_err(|_| ParseError::InvalidCoordinateValue(parts[2].to_string()))?,
            ))
        })
        .collect::<Result<Vec<_>, ParseError>>()?;

    Ok(DataSection::NodeCoordSection2D(coords))
}

/// Helper function to parse the NODE_COORD_SECTION data section when the coordinates are in 3D format, which expects each line to contain a node index followed by x, y, and z coordinates, and collects these into a vector of tuples.
///
/// # Arguments
/// * `lines` - The lines of the NODE_COORD_SECTION data section being parsed, which are expected to contain 3D coordinates in the format of a node index followed by x, y, and z coordinates.
///
/// # Returns
/// * `Result<DataSection, ParseError>` - The parsed NODE_COORD_SECTION data section, containing a vector of tuples with node indices and their corresponding x, y, and z coordinates, or an error if parsing fails.
///
/// # Errors
/// * `Err(ParseError::InvalidNodeCoordLineFormat)` if any line in the NODE_COORD_SECTION does not contain exactly 4 values, with the error containing the invalid line format in the NODE_COORD_SECTION.
/// * `Err(ParseError::InvalidNodeIndexValue)` if any node index value in the lines cannot be parsed as a usize, with the error containing the invalid node index value.
/// * `Err(ParseError::InvalidCoordinateValue)` if any coordinate value in the lines cannot be parsed as an f64, with the error containing the invalid coordinate value.
fn try_parse_node_coord_section_3d(lines: Vec<&str>) -> Result<DataSection, ParseError> {
    let coords = lines
        .into_iter()
        .map(|line| {
            let parts = line.split_whitespace().collect::<Vec<_>>();
            if parts.len() != 4 {
                Err(ParseError::InvalidNodeCoordLineFormat(line.to_string()))?;
            }
            Ok((
                parts[0]
                    .parse()
                    .map_err(|_| ParseError::InvalidNodeIndexValue(parts[0].to_string()))?,
                parts[1]
                    .parse()
                    .map_err(|_| ParseError::InvalidCoordinateValue(parts[1].to_string()))?,
                parts[2]
                    .parse()
                    .map_err(|_| ParseError::InvalidCoordinateValue(parts[2].to_string()))?,
                parts[3]
                    .parse()
                    .map_err(|_| ParseError::InvalidCoordinateValue(parts[3].to_string()))?,
            ))
        })
        .collect::<Result<Vec<_>, ParseError>>()?;

    Ok(DataSection::NodeCoordSection3D(coords))
}

/// Helper function to parse the FIXED_EDGES_SECTION data section, which expects each line to contain two node indices representing a fixed edge, and collects these into a vector of tuples.
///
/// # Arguments
/// * `lines` - The lines of the FIXED_EDGES_SECTION data section being parsed, which are expected to contain fixed edges in the format of two node indices representing a fixed edge.
///
/// # Returns
/// * `Result<DataSection, ParseError>` - The parsed FIXED_EDGES_SECTION data section, containing a vector of tuples with pairs of node indices representing fixed edges, or an error if parsing fails.
///
/// # Errors
/// * `Err(ParseError::InvalidFixedEdgesLineFormat)` if any line in the FIXED_EDGES_SECTION does not contain exactly 2 values, with the error containing the invalid line format in the FIXED_EDGES_SECTION.
/// * `Err(ParseError::InvalidNodeIndexValue)` if any node index value in the lines cannot be parsed as a usize, with the error containing the invalid node index value.
fn try_parse_fixed_edges_section(lines: Vec<&str>) -> Result<DataSection, ParseError> {
    let edges = lines
        .into_iter()
        .map(|line| {
            let parts = line.split_whitespace().collect::<Vec<_>>();
            if parts.len() != 2 {
                Err(ParseError::InvalidFixedEdgesLineFormat(line.to_string()))?;
            }
            Ok((
                parts[0]
                    .parse()
                    .map_err(|_| ParseError::InvalidNodeIndexValue(parts[0].to_string()))?,
                parts[1]
                    .parse()
                    .map_err(|_| ParseError::InvalidNodeIndexValue(parts[1].to_string()))?,
            ))
        })
        .collect::<Result<Vec<_>, ParseError>>()?;

    Ok(DataSection::FixedEdgesSection(edges))
}

/// Helper function to parse the DISPLAY_DATA_SECTION data section, which expects each line to contain a node index followed by x and y coordinates for display purposes, and collects these into a vector of tuples.
///
/// # Arguments
/// * `lines` - The lines of the DISPLAY_DATA_SECTION data section being parsed, which are expected to contain display data in the format of a node index followed by x and y coordinates for display purposes.
///
/// # Returns
/// * `Result<DataSection, ParseError>` - The parsed DISPLAY_DATA_SECTION data section, containing a vector of tuples with node indices and their corresponding x and y coordinates for display purposes, or an error if parsing fails.
///
/// # Errors
/// * `Err(ParseError::InvalidDisplayDataLineFormat)` if any line in the DISPLAY_DATA_SECTION does not contain exactly 3 values, with the error containing the invalid line format in the DISPLAY_DATA_SECTION.
/// * `Err(ParseError::InvalidNodeIndexValue)` if any node index value in the lines cannot be parsed as a usize, with the error containing the invalid node index value.
/// * `Err(ParseError::InvalidCoordinateValue)` if any coordinate value in the lines cannot be parsed as an f64, with the error containing the invalid coordinate value.
fn try_parse_display_data_section(lines: Vec<&str>) -> Result<DataSection, ParseError> {
    let display_data = lines
        .into_iter()
        .map(|line| {
            let parts = line.split_whitespace().collect::<Vec<_>>();
            if parts.len() != 3 {
                Err(ParseError::InvalidDisplayDataLineFormat(line.to_string()))?;
            }
            Ok((
                parts[0]
                    .parse()
                    .map_err(|_| ParseError::InvalidNodeIndexValue(parts[0].to_string()))?,
                parts[1]
                    .parse()
                    .map_err(|_| ParseError::InvalidCoordinateValue(parts[1].to_string()))?,
                parts[2]
                    .parse()
                    .map_err(|_| ParseError::InvalidCoordinateValue(parts[2].to_string()))?,
            ))
        })
        .collect::<Result<Vec<_>, ParseError>>()?;

    Ok(DataSection::DisplayDataSection(display_data))
}

/// Helper function to parse the EDGE_WEIGHT_SECTION data section, which expects each line to contain a series of edge weight values, and collects these into a vector of vectors of f64 values.
///
/// # Arguments
/// * `lines` - The lines of the EDGE_WEIGHT_SECTION data section being parsed, which are expected to contain edge weight values in the format of a series of edge weight values on each line.
///
/// # Returns
/// * `Result<DataSection, ParseError>` - The parsed EDGE_WEIGHT_SECTION data section, containing a vector of vectors of f64 values representing the edge weights, or an error if parsing fails.
///
/// # Errors
/// * `Err(ParseError::InvalidEdgeWeightLineFormat)` if any value in the lines cannot be parsed as an f64, with the error containing the invalid line format in the EDGE_WEIGHT_SECTION.
fn try_parse_edge_weight_section(lines: Vec<&str>) -> Result<DataSection, ParseError> {
    let edge_weights = lines
        .into_iter()
        .map(|line| {
            line.split_whitespace()
                .map(|part| {
                    part.parse()
                        .map_err(|_| ParseError::InvalidEdgeWeightLineFormat(line.to_string()))
                })
                .collect::<Result<Vec<i32>, ParseError>>()
        })
        .collect::<Result<Vec<_>, ParseError>>()?;

    Ok(DataSection::EdgeWeightSection(edge_weights))
}
