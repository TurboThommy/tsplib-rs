//! Response models used in the REST API endpoints.
use serde::Serialize;
use tsplib_core::enums::EdgeWeightType;
use tsplib_parser::SpecificationPart;

use crate::errors::ServerError;

/// Enum to specify the type of node display for visualization purposes,
/// based on the edge weight type and coordinate information in the TSP file header.
#[derive(Debug, Serialize)]
pub enum NodeDisplayType {
    TwoD,
    ThreeD,
    Geo,
}

/// Response struct for the GET /problems/{problemId} endpoint, containing the relevant metadata for a TSP problem instance
#[derive(Debug, Serialize)]
pub(crate) struct ProblemDescriptionResponse {
    pub problem_id: String,
    pub name: String,
    pub dimension: usize,
    pub node_display_type: NodeDisplayType,
}

impl ProblemDescriptionResponse {
    /// Try to create a ProblemDescriptionResponse from the given problem ID and specification part,
    /// which contains the parsed header fields from the TSP file.
    ///
    /// # Arguments
    /// * `problem_id` - The unique identifier for the problem instance, typically derived from the filename without extension.
    /// * `specification` - The SpecificationPart struct containing the parsed header fields from the TSP file,
    ///   including name, dimension, edge weight type, etc.
    ///
    /// # Returns
    /// * `Result<ProblemDescriptionResponse, ServerError>` - Ok with a new ProblemDescriptionResponse
    ///   if the necessary metadata is present and valid, or an Err with a ServerError if there
    pub fn try_from_specification(
        problem_id: String,
        specification: &SpecificationPart,
    ) -> Result<Self, ServerError> {
        // Determine the node display type based on the edge weight type and coordinate information in the specification.
        let node_display_type = match specification
            .edge_weight_type
            .ok_or(ServerError::MetadataParseError(problem_id.to_string()))?
        {
            // geo problems
            EdgeWeightType::Geo => NodeDisplayType::Geo,

            // problems with 2D coordinates
            EdgeWeightType::Euc2D
            | EdgeWeightType::Max2D
            | EdgeWeightType::Man2D
            | EdgeWeightType::Ceil2D
            | EdgeWeightType::Att => NodeDisplayType::TwoD,

            // problems with 3D coordinates
            EdgeWeightType::Euc3D | EdgeWeightType::Max3D | EdgeWeightType::Man3D => {
                NodeDisplayType::ThreeD
            }

            // explicit edge weight problems, we will assume 2D coordinates for visualization purposes,
            // as there is no coordinate information available
            EdgeWeightType::Explicit => NodeDisplayType::TwoD,

            // Unsupported for now
            _ => {
                return Err(ServerError::UnsupportedEdgeWeightType(
                    problem_id.to_string(),
                ));
            }
        };

        // Extract the name and return an error if it is missing
        let name = match &specification.name {
            Some(name) => name.clone(),
            None => return Err(ServerError::MetadataParseError(problem_id.to_string())),
        };

        // Extract the dimension and return an error if it is missing
        let dimension = match specification.dimension {
            Some(dimension) => dimension,
            None => return Err(ServerError::MetadataParseError(problem_id.to_string())),
        };

        // Create and return the ProblemDescriptionResponse with the extracted metadata
        Ok(ProblemDescriptionResponse {
            problem_id,
            name,
            dimension,
            node_display_type,
        })
    }
}

/// Response struct for the GET /solutions/{problemId} endpoint, containing the solution cost for a given problem instance
#[derive(Debug, Serialize)]
pub(crate) struct SolutionResponse {
    pub id: String,
    pub cost: i64,
}
