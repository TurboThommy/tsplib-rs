//! This module contains all the enums used in the library.
mod algorithm_type;
mod data_section;
mod data_section_type;
mod display_data_type;
mod edge_data_format;
mod edge_weight_format;
mod edge_weight_type;
mod errors;
mod node_coord_type;
mod problem_type;

pub use algorithm_type::AlgorithmType;
pub use data_section::DataSection;
pub use data_section_type::DataSectionType;
pub use display_data_type::DisplayDataType;
pub use edge_data_format::EdgeDataFormat;
pub use edge_weight_format::EdgeWeightFormat;
pub use edge_weight_type::EdgeWeightType;
pub use errors::{ConversionError, InstanceError, IoError};
pub use node_coord_type::NodeCoordType;
pub use problem_type::ProblemType;
