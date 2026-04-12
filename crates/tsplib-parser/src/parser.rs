use std::fs;
use tsplib_core::{
    enums::{
        DataSection, DataSectionType, DisplayDataType, EdgeDataFormat, EdgeWeightFormat,
        EdgeWeightType, NodeCoordType, ProblemType,
    },
    models::TSPInstance,
};

fn read_tsp_file(file_path: &str) -> String {
    fs::read_to_string(file_path).expect("Unable to read file")
}

enum ParserState {
    Header,
    Section(DataSectionType),
}

pub fn parse_tsp_file(file_path: &str) -> TSPInstance {
    let file_content = read_tsp_file(file_path);

    let mut name: Option<String> = None;
    let mut problem_type: Option<ProblemType> = None;
    let mut dimension: Option<usize> = None;
    let mut edge_weight_type: Option<EdgeWeightType> = None;
    let mut comment: Vec<String> = Vec::new();
    let mut capacity: Option<usize> = None;
    let mut edge_weight_format: Option<EdgeWeightFormat> = None;
    let mut edge_data_format: Option<EdgeDataFormat> = None;
    let mut node_coord_type: Option<NodeCoordType> = None;
    let mut display_data_type: Option<DisplayDataType> = None;
    let mut data_sections: Vec<DataSection> = Vec::new();

    let mut state = ParserState::Header;
    let mut curr_lines: Vec<&str> = Vec::new();

    for line in file_content.lines() {
        match state {
            ParserState::Header => {
                if line.contains(':') {
                    let parts = line.split(':').map(|s| s.trim()).collect::<Vec<_>>();
                    if parts.len() != 2 {
                        panic!("Invalid line format: {}", line);
                    }

                    match parts[0] {
                        "NAME" => name = Some(parts[1].to_string()),
                        "TYPE" => {
                            problem_type = match parts[1] {
                                "TSP" => Some(ProblemType::TSP),
                                "TSP (M.~Hofmeister)" => Some(ProblemType::TSP), // special case for some wrongly formatted files in the tsplib repo
                                "ATSP" => Some(ProblemType::ATSP),
                                "SOP" => Some(ProblemType::SOP),
                                "HCP" => Some(ProblemType::HCP),
                                "CVRP" => Some(ProblemType::CVRP),
                                _ => panic!("Unknown problem type: {}", parts[1]),
                            }
                        }
                        "DIMENSION" => {
                            dimension = Some(parts[1].parse().expect("Invalid dimension value"))
                        }
                        "EDGE_WEIGHT_TYPE" => {
                            edge_weight_type = match parts[1] {
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
                                _ => panic!("Unknown edge weight type: {}", parts[1]),
                            }
                        }
                        "COMMENT" => comment.push(parts[1].to_string()),
                        "CAPACITY" => {
                            capacity = Some(parts[1].parse().expect("Invalid capacity value"))
                        }
                        "EDGE_WEIGHT_FORMAT" => {
                            edge_weight_format = match parts[1] {
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
                                _ => panic!("Unknown edge weight format: {}", parts[1]),
                            }
                        }
                        "EDGE_DATA_FORMAT" => {
                            edge_data_format = match parts[1] {
                                "EDGE_LIST" => Some(EdgeDataFormat::EdgeList),
                                "ADJ_LIST" => Some(EdgeDataFormat::AdjList),
                                _ => panic!("Unknown edge data format: {}", parts[1]),
                            }
                        }
                        "NODE_COORD_TYPE" => {
                            node_coord_type = match parts[1] {
                                "TWOD_COORDS" => Some(NodeCoordType::TwoDCoords),
                                "THREED_COORDS" => Some(NodeCoordType::ThreeDCoords),
                                "NO_COORDS" => Some(NodeCoordType::NoCoords),
                                _ => panic!("Unknown node coord type: {}", parts[1]),
                            }
                        }
                        "DISPLAY_DATA_TYPE" => {
                            display_data_type = match parts[1] {
                                "COORD_DISPLAY" => Some(DisplayDataType::CoordDisplay),
                                "TWOD_DISPLAY" => Some(DisplayDataType::TwoDDisplay),
                                "NO_DISPLAY" => Some(DisplayDataType::NoDisplay),
                                _ => panic!("Unknown display data type: {}", parts[1]),
                            }
                        }
                        _ => panic!("Unknown header field: {}", parts[0]),
                    }
                } else if line.contains("SECTION") {
                    state = match line.trim() {
                        "NODE_COORD_SECTION" => {
                            ParserState::Section(DataSectionType::NodeCoordSection(
                                node_coord_type.clone().unwrap_or(NodeCoordType::NoCoords),
                            ))
                        }
                        "FIXED_EDGES_SECTION" => {
                            ParserState::Section(DataSectionType::FixedEdgesSection)
                        }
                        "DISPLAY_DATA_SECTION" => {
                            ParserState::Section(DataSectionType::DisplayDataSection)
                        }
                        "EDGE_WEIGHT_SECTION" => {
                            ParserState::Section(DataSectionType::EdgeWeightSection)
                        }
                        _ => panic!("Unknown section type: {}", line),
                    };
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

                    state = match line {
                        "NODE_COORD_SECTION" => {
                            ParserState::Section(DataSectionType::NodeCoordSection(
                                node_coord_type.clone().unwrap_or(NodeCoordType::NoCoords),
                            ))
                        }
                        "FIXED_EDGES_SECTION" => {
                            ParserState::Section(DataSectionType::FixedEdgesSection)
                        }
                        "DISPLAY_DATA_SECTION" => {
                            ParserState::Section(DataSectionType::DisplayDataSection)
                        }
                        "EDGE_WEIGHT_SECTION" => {
                            ParserState::Section(DataSectionType::EdgeWeightSection)
                        }
                        _ => panic!("Unknown section type: {}", line),
                    };
                    continue;
                }

                curr_lines.push(line);
            }
        }
    }

    tsplib_core::models::TSPInstance {
        name: name.expect("Missing required field: NAME"),
        problem_type: problem_type.expect("Missing required field: TYPE"),
        dimension: dimension.expect("Missing required field: DIMENSION"),
        edge_weight_type: edge_weight_type.expect("Missing required field: EDGE_WEIGHT_TYPE"),
        comment: Some(comment),
        capacity,
        edge_weight_format,
        edge_data_format,
        node_coord_type,
        display_data_type,
        data_sections,
    }
}

fn to_data_section(section_type: &DataSectionType, lines: Vec<&str>) -> DataSection {
    match section_type {
        DataSectionType::NodeCoordSection(_) => {
            match lines
                .first()
                .expect("NODE_COORD_SECTION cannot be empty")
                .split_whitespace()
                .count()
            {
                3 => {
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
                4 => {
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
                _ => panic!("Invalid line format in NODE_COORD_SECTION: {}", lines[0]),
            }
        }
        DataSectionType::FixedEdgesSection => {
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
        DataSectionType::DisplayDataSection => {
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
        DataSectionType::EdgeWeightSection => {
            // read all lines, split by whitespace, parse as f64 and collect into a flat vector
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

        DataSectionType::TourSection => unimplemented!(),
        DataSectionType::DepotSection => unimplemented!(),
        DataSectionType::DemandSection => unimplemented!(),
        DataSectionType::EdgeDataSection => unimplemented!(),
    }
}
