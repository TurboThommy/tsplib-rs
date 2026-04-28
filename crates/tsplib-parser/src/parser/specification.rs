//! Helper functions to parse specification fields
use super::SpecificationPart;
use super::errors::ParseError;
use tsplib_core::enums::{
    DisplayDataType, EdgeDataFormat, EdgeWeightFormat, EdgeWeightType, NodeCoordType, ProblemType,
};

/// Helper functions to parse individual header fields.
/// Each function takes the value of the header field as a string and updates the corresponding field in the `SpecificationPart` struct.
///
/// # Arguments
/// * `key` - The key of the header field being parsed, used to determine which field in the `SpecificationPart` struct to update.
/// * `value` - The value of the header field being parsed, which will be parsed and assigned to the corresponding field in the `SpecificationPart` struct.
///
/// # Returns
/// * `Result<(), ParseError>` - `Ok(())` if the header field was successfully parsed and assigned to the corresponding field in the `SpecificationPart` struct, or an error if the key does not correspond to a known header field or if the value cannot be parsed for the specific field.
///
/// Errors
/// * `Err(ParseError::UnknownHeaderField)` - An error indicating that the key does not correspond to a known header field, with the error containing the unknown header field key.
/// * `Err(ParseError::_)` - An error indicating any other issue encountered during the parsing of the header field, with the error containing details about the specific issue (e.g., invalid value format for a specific field).
pub(super) fn try_parse_header_line(
    key: &str,
    value: &str,
    specification: &mut SpecificationPart,
) -> Result<(), ParseError> {
    match key {
        "NAME" => try_parse_name(value, &mut specification.name),
        "TYPE" => try_parse_problem_type(value, &mut specification.problem_type),
        "DIMENSION" => try_parse_dimension(value, &mut specification.dimension),
        "EDGE_WEIGHT_TYPE" => {
            try_parse_edge_weight_type(value, &mut specification.edge_weight_type)
        }
        "COMMENT" => {
            specification.comment.push(value.to_string());
            Ok(())
        }
        "CAPACITY" => try_parse_capacity(value, &mut specification.capacity),
        "EDGE_WEIGHT_FORMAT" => {
            try_parse_edge_weight_format(value, &mut specification.edge_weight_format)
        }
        "EDGE_DATA_FORMAT" => {
            try_parse_edge_data_format(value, &mut specification.edge_data_format)
        }
        "NODE_COORD_TYPE" => try_parse_node_coord_type(value, &mut specification.node_coord_type),
        "DISPLAY_DATA_TYPE" => {
            try_parse_display_data_type(value, &mut specification.display_data_type)
        }
        _ => Err(ParseError::UnknownHeaderField(key.to_string())),
    }
}

/// Helper function to parse the NAME header field, which simply assigns the value to the `name` field in the `SpecificationPart` struct.
///
/// # Arguments
/// * `value` - The value of the NAME header field, which will be assigned to the `name` field in the `SpecificationPart` struct.
/// * `name` - A mutable reference to the `name` field in the `SpecificationPart` struct, which will be updated with the parsed value.
///
/// # Returns
/// * `Ok(())` if the value was successfully assigned to the `name` field in the `SpecificationPart` struct.
///
/// # Errors
/// * For now this function does not return any errors, but in the future it could be extended to return an error if the value is empty or does not meet certain criteria for a valid name.
fn try_parse_name(value: &str, name: &mut Option<String>) -> Result<(), ParseError> {
    *name = Some(value.to_string());
    Ok(())
}

/// Helper function to parse the TYPE header field, which maps the string value to the corresponding `ProblemType` enum variant and assigns it to the `problem_type` field in the `SpecificationPart` struct.
///
/// # Arguments
/// * `value` - The value of the TYPE header field, which will be parsed and assigned to the `problem_type` field in the `SpecificationPart` struct.
/// * `problem_type` - A mutable reference to the `problem_type` field in the `SpecificationPart` struct, which will be updated with the parsed value if it corresponds to a known problem type.
///
/// # Returns
/// * `Result<(), ParseError>` if the value was successfully parsed and assigned to the `problem_type` field in the `SpecificationPart` struct, or an error if the value cannot be parsed.
///
/// Errors
/// * `Err(ParseError::UnknownProblemType)` - An error indicating that the value does not correspond to a known problem type, with the error containing the unknown problem type value.
fn try_parse_problem_type(
    value: &str,
    problem_type: &mut Option<ProblemType>,
) -> Result<(), ParseError> {
    let parsed_problem_type = match value {
        "TSP" => Ok(Some(ProblemType::TSP)),
        "TSP (M.~Hofmeister)" => Ok(Some(ProblemType::TSP)), // special case for some wrongly formatted files in the tsplib repo
        "ATSP" => Ok(Some(ProblemType::ATSP)),
        "SOP" => Ok(Some(ProblemType::SOP)),
        "HCP" => Ok(Some(ProblemType::HCP)),
        "CVRP" => Ok(Some(ProblemType::CVRP)),
        _ => Err(ParseError::UnknownProblemType(value.to_string())),
    };
    *problem_type = parsed_problem_type?;
    Ok(())
}

/// Helper function to parse the DIMENSION header field, which simply parses the value as a usize and assigns it to the `dimension` field in the `SpecificationPart` struct.
///
/// # Arguments
/// * `value` - The value of the DIMENSION header field, which will be parsed as a usize and assigned to the `dimension` field in the `SpecificationPart` struct.
/// * `dimension` - A mutable reference to the `dimension` field in the `SpecificationPart` struct, which will be updated with the parsed value if it can be successfully parsed as a usize.
///
/// # Returns
/// * `Result<(), ParseError>` if the value was successfully parsed as a usize and assigned to the `dimension` field in the `SpecificationPart` struct, or an error if the value cannot be parsed as a usize.
///
/// # Errors
/// * `Err(ParseError::InvalidDimensionValue)` - An error indicating that the value cannot be parsed as a usize, with the error containing the invalid dimension value.
fn try_parse_dimension(value: &str, dimension: &mut Option<usize>) -> Result<(), ParseError> {
    *dimension = Some(
        value
            .parse()
            .map_err(|_| ParseError::InvalidDimensionValue(value.to_string()))?,
    );
    Ok(())
}

/// Helper function to parse the EDGE_WEIGHT_TYPE header field, which maps the string value to the corresponding `EdgeWeightType` enum variant and assigns it to the `edge_weight_type` field in the `SpecificationPart` struct.
///
/// # Arguments
/// * `value` - The value of the EDGE_WEIGHT_TYPE header field, which will be parsed and assigned to the `edge_weight_type` field in the `SpecificationPart` struct.
/// * `edge_weight_type` - A mutable reference to the `edge_weight_type` field in the `SpecificationPart` struct, which will be updated with the parsed value if it corresponds to a known edge weight type.
///
/// # Returns
/// * `Result<(), ParseError>` if the value was successfully parsed and assigned to the `edge_weight_type` field in the `SpecificationPart` struct, or an error if the value cannot be parsed.
///
/// Errors
/// * `Err(ParseError::UnknownEdgeWeightType)` - An error indicating that the value does not correspond to a known edge weight type, with the error containing the unknown edge weight type value.
fn try_parse_edge_weight_type(
    value: &str,
    edge_weight_type: &mut Option<EdgeWeightType>,
) -> Result<(), ParseError> {
    let parsed_edge_weight_type = match value {
        "EXPLICIT" => Ok(Some(EdgeWeightType::Explicit)),
        "EUC_2D" => Ok(Some(EdgeWeightType::Euc2D)),
        "EUC_3D" => Ok(Some(EdgeWeightType::Euc3D)),
        "MAX_2D" => Ok(Some(EdgeWeightType::Max2D)),
        "MAX_3D" => Ok(Some(EdgeWeightType::Max3D)),
        "MAN_2D" => Ok(Some(EdgeWeightType::Man2D)),
        "MAN_3D" => Ok(Some(EdgeWeightType::Man3D)),
        "CEIL_2D" => Ok(Some(EdgeWeightType::Ceil2D)),
        "GEO" => Ok(Some(EdgeWeightType::Geo)),
        "ATT" => Ok(Some(EdgeWeightType::Att)),
        "XRAY1" => Ok(Some(EdgeWeightType::Xray1)),
        "XRAY2" => Ok(Some(EdgeWeightType::Xray2)),
        "SPECIAL" => Ok(Some(EdgeWeightType::Special)),
        _ => Err(ParseError::UnknownEdgeWeightType(value.to_string())),
    };
    *edge_weight_type = parsed_edge_weight_type?;
    Ok(())
}

/// Helper function to parse the CAPACITY header field, which simply parses the value as a usize and assigns it to the `capacity` field in the `SpecificationPart` struct.
///
/// # Arguments
/// * `value` - The value of the CAPACITY header field, which will be parsed as a usize and assigned to the `capacity` field in the `SpecificationPart` struct.
/// * `capacity` - A mutable reference to the `capacity` field in the `SpecificationPart` struct, which will be updated with the parsed value if it can be successfully parsed as a usize.
///
/// # Returns
/// * `Result<(), ParseError>` if the value was successfully parsed as a usize and assigned to the `capacity` field in the `SpecificationPart` struct, or an error if the value cannot be parsed as a usize.
///
/// Errors
/// * `Err(ParseError::InvalidCapacityValue)` - An error indicating that the value cannot be parsed as a usize, with the error containing the invalid capacity value.
fn try_parse_capacity(value: &str, capacity: &mut Option<usize>) -> Result<(), ParseError> {
    *capacity = Some(
        value
            .parse()
            .map_err(|_| ParseError::InvalidCapacityValue(value.to_string()))?,
    );
    Ok(())
}

/// Helper function to parse the EDGE_WEIGHT_FORMAT header field, which maps the string value to the corresponding `EdgeWeightFormat` enum variant and assigns it to the `edge_weight_format` field in the `SpecificationPart` struct.
///
/// # Arguments
/// * `value` - The value of the EDGE_WEIGHT_FORMAT header field, which will be parsed and assigned to the `edge_weight_format` field in the `SpecificationPart` struct.
/// * `edge_weight_format` - A mutable reference to the `edge_weight_format` field in the `SpecificationPart` struct, which will be updated with the parsed value if it corresponds to a known edge weight format.
///
/// # Returns
/// * `Result<(), ParseError>` if the value was successfully parsed and assigned to the `edge_weight_format` field in the `SpecificationPart` struct, or an error if the value cannot be parsed.
///
/// # Errors
/// * `Err(ParseError::UnknownEdgeWeightFormat)` - An error indicating that the value does not correspond to a known edge weight format, with the error containing the unknown edge weight format value
fn try_parse_edge_weight_format(
    value: &str,
    edge_weight_format: &mut Option<EdgeWeightFormat>,
) -> Result<(), ParseError> {
    let parsed_edge_weight_format = match value {
        "FUNCTION" => Ok(Some(EdgeWeightFormat::Function)),
        "FULL_MATRIX" => Ok(Some(EdgeWeightFormat::FullMatrix)),
        "UPPER_ROW" => Ok(Some(EdgeWeightFormat::UpperRow)),
        "LOWER_ROW" => Ok(Some(EdgeWeightFormat::LowerRow)),
        "UPPER_DIAG_ROW" => Ok(Some(EdgeWeightFormat::UpperDiagRow)),
        "LOWER_DIAG_ROW" => Ok(Some(EdgeWeightFormat::LowerDiagRow)),
        "UPPER_COL" => Ok(Some(EdgeWeightFormat::UpperCol)),
        "LOWER_COL" => Ok(Some(EdgeWeightFormat::LowerCol)),
        "UPPER_DIAG_COL" => Ok(Some(EdgeWeightFormat::UpperDiagCol)),
        "LOWER_DIAG_COL" => Ok(Some(EdgeWeightFormat::LowerDiagCol)),
        _ => Err(ParseError::UnknownEdgeWeightFormat(value.to_string())),
    };
    *edge_weight_format = parsed_edge_weight_format?;
    Ok(())
}

/// Helper function to parse the EDGE_DATA_FORMAT header field, which maps the string value to the corresponding `EdgeDataFormat` enum variant and assigns it to the `edge_data_format` field in the `SpecificationPart` struct.
///
/// # Arguments
/// * `value` - The value of the EDGE_DATA_FORMAT header field, which will be parsed and assigned to the `edge_data_format` field in the `SpecificationPart` struct.
/// * `edge_data_format` - A mutable reference to the `edge_data_format` field in the `SpecificationPart` struct, which will be updated with the parsed value if it corresponds to a known edge data format.
///
/// # Returns
/// * `Result<(), ParseError>` if the value was successfully parsed and assigned to the `edge_data_format` field in the `SpecificationPart` struct, or an error if the value cannot be parsed.
///
/// # Errors
/// * `Err(ParseError::UnknownEdgeDataFormat)` - An error indicating that the value does not correspond to a known edge data format, with the error containing the unknown edge data format value.
fn try_parse_edge_data_format(
    value: &str,
    edge_data_format: &mut Option<EdgeDataFormat>,
) -> Result<(), ParseError> {
    let parsed_edge_data_format = match value {
        "EDGE_LIST" => Ok(Some(EdgeDataFormat::EdgeList)),
        "ADJ_LIST" => Ok(Some(EdgeDataFormat::AdjList)),
        _ => Err(ParseError::UnknownEdgeDataFormat(value.to_string())),
    };
    *edge_data_format = parsed_edge_data_format?;
    Ok(())
}

/// Helper function to parse the NODE_COORD_TYPE header field, which maps the string value to the corresponding `NodeCoordType` enum variant and assigns it to the `node_coord_type` field in the `SpecificationPart` struct.
///
/// # Arguments
/// * `value` - The value of the NODE_COORD_TYPE header field, which will be parsed and assigned to the `node_coord_type` field in the `SpecificationPart` struct.
/// * `node_coord_type` - A mutable reference to the `node_coord_type` field in the `SpecificationPart` struct, which will be updated with the parsed value if it corresponds to a known node coordinate type.
///
/// # Returns
/// * `Result<(), ParseError>` if the value was successfully parsed and assigned to the `node_coord_type` field in the `SpecificationPart` struct, or an error if the value cannot be parsed.
///
/// # Errors
/// * `Err(ParseError::UnknownNodeCoordType)` - An error indicating that the value does not correspond to a known node coordinate type, with the error containing the unknown node coordinate type value.
fn try_parse_node_coord_type(
    value: &str,
    node_coord_type: &mut Option<NodeCoordType>,
) -> Result<(), ParseError> {
    let parsed_node_coord_type = match value {
        "TWOD_COORDS" => Ok(Some(NodeCoordType::TwoDCoords)),
        "THREED_COORDS" => Ok(Some(NodeCoordType::ThreeDCoords)),
        "NO_COORDS" => Ok(Some(NodeCoordType::NoCoords)),
        _ => Err(ParseError::UnknownNodeCoordType(value.to_string())),
    };
    *node_coord_type = parsed_node_coord_type?;
    Ok(())
}

/// Helper function to parse the DISPLAY_DATA_TYPE header field, which maps the string value to the corresponding `DisplayDataType` enum variant and assigns it to the `display_data_type` field in the `SpecificationPart` struct.
///
/// # Arguments
/// * `value` - The value of the DISPLAY_DATA_TYPE header field, which will be parsed and assigned to the `display_data_type` field in the `SpecificationPart` struct.
/// * `display_data_type` - A mutable reference to the `display_data_type` field in the `SpecificationPart` struct, which will be updated with the parsed value if it corresponds to a known display data type.
/// # Returns
/// * `Result<(), ParseError>` if the value was successfully parsed and assigned to the `display_data_type` field in the `SpecificationPart` struct, or an error if the value cannot be parsed.
///
/// # Errors
/// * `Err(ParseError::UnknownDisplayDataType)` - An error indicating that the value does not correspond to a known display data type, with the error containing the unknown display data type value.
fn try_parse_display_data_type(
    value: &str,
    display_data_type: &mut Option<DisplayDataType>,
) -> Result<(), ParseError> {
    let parsed_display_data_type = match value {
        "COORD_DISPLAY" => Ok(Some(DisplayDataType::CoordDisplay)),
        "TWOD_DISPLAY" => Ok(Some(DisplayDataType::TwoDDisplay)),
        "NO_DISPLAY" => Ok(Some(DisplayDataType::NoDisplay)),
        _ => Err(ParseError::UnknownDisplayDataType(value.to_string())),
    };
    *display_data_type = parsed_display_data_type?;
    Ok(())
}
