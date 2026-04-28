pub mod errors;
mod sections;
mod specification;

use errors::ParseError;
use sections::try_to_data_section;
use specification::try_parse_header_line;
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

/// Parses the content of a TSP file and returns a TSPInstance.
///
/// Convenience wrapper around `try_parse` that panics on error.
/// Use `try_parse` directly if you need error handling.
///
/// # Arguments
/// * `file_content` - A string containing the content of the TSP file to be parsed.
///
/// # Returns
/// * `TSPInstance` - The parsed TSP instance containing the specification and data sections from the file.
///
/// # Panics
/// * If the file content cannot be parsed successfully.
pub fn parse(file_content: String) -> TSPInstance {
    try_parse(file_content).expect("Failed to parse TSP file content")
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

    // Handle the case where the file ends without 'EOF' or '-1', but there are still lines in the current data section that need to be saved
    if let ParserState::Section(section_type) = state
        && !curr_lines.is_empty()
    {
        data_sections.push(try_to_data_section(&section_type, curr_lines)?)
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
