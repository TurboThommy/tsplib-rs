//! Main parser crate for parsing TSP files into TSPInstance structs,
//! containing the main parsing logic and state machine to handle the different parts of the file,
//! as well as helper functions to parse individual header fields and data sections
//! and error handling for various parsing issues.
mod parser;

pub use parser::errors::ParseError;
pub use parser::{parse, try_parse};
