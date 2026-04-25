use errors::ParseError;
use tsplib_core::{
    enums::{
        DataSection, DataSectionType, DisplayDataType, EdgeDataFormat, EdgeWeightFormat,
        EdgeWeightType, NodeCoordType, ProblemType,
    },
    models::TSPInstance,
};

/// Internal struct to hold the parsed specification/header fields while parsing the file
struct SpecificationPart {
    name: Option<String>,
    problem_type: Option<ProblemType>,
    dimension: Option<usize>,
    edge_weight_type: Option<EdgeWeightType>,
    comment: Vec<String>,
    capacity: Option<usize>,
    edge_weight_format: Option<EdgeWeightFormat>,
    edge_data_format: Option<EdgeDataFormat>,
    node_coord_type: Option<NodeCoordType>,
    display_data_type: Option<DisplayDataType>,
}

impl SpecificationPart {
    fn new() -> Self {
        Self {
            name: None,
            problem_type: None,
            dimension: None,
            edge_weight_type: None,
            comment: Vec::new(),
            capacity: None,
            edge_weight_format: None,
            edge_data_format: None,
            node_coord_type: None,
            display_data_type: None,
        }
    }
}

/// Internal parser state
enum ParserState {
    /// The parser is currently parsing the header/specification part of the file, which consists of key-value pairs until the first section header is encountered.
    Header,
    /// The parser is currently parsing a data section, which consists of lines of data until the next section header or the end of the file is encountered.
    /// The specific type of data section being parsed is indicated by the associated `DataSectionType` value.
    Section(DataSectionType),
}

impl ParserState {
    /// Helper function to transition to a new section based on the section header line.
    ///
    /// # Arguments:
    /// * `self` - The current parser state, which will be updated to the new section if the line is a valid section header.
    /// * `line` - The line containing the section header.
    ///
    /// # Panics
    /// * If the line does not correspond to a known section header, the function will panic with an error message indicating the unknown section type.
    fn new_section_from_line(&mut self, line: &str) {
        *self = match line.trim() {
            "NODE_COORD_SECTION" => ParserState::Section(DataSectionType::NodeCoordSection),
            "FIXED_EDGES_SECTION" => ParserState::Section(DataSectionType::FixedEdgesSection),
            "DISPLAY_DATA_SECTION" => ParserState::Section(DataSectionType::DisplayDataSection),
            "EDGE_WEIGHT_SECTION" => ParserState::Section(DataSectionType::EdgeWeightSection),
            _ => panic!("Unknown section type: {}", line),
        };
    }

    /// Helper function to transition to a new section based on the section header line, with error handling.
    ///
    /// # Arguments:
    /// * `self` - The current parser state, which will be updated to the new section if the line is a valid section header.
    /// * `line` - The line containing the section header.
    ///
    /// # Returns
    /// * `Ok(())` if the line corresponds to a known section header and the parser state was successfully updated.
    /// * `Err(ParseError::UnknownSectionType)` if the line does not correspond to a known section header, with the error containing the unknown section type.
    fn try_new_section_from_line(&mut self, line: &str) -> Result<(), ParseError> {
        *self = match line.trim() {
            "NODE_COORD_SECTION" => Ok(ParserState::Section(DataSectionType::NodeCoordSection)),
            "FIXED_EDGES_SECTION" => Ok(ParserState::Section(DataSectionType::FixedEdgesSection)),
            "DISPLAY_DATA_SECTION" => Ok(ParserState::Section(DataSectionType::DisplayDataSection)),
            "EDGE_WEIGHT_SECTION" => Ok(ParserState::Section(DataSectionType::EdgeWeightSection)),
            _ => Err(ParseError::UnknownSectionType(line.to_string())),
        }?;
        Ok(())
    }
}

/// Main parsing function that takes the content of a TSP file as a string and returns a TSPInstance.
/// For parsing the function uses a state machine approach, where it remains parsing key-value pairs for the header part of the file until it encounters the first section header.
/// When a section header is encountered, it transitions to parsing the corresponding data section until it encounters the next section header or the end of the file, at which point it transitions back to parsing the header or finishes parsing, respectively.
/// Once the first section header is encountered, the remaining lines are considered part of the data part of the file.
///
/// # Arguments
/// * `file_content` - A string containing the content of the TSP file to be parsed.
///
/// # Returns
/// * `TSPInstance` - The parsed TSP instance containing the specification and data sections from the file.
///
/// # Panics
/// * If the file content is not in the expected format, the function may panic with an error message indicating the specific issue encountered during parsing, such as invalid line formats, unknown header fields, or missing required fields.
pub fn parse(file_content: String) -> TSPInstance {
    let mut specification = SpecificationPart::new();
    let mut data_sections: Vec<DataSection> = Vec::new();
    let mut state = ParserState::Header;
    let mut curr_lines: Vec<&str> = Vec::new();

    // Iterate through each line of the file content, trimming whitespace and filtering out empty lines
    for line in file_content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
    {
        // state machine to parse the file content based on the current parser state
        match state {
            // parse each line as key-value pair or transition to new state if a section header is encountered
            ParserState::Header => {
                if line.contains(':') {
                    let parts = line.split(':').map(|s| s.trim()).collect::<Vec<_>>();
                    if parts.len() != 2 {
                        panic!("Invalid line format: {}", line);
                    }

                    parse_header_line(parts[0], parts[1], &mut specification);
                } else if line.contains("SECTION") {
                    state.new_section_from_line(line);
                } else if !line.trim().is_empty() {
                    panic!("Invalid line in header: {}", line);
                }
            }

            // parse each line as part of the current data section, or transition to new section state if a new section header is encountered or the end of the file is reached
            ParserState::Section(ref section_type) => {
                // end of section or file reached, save the parsed data section
                if line == "EOF" || line == "-1" {
                    data_sections.push(to_data_section(section_type, curr_lines));
                    curr_lines = Vec::new();
                    continue;
                }

                // new section encountered, save the parsed data section and transition to the new section state
                if line.contains("SECTION") {
                    data_sections.push(to_data_section(section_type, curr_lines));
                    curr_lines = Vec::new();
                    state.new_section_from_line(line);
                    continue;
                }

                // line is part of the current data section, add it to the current lines buffer
                curr_lines.push(line);
            }
        }
    }

    // After parsing all lines, create the TSPInstance from the parsed specification and data sections
    create_tsp_instance(specification, data_sections)
}

/// Main parsing function that takes the content of a TSP file as a string and returns a TSPInstance.
/// For parsing the function uses a state machine approach, where it remains parsing key-value pairs for the header part of the file until it encounters the first section header.
/// When a section header is encountered, it transitions to parsing the corresponding data section until it encounters the next section header or the end of the file, at which point it transitions back to parsing the header or finishes parsing, respectively.
/// Once the first section header is encountered, the remaining lines are considered part of the data part of the file.
///
/// # Arguments
/// * `file_content` - A string containing the content of the TSP file to be parsed.
///
/// # Returns
/// * `Result<TSPInstance, ParseError>` - The parsed TSP instance containing the specification and data sections from the file, or an error if parsing fails.
///
/// # Errors
/// * `Err(ParseError)` - An error indicating the specific issue encountered during parsing, such as invalid line formats, unknown header fields, missing required fields, or unknown section types.
pub fn try_parse(file_content: String) -> Result<TSPInstance, ParseError> {
    let mut specification = SpecificationPart::new();
    let mut data_sections: Vec<DataSection> = Vec::new();
    let mut state = ParserState::Header;
    let mut curr_lines: Vec<&str> = Vec::new();

    // Iterate through each line of the file content, trimming whitespace and filtering out empty lines
    for line in file_content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
    {
        // state machine to parse the file content based on the current parser state
        match state {
            // parse each line as key-value pair or transition to new state if a section header is encountered
            ParserState::Header => {
                if line.contains(':') {
                    let parts = line.split(':').map(|s| s.trim()).collect::<Vec<_>>();
                    if parts.len() != 2 {
                        Err(ParseError::InvalidLineFormat(line.to_string()))?;
                    }

                    try_parse_header_line(parts[0], parts[1], &mut specification)?;
                } else if line.contains("SECTION") {
                    state.try_new_section_from_line(line)?;
                } else if !line.trim().is_empty() {
                    Err(ParseError::EmptyLineInHeader)?;
                }
            }

            // parse each line as part of the current data section, or transition to new section state if a new section header is encountered or the end of the file is reached
            ParserState::Section(ref section_type) => {
                // end of section or file reached, save the parsed data section
                if line == "EOF" || line == "-1" {
                    data_sections.push(try_to_data_section(section_type, curr_lines)?);
                    curr_lines = Vec::new();
                    continue;
                }

                // new section encountered, save the parsed data section and transition to the new section state
                if line.contains("SECTION") {
                    data_sections.push(try_to_data_section(section_type, curr_lines)?);
                    curr_lines = Vec::new();
                    state.try_new_section_from_line(line)?;
                    continue;
                }

                // line is part of the current data section, add it to the current lines buffer
                curr_lines.push(line);
            }
        }
    }

    // After parsing all lines, create the TSPInstance from the parsed specification and data sections
    try_create_tsp_instance(specification, data_sections)
}

// -----------------------------------------------------------------------------------------------------------------------------------------------
// Helper functions to create TSPInstance from the parsed specification and data sections, and to parse individual header fields and data sections
// -----------------------------------------------------------------------------------------------------------------------------------------------

/// Helper function to create a TSPInstance from the parsed specification and data sections.
///
/// # Arguments
/// * `specification` - The parsed specification/header fields from the TSP file, containing optional values for each field.
/// * `data_sections` - The parsed data sections from the TSP file, containing the specific data for each section.
///
/// # Returns
/// * `TSPInstance` - The created TSP instance containing the specification and data sections from the file.
///
/// # Panics
/// * If any of the required fields in the specification are missing (i.e., `name`, `problem_type`, `dimension`, or `edge_weight_type`), the function will panic with an error message indicating the missing field.
fn create_tsp_instance(
    specification: SpecificationPart,
    data_sections: Vec<DataSection>,
) -> TSPInstance {
    tsplib_core::models::TSPInstance {
        // required fields, panics if any of these are missing from the specification
        name: specification.name.expect("Missing required field: NAME"),
        problem_type: specification
            .problem_type
            .expect("Missing required field: TYPE"),
        dimension: specification
            .dimension
            .expect("Missing required field: DIMENSION"),
        edge_weight_type: specification
            .edge_weight_type
            .expect("Missing required field: EDGE_WEIGHT_TYPE"),

        // optional fields
        comment: Some(specification.comment), // comment is optional, but we still want to include it in the TSPInstance, so we wrap it in Some() even if it's empty
        capacity: specification.capacity,
        edge_weight_format: specification.edge_weight_format,
        edge_data_format: specification.edge_data_format,
        node_coord_type: specification.node_coord_type,
        display_data_type: specification.display_data_type,
        data_sections,
    }
}

/// Helper function to create a TSPInstance from the parsed specification and data sections.
///
/// # Arguments
/// * `specification` - The parsed specification/header fields from the TSP file, containing optional values for each field.
/// * `data_sections` - The parsed data sections from the TSP file, containing the specific data for each section.
///
/// # Returns
/// * `Result<TSPInstance, ParseError>` - The created TSP instance containing the specification and data sections from the file, or an error if any required fields are missing.
///
/// # Errors
/// * `Err(ParseError::MissingRequiredField)` - An error indicating that a required field is missing from the specification, with the error containing the name of the missing field.
/// * `Err(ParseError::_)` - An error indicating any other issue encountered during the creation of the TSPInstance, with the error containing details about the specific issue.
fn try_create_tsp_instance(
    specification: SpecificationPart,
    data_sections: Vec<DataSection>,
) -> Result<TSPInstance, ParseError> {
    let tsp_instance = tsplib_core::models::TSPInstance {
        // required fields, returns an error if any of these are missing from the specification
        name: specification
            .name
            .ok_or_else(|| ParseError::MissingRequiredField("NAME".to_string()))?,
        problem_type: specification
            .problem_type
            .ok_or_else(|| ParseError::MissingRequiredField("TYPE".to_string()))?,
        dimension: specification
            .dimension
            .ok_or_else(|| ParseError::MissingRequiredField("DIMENSION".to_string()))?,
        edge_weight_type: specification
            .edge_weight_type
            .ok_or_else(|| ParseError::MissingRequiredField("EDGE_WEIGHT_TYPE".to_string()))?,

        // optional fields
        comment: Some(specification.comment), // comment is optional, but we still want to include it in the TSPInstance, so we wrap it in Some() even if it's empty
        capacity: specification.capacity,
        edge_weight_format: specification.edge_weight_format,
        edge_data_format: specification.edge_data_format,
        node_coord_type: specification.node_coord_type,
        display_data_type: specification.display_data_type,
        data_sections,
    };
    Ok(tsp_instance)
}

// -----------------------------------------------------------------------------------------------------------------------------------------------
// Helper functions to parse specification fields, with both panicking and error-handling versions for each field
// -----------------------------------------------------------------------------------------------------------------------------------------------

/// Helper functions to parse individual header fields.
/// Each function takes the value of the header field as a string and updates the corresponding field in the `SpecificationPart` struct.
///
/// # Arguments
/// * `key` - The key of the header field being parsed, used to determine which field in the `SpecificationPart` struct to update.
/// * `value` - The value of the header field being parsed, which will be parsed and assigned to the corresponding field in the `SpecificationPart` struct.
///
/// # Panics
/// * If the key does not correspond to a known header field, the function will panic with an error message indicating the unknown header field.
fn parse_header_line(key: &str, value: &str, specification: &mut SpecificationPart) {
    match key {
        "NAME" => specification.name = Some(value.to_string()),
        "TYPE" => specification.problem_type = parse_problem_type(value),
        "DIMENSION" => specification.dimension = parse_dimension(value),
        "EDGE_WEIGHT_TYPE" => {
            specification.edge_weight_type = parse_edge_weight_type(value);
        }
        "COMMENT" => specification.comment.push(value.to_string()),
        "CAPACITY" => specification.capacity = parse_capacity(value),
        "EDGE_WEIGHT_FORMAT" => {
            specification.edge_weight_format = parse_edge_weight_format(value);
        }
        "EDGE_DATA_FORMAT" => {
            specification.edge_data_format = parse_edge_data_format(value);
        }
        "NODE_COORD_TYPE" => {
            specification.node_coord_type = parse_node_coord_type(value);
        }
        "DISPLAY_DATA_TYPE" => {
            specification.display_data_type = parse_display_data_type(value);
        }
        _ => panic!("Unknown header field: {}", key),
    }
}

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
fn try_parse_header_line(
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

/// Helper function to parse the TYPE header field, which maps the string value to the corresponding `ProblemType` enum variant.
///
/// # Arguments
/// * `value` - The value of the TYPE header field, which will be parsed.
///
/// # Returns
/// * `Some(ProblemType)` if the value corresponds to a known problem type and was successfully parsed.
///
/// # Panics
/// * If the value does not correspond to a known problem type, the function will panic with an error message indicating the unknown problem type.
fn parse_problem_type(value: &str) -> Option<ProblemType> {
    match value {
        "TSP" => Some(ProblemType::TSP),
        "TSP (M.~Hofmeister)" => Some(ProblemType::TSP), // special case for some wrongly formatted files in the tsplib repo
        "ATSP" => Some(ProblemType::ATSP),
        "SOP" => Some(ProblemType::SOP),
        "HCP" => Some(ProblemType::HCP),
        "CVRP" => Some(ProblemType::CVRP),
        _ => panic!("Unknown problem type: {}", value),
    }
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

/// Helper function to parse the DIMENSION header field, which simply parses the value as a usize.
///
/// # Arguments
/// * `value` - The value of the DIMENSION header field, which will be parsed as a usize.
///
/// # Returns
/// * `Some(usize)` if the value was successfully parsed as a usize.
///
/// # Panics
/// * If the value cannot be parsed as a usize, the function will panic with an error message indicating the invalid dimension value.
fn parse_dimension(value: &str) -> Option<usize> {
    Some(value.parse().expect("Invalid dimension value"))
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

/// Helper function to parse the EDGE_WEIGHT_TYPE header field, which maps the string value to the corresponding `EdgeWeightType` enum variant.
///
/// # Arguments
/// * `value` - The value of the EDGE_WEIGHT_TYPE header field, which will be parsed.
///
/// # Returns
/// * `Some(EdgeWeightType)` if the value corresponds to a known edge weight type and was successfully parsed.
///
/// # Panics
/// * If the value does not correspond to a known edge weight type, the function will panic with an error message indicating the unknown edge weight type.
fn parse_edge_weight_type(value: &str) -> Option<EdgeWeightType> {
    match value {
        "EXPLICIT" => Some(EdgeWeightType::Explicit),
        "EUC_2D" => Some(EdgeWeightType::Euc2D),
        "EUC_3D" => Some(EdgeWeightType::Euc3D),
        "MAX_2D" => Some(EdgeWeightType::Max2D),
        "MAX_3D" => Some(EdgeWeightType::Max3D),
        "MAN_2D" => Some(EdgeWeightType::Man2D),
        "MAN_3D" => Some(EdgeWeightType::Man3D),
        "CEIL_2D" => Some(EdgeWeightType::Ceil2D),
        "GEO" => Some(EdgeWeightType::Geo),
        "ATT" => Some(EdgeWeightType::Att),
        "XRAY1" => Some(EdgeWeightType::Xray1),
        "XRAY2" => Some(EdgeWeightType::Xray2),
        "SPECIAL" => Some(EdgeWeightType::Special),
        _ => panic!("Unknown edge weight type: {}", value),
    }
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

/// Helper function to parse the CAPACITY header field, which simply parses the value as a usize.
///
/// # Arguments
/// * `value` - The value of the CAPACITY header field, which will be parsed as a usize.
///
/// # Returns
/// * `Some(usize)` if the value was successfully parsed as a usize.
///
/// # Panics
/// * If the value cannot be parsed as a usize, the function will panic with an error message indicating the invalid capacity value.
fn parse_capacity(value: &str) -> Option<usize> {
    Some(value.parse().expect("Invalid capacity value"))
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

/// Helper function to parse the EDGE_WEIGHT_FORMAT header field, which maps the string value to the corresponding `EdgeWeightFormat` enum variant.
///
/// # Arguments
/// * `value` - The value of the EDGE_WEIGHT_FORMAT header field, which will be parsed.
///
/// # Returns
/// * `Some(EdgeWeightFormat)` if the value corresponds to a known edge weight format and was successfully parsed.
///
/// # Panics
/// * If the value does not correspond to a known edge weight format, the function will panic with an error message indicating the unknown edge weight format.
fn parse_edge_weight_format(value: &str) -> Option<EdgeWeightFormat> {
    match value {
        "FUNCTION" => Some(EdgeWeightFormat::Function),
        "FULL_MATRIX" => Some(EdgeWeightFormat::FullMatrix),
        "UPPER_ROW" => Some(EdgeWeightFormat::UpperRow),
        "LOWER_ROW" => Some(EdgeWeightFormat::LowerRow),
        "UPPER_DIAG_ROW" => Some(EdgeWeightFormat::UpperDiagRow),
        "LOWER_DIAG_ROW" => Some(EdgeWeightFormat::LowerDiagRow),
        "UPPER_COL" => Some(EdgeWeightFormat::UpperCol),
        "LOWER_COL" => Some(EdgeWeightFormat::LowerCol),
        "UPPER_DIAG_COL" => Some(EdgeWeightFormat::UpperDiagCol),
        "LOWER_DIAG_COL" => Some(EdgeWeightFormat::LowerDiagCol),
        _ => panic!("Unknown edge weight format: {}", value),
    }
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

/// Helper function to parse the EDGE_DATA_FORMAT header field, which maps the string value to the corresponding `EdgeDataFormat` enum variant.
///
/// # Arguments
/// * `value` - The value of the EDGE_DATA_FORMAT header field, which will be parsed.
///
/// # Returns
/// * `Some(EdgeDataFormat)` if the value corresponds to a known edge data format and was successfully parsed.
///
/// # Panics
/// * If the value does not correspond to a known edge data format, the function will panic with an error message indicating the unknown edge data format.
fn parse_edge_data_format(value: &str) -> Option<EdgeDataFormat> {
    match value {
        "EDGE_LIST" => Some(EdgeDataFormat::EdgeList),
        "ADJ_LIST" => Some(EdgeDataFormat::AdjList),
        _ => panic!("Unknown edge data format: {}", value),
    }
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

/// Helper function to parse the NODE_COORD_TYPE header field, which maps the string value to the corresponding `NodeCoordType` enum variant.
///
/// # Arguments
/// * `value` - The value of the NODE_COORD_TYPE header field, which will be parsed.
///
/// # Returns
/// * `Some(NodeCoordType)` if the value corresponds to a known node coordinate type and was successfully parsed.
///
/// # Panics
/// * If the value does not correspond to a known node coordinate type, the function will panic with an error message indicating the unknown node coordinate type.
fn parse_node_coord_type(value: &str) -> Option<NodeCoordType> {
    match value {
        "TWOD_COORDS" => Some(NodeCoordType::TwoDCoords),
        "THREED_COORDS" => Some(NodeCoordType::ThreeDCoords),
        "NO_COORDS" => Some(NodeCoordType::NoCoords),
        _ => panic!("Unknown node coord type: {}", value),
    }
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

/// Helper function to parse the DISPLAY_DATA_TYPE header field, which maps the string value to the corresponding `DisplayDataType` enum variant.
///
/// # Arguments
/// * `value` - The value of the DISPLAY_DATA_TYPE header field, which will be parsed.
///
/// # Returns
/// * `Some(DisplayDataType)` if the value corresponds to a known display data type and was successfully parsed.
///
/// # Panics
/// * If the value does not correspond to a known display data type, the function will panic with an error message indicating the unknown display data type.
fn parse_display_data_type(value: &str) -> Option<DisplayDataType> {
    match value {
        "COORD_DISPLAY" => Some(DisplayDataType::CoordDisplay),
        "TWOD_DISPLAY" => Some(DisplayDataType::TwoDDisplay),
        "NO_DISPLAY" => Some(DisplayDataType::NoDisplay),
        _ => panic!("Unknown display data type: {}", value),
    }
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

// -----------------------------------------------------------------------------------------------------------------------------------------------
// Helper functions to parse individual data sections, with both panicking and error-handling versions for each field
// -----------------------------------------------------------------------------------------------------------------------------------------------

/// Helper function to parse a data section based on its type, which dispatches to the appropriate parsing function for that section type.
///
/// # Arguments
/// * `section_type` - The type of the data section being parsed, which determines which parsing function will be called to parse the lines of the section.
/// * `lines` - The lines of the data section being parsed, which will be passed to the appropriate parsing function based on the section type.
///
/// # Returns
/// * `DataSection` - The parsed data section, containing the specific data for that section type.
///
/// # Panics
/// * If the section type does not correspond to a known data section type, the function will panic with an error message indicating the unknown data section type.
/// * If the lines of the data section do not conform to the expected format for that section type, the function will panic with an error message indicating the specific issue with the line format.
/// * If the section type does not correspond to a known data section type for which parsing is implemented, the function will panic with an error message indicating that parsing for that section type is not implemented.
fn to_data_section(section_type: &DataSectionType, lines: Vec<&str>) -> DataSection {
    match section_type {
        // example given in the tsplib repository
        DataSectionType::NodeCoordSection => parse_node_coord_section(lines),
        DataSectionType::FixedEdgesSection => parse_fixed_edges_section(lines),
        DataSectionType::DisplayDataSection => parse_display_data_section(lines),
        DataSectionType::EdgeWeightSection => parse_edge_weight_section(lines),

        // other section types for which no examples exist in the tsplib repository
        DataSectionType::TourSection => unimplemented!(),
        DataSectionType::DepotSection => unimplemented!(),
        DataSectionType::DemandSection => unimplemented!(),
        DataSectionType::EdgeDataSection => unimplemented!(),
    }
}

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
fn try_to_data_section(
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
/// * `DataSection` - The parsed NODE_COORD_SECTION data section, containing either 2D or 3D coordinates based on the format of the lines.
///
/// # Panics
/// * If the first line of the NODE_COORD_SECTION is empty, the function will panic with an error message indicating that the section cannot be empty.
/// * If the first line of the NODE_COORD_SECTION does not contain either 3 or 4 values, the function will panic with an error message indicating the invalid line format in the NODE_COORD_SECTION.
fn parse_node_coord_section(lines: Vec<&str>) -> DataSection {
    // determine whether the coordinates are 2D or 3D based on the number of values in the first line
    // this will fail if there is a mix of 2D and 3D coordinates, but such a format is not valid according to the TSPLIB specification
    match lines
        .first()
        .expect("NODE_COORD_SECTION cannot be empty")
        .split_whitespace()
        .count()
    {
        3 => parse_node_coord_section_2d(lines),
        4 => parse_node_coord_section_3d(lines),
        _ => panic!("Invalid line format in NODE_COORD_SECTION: {}", lines[0]),
    }
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
/// * `DataSection` - The parsed NODE_COORD_SECTION data section, containing a vector of tuples with node indices and their corresponding x and y coordinates.
///
/// # Panics
/// * If any line in the NODE_COORD_SECTION does not contain exactly 3 values, the function will panic with an error message indicating the invalid line format in the NODE_COORD_SECTION.
/// * If any of the values in the lines cannot be parsed as the expected types (node index as usize, x and y coordinates as f64), the function will panic with an error message indicating the specific issue.
fn parse_node_coord_section_2d(lines: Vec<&str>) -> DataSection {
    let coords = lines
        .into_iter()
        .map(|line| {
            let parts = line.split_whitespace().collect::<Vec<_>>();
            if parts.len() != 3 {
                panic!("Invalid line format in NODE_COORD_SECTION: {}", line);
            }
            (
                parts[0].parse().expect("Invalid node index"),
                parts[1].parse().expect("Invalid x coordinate"),
                parts[2].parse().expect("Invalid y coordinate"),
            )
        })
        .collect::<Vec<_>>();

    DataSection::NodeCoordSection2D(coords)
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
/// * `DataSection` - The parsed NODE_COORD_SECTION data section, containing a vector of tuples with node indices and their corresponding x, y, and z coordinates.
///
/// # Panics
/// * If any line in the NODE_COORD_SECTION does not contain exactly 4 values, the function will panic with an error message indicating the invalid line format in the NODE_COORD_SECTION.
/// * If any of the values in the lines cannot be parsed as the expected types (node index as usize, x, y, and z coordinates as f64), the function will panic with an error message indicating the specific issue.
fn parse_node_coord_section_3d(lines: Vec<&str>) -> DataSection {
    let coords = lines
        .into_iter()
        .map(|line| {
            let parts = line.split_whitespace().collect::<Vec<_>>();
            if parts.len() != 4 {
                panic!("Invalid line format in NODE_COORD_SECTION: {}", line);
            }
            (
                parts[0].parse().expect("Invalid node index"),
                parts[1].parse().expect("Invalid x coordinate"),
                parts[2].parse().expect("Invalid y coordinate"),
                parts[3].parse().expect("Invalid z coordinate"),
            )
        })
        .collect::<Vec<_>>();

    DataSection::NodeCoordSection3D(coords)
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
/// * `DataSection` - The parsed FIXED_EDGES_SECTION data section, containing a vector of tuples with pairs of node indices representing fixed edges.
///
/// # Panics
/// * If any line in the FIXED_EDGES_SECTION does not contain exactly 2 values, the function will panic with an error message indicating the invalid line format in the FIXED_EDGES_SECTION.
/// * If any of the values in the lines cannot be parsed as usize node indices, the function will panic with an error message indicating the invalid node index value.
fn parse_fixed_edges_section(lines: Vec<&str>) -> DataSection {
    let edges = lines
        .into_iter()
        .map(|line| {
            let parts = line.split_whitespace().collect::<Vec<_>>();
            if parts.len() != 2 {
                panic!("Invalid line format in FIXED_EDGES_SECTION: {}", line);
            }
            (
                parts[0].parse().expect("Invalid node index"),
                parts[1].parse().expect("Invalid node index"),
            )
        })
        .collect::<Vec<_>>();
    DataSection::FixedEdgesSection(edges)
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
/// * `DataSection` - The parsed DISPLAY_DATA_SECTION data section, containing a vector of tuples with node indices and their corresponding x and y coordinates for display purposes.
///
/// # Panics
/// * If any line in the DISPLAY_DATA_SECTION does not contain exactly 3 values, the function will panic with an error message indicating the invalid line format in the DISPLAY_DATA_SECTION.
/// * If any of the values in the lines cannot be parsed as the expected types (node index as usize, x and y coordinates as f64), the function will panic with an error message indicating the specific issue.
fn parse_display_data_section(lines: Vec<&str>) -> DataSection {
    let display_data = lines
        .into_iter()
        .map(|line| {
            let parts = line.split_whitespace().collect::<Vec<_>>();
            if parts.len() != 3 {
                panic!("Invalid line format in DISPLAY_DATA_SECTION: {}", line);
            }
            (
                parts[0].parse().expect("Invalid node index"),
                parts[1].parse().expect("Invalid x coordinate"),
                parts[2].parse().expect("Invalid y coordinate"),
            )
        })
        .collect::<Vec<_>>();
    DataSection::DisplayDataSection(display_data)
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
/// * `DataSection` - The parsed EDGE_WEIGHT_SECTION data section, containing a vector of vectors of f64 values representing the edge weights.
///
/// # Panics
/// * If any value in the lines cannot be parsed as an f64, the function will panic with an error message indicating the invalid edge weight value.
fn parse_edge_weight_section(lines: Vec<&str>) -> DataSection {
    let edge_weights = lines
        .into_iter()
        .map(|line| {
            line.split_whitespace()
                .map(|part| part.parse().expect("Invalid edge weight value"))
                .collect::<Vec<f64>>()
        })
        .collect::<Vec<_>>();

    DataSection::EdgeWeightSection(edge_weights)
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
                .collect::<Result<Vec<f64>, ParseError>>()
        })
        .collect::<Result<Vec<_>, ParseError>>()?;

    Ok(DataSection::EdgeWeightSection(edge_weights))
}

/// Module containing the specific error types that can occur during parsing, using the `thiserror` crate for convenient error definitions and formatting.
pub mod errors {
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
}
