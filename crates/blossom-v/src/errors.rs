use thiserror::Error;

#[derive(Error, Debug)]
pub enum BlossomVError {
    #[error("Blossom V failed because node count must be a positive even number")]
    InvalidNodeCount,
    #[error("Blossom V failed because edge node index was out of bounds: {0} -- {1}")]
    EdgeNodeOutOfBounds(usize, usize),
    #[error("Blossom V failed with status code {0}")]
    SolverFailed(i32),
    #[error("Blossom V returned invalid mate for node {0}: {1}")]
    InvalidMate(usize, i32),
}
