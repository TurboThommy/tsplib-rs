//! A Rust wrapper around the Blossom V algorithm for solving the minimum weight perfect matching problem in general graphs.

mod errors;

pub use errors::BlossomVError;

/// Solves the minimum weight perfect matching problem using the Blossom V algorithm.
///
/// # Arguments
/// * `node_count` - The number of nodes in the graph. Must be even and greater than zero.
/// * `edges` - A slice of tuples representing the edges in the graph. Each tuple should contain the source node index,
///   target node index, and edge weight. Node indices must be in the range [0, node_count - 1].
///
/// # Returns
/// * `Result<Vec<(usize, usize)>, BlossomVError>` - On success, returns a vector of tuples representing the matched pairs of nodes.
///   On failure, returns a `BlossomVError` indicating the reason for the failure.
pub fn try_solve_min_weight_perfect_matching(
    node_count: usize,
    edges: &[(usize, usize, i32)],
) -> Result<Vec<(usize, usize)>, BlossomVError> {
    // check that node_count is even and not zero
    if node_count == 0 || !node_count.is_multiple_of(2) {
        return Err(BlossomVError::InvalidNodeCount);
    }

    // check that all edge node indices are within bounds
    for &(u, v, _) in edges {
        if u >= node_count || v >= node_count {
            return Err(BlossomVError::EdgeNodeOutOfBounds(u, v));
        }
    }

    // edge source nodes
    let from = edges.iter().map(|&(u, _, _)| u as i32).collect::<Vec<_>>();

    // edge target nodes
    let to = edges.iter().map(|&(_, v, _)| v as i32).collect::<Vec<_>>();

    // edge weights
    let weight = edges
        .iter()
        .map(|&(_, _, weight)| weight)
        .collect::<Vec<_>>();

    // output array for the mate of each node, initialized to -1 (indicating unmatched)
    let mut out_mate: Vec<i32> = vec![-1; node_count];

    // call the blossom v solver
    let status = unsafe {
        blossom_v_sys::blossom_v_solve(
            node_count as i32,
            edges.len() as i32,
            from.as_ptr(),
            to.as_ptr(),
            weight.as_ptr(),
            out_mate.as_mut_ptr(),
        )
    };

    // check if the solver succeeded
    if status != 0 {
        return Err(BlossomVError::SolverFailed(status));
    }

    // construct the matching from the output mate array
    let mut matching = Vec::with_capacity(node_count / 2);

    for (node, &mate) in out_mate.iter().enumerate() {
        // check that the mate is a valid node index or -1 (indicating unmatched)
        if mate < 0 || mate as usize >= node_count {
            return Err(BlossomVError::InvalidMate(node, mate));
        }

        let mate = mate as usize;

        // to avoid duplicates, only include the edge if the current node index is less than its mate
        if node < mate {
            matching.push((node, mate));
        }
    }

    Ok(matching)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solve_min_weight_perfect_matching() {
        let node_count = 4;
        let edges = vec![
            (0, 1, 10),
            (0, 2, 1),
            (0, 3, 10),
            (1, 2, 10),
            (1, 3, 1),
            (2, 3, 10),
        ];

        let matching = try_solve_min_weight_perfect_matching(node_count, &edges).unwrap();

        assert_eq!(matching.len(), 2);

        assert!(matching.contains(&(0, 2)));
        assert!(matching.contains(&(1, 3)));
    }

    #[test]
    fn test_invalid_node_count() {
        let result = try_solve_min_weight_perfect_matching(3, &[]);

        assert!(matches!(result, Err(BlossomVError::InvalidNodeCount)));
    }
}
