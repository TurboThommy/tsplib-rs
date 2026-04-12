use errors::ParseError;
use tsplib_core::{
    enums::{
        DataSection, DataSectionType, DisplayDataType, EdgeDataFormat, EdgeWeightFormat,
        EdgeWeightType, NodeCoordType, ProblemType,
    },
    models::TSPInstance,
};

// Internal struct to hold the parsed specification/header fields while parsing the file
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

// Internal parser state
enum ParserState {
    Header,
    Section(DataSectionType),
}

impl ParserState {
    fn new_section_from_line(&mut self, line: &str) {
        *self = match line.trim() {
            "NODE_COORD_SECTION" => ParserState::Section(DataSectionType::NodeCoordSection),
            "FIXED_EDGES_SECTION" => ParserState::Section(DataSectionType::FixedEdgesSection),
            "DISPLAY_DATA_SECTION" => ParserState::Section(DataSectionType::DisplayDataSection),
            "EDGE_WEIGHT_SECTION" => ParserState::Section(DataSectionType::EdgeWeightSection),
            _ => panic!("Unknown section type: {}", line),
        };
    }

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

// Main parsing function that takes the content of a TSP file as a string and returns a TSPInstance
pub fn parse(file_content: String) -> TSPInstance {
    let mut specification = SpecificationPart::new();
    let mut data_sections: Vec<DataSection> = Vec::new();
    let mut state = ParserState::Header;
    let mut curr_lines: Vec<&str> = Vec::new();

    for line in file_content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
    {
        match state {
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
            ParserState::Section(ref section_type) => {
                if line == "EOF" || line == "-1" {
                    data_sections.push(to_data_section(section_type, curr_lines));
                    curr_lines = Vec::new();
                    continue;
                }

                if line.contains("SECTION") {
                    data_sections.push(to_data_section(section_type, curr_lines));
                    curr_lines = Vec::new();
                    state.new_section_from_line(line);
                    continue;
                }

                curr_lines.push(line);
            }
        }
    }

    create_tsp_instance(specification, data_sections)
}

pub fn try_parse(file_content: String) -> Result<TSPInstance, ParseError> {
    let mut specification = SpecificationPart::new();
    let mut data_sections: Vec<DataSection> = Vec::new();
    let mut state = ParserState::Header;
    let mut curr_lines: Vec<&str> = Vec::new();

    for line in file_content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
    {
        match state {
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
            ParserState::Section(ref section_type) => {
                if line == "EOF" || line == "-1" {
                    data_sections.push(try_to_data_section(section_type, curr_lines)?);
                    curr_lines = Vec::new();
                    continue;
                }

                if line.contains("SECTION") {
                    data_sections.push(try_to_data_section(section_type, curr_lines)?);
                    curr_lines = Vec::new();
                    state.try_new_section_from_line(line)?;
                    continue;
                }

                curr_lines.push(line);
            }
        }
    }

    try_create_tsp_instance(specification, data_sections)
}

// Helper functions to create TSPInstance from the parsed specification and data sections, and to parse individual header fields and data sections
fn create_tsp_instance(
    specification: SpecificationPart,
    data_sections: Vec<DataSection>,
) -> TSPInstance {
    tsplib_core::models::TSPInstance {
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
        comment: Some(specification.comment),
        capacity: specification.capacity,
        edge_weight_format: specification.edge_weight_format,
        edge_data_format: specification.edge_data_format,
        node_coord_type: specification.node_coord_type,
        display_data_type: specification.display_data_type,
        data_sections,
    }
}

fn try_create_tsp_instance(
    specification: SpecificationPart,
    data_sections: Vec<DataSection>,
) -> Result<TSPInstance, ParseError> {
    let tsp_instance = tsplib_core::models::TSPInstance {
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
        comment: Some(specification.comment),
        capacity: specification.capacity,
        edge_weight_format: specification.edge_weight_format,
        edge_data_format: specification.edge_data_format,
        node_coord_type: specification.node_coord_type,
        display_data_type: specification.display_data_type,
        data_sections,
    };
    Ok(tsp_instance)
}

// Helper functions to parse individual header fields
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

fn try_parse_name(value: &str, name: &mut Option<String>) -> Result<(), ParseError> {
    *name = Some(value.to_string());
    Ok(())
}

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

fn parse_dimension(value: &str) -> Option<usize> {
    Some(value.parse().expect("Invalid dimension value"))
}

fn try_parse_dimension(value: &str, dimension: &mut Option<usize>) -> Result<(), ParseError> {
    *dimension = Some(
        value
            .parse()
            .map_err(|_| ParseError::InvalidDimensionValue(value.to_string()))?,
    );
    Ok(())
}

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

fn parse_capacity(value: &str) -> Option<usize> {
    Some(value.parse().expect("Invalid capacity value"))
}

fn try_parse_capacity(value: &str, capacity: &mut Option<usize>) -> Result<(), ParseError> {
    *capacity = Some(
        value
            .parse()
            .map_err(|_| ParseError::InvalidCapacityValue(value.to_string()))?,
    );
    Ok(())
}

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

fn parse_edge_data_format(value: &str) -> Option<EdgeDataFormat> {
    match value {
        "EDGE_LIST" => Some(EdgeDataFormat::EdgeList),
        "ADJ_LIST" => Some(EdgeDataFormat::AdjList),
        _ => panic!("Unknown edge data format: {}", value),
    }
}

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

fn parse_node_coord_type(value: &str) -> Option<NodeCoordType> {
    match value {
        "TWOD_COORDS" => Some(NodeCoordType::TwoDCoords),
        "THREED_COORDS" => Some(NodeCoordType::ThreeDCoords),
        "NO_COORDS" => Some(NodeCoordType::NoCoords),
        _ => panic!("Unknown node coord type: {}", value),
    }
}

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

fn parse_display_data_type(value: &str) -> Option<DisplayDataType> {
    match value {
        "COORD_DISPLAY" => Some(DisplayDataType::CoordDisplay),
        "TWOD_DISPLAY" => Some(DisplayDataType::TwoDDisplay),
        "NO_DISPLAY" => Some(DisplayDataType::NoDisplay),
        _ => panic!("Unknown display data type: {}", value),
    }
}

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

// Helper functions to parse individual data sections
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
        DataSectionType::TourSection => unimplemented!(),
        DataSectionType::DepotSection => unimplemented!(),
        DataSectionType::DemandSection => unimplemented!(),
        DataSectionType::EdgeDataSection => unimplemented!(),
    };
    Ok(data_section)
}

fn parse_node_coord_section(lines: Vec<&str>) -> DataSection {
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
    }
}
