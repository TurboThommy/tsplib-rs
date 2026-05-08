use tsplib_core::models::{ProblemInstance, TspSolution};

use crate::{TspSolver, errors::SolverError};

/// The Held-Karp algorithm is a dynamic programming approach to solve the TSP problem.
pub struct HeldCarp {
    /// The maximum dimension (number of nodes) that the Held-Karp algorithm can handle.
    /// The table used by the algorithm grows exponentially with the number of nodes,
    /// the memory usage is O(n * 2^n) * size of an integer
    pub max_dimension: usize,
}

impl TspSolver for HeldCarp {
    fn try_solve(
        &self,
        problem: &ProblemInstance,
        start_node: usize,
    ) -> Result<TspSolution, SolverError> {
        // TODO: add support for fixed edges
        let n = problem.nodes.len();

        // TODO: perhaps add a maximum of 64 nodes, since the bitmask used is i64
        // check if dimension is within limits
        if n > self.max_dimension {
            return Err(SolverError::DimensionExceeded);
        }

        // number of subsets of nodes = 2^n
        let size = 1 << n;

        // dp table with size 2^n x n (subsets x number of nodes)
        let mut dp = vec![vec![i64::MAX; n]; size];

        // table to reconstruct the path, stores the previous node for each state
        let mut parent = vec![vec![usize::MAX; n]; size];

        // vectors use 0-based indexing, but TSP nodes are 1-based,
        // so we adjust the start_node index
        let start_idx = start_node - 1;

        // the bitmask for the set of visited nodes, initially only the start node is visited
        let start_mask = 1 << start_idx;

        // base case: starting at the start_node with only that node visited has a cost of 0
        dp[start_mask][start_idx] = 0;

        // build table by iterating over all subsets of nodes
        // start from start_mask (lower numbers represent subsets that don't include the start node, which can be skipped)
        for s in start_mask..size {
            // skip subsets which don't include the start node
            if (s & start_mask) == 0 {
                continue;
            }

            // for each node j in the subset s (the last node visited in the path represented by subset s)
            for j in 0..n {
                // if j is not in the subset s, skip
                if (s & (1 << j)) == 0 {
                    continue;
                }

                // if the cost to reach node j with subset s is infinity, skip
                if dp[s][j] == i64::MAX {
                    continue;
                }

                // try to extend the path from node j to a new node k that is not in subset s
                for k in 0..n {
                    // if k is already in subset s, skip
                    if (s & (1 << k)) != 0 {
                        continue;
                    }

                    // new subset with node k added
                    let s_next = s | (1 << k);

                    // calculate the cost to extend the path from j to k
                    let cost = dp[s][j] + problem.adjacency_matrix[j][k] as i64;

                    // update the tables if the new cost is lower
                    if cost < dp[s_next][k] {
                        dp[s_next][k] = cost;
                        parent[s_next][k] = j;
                    }
                }
            }
        }

        // find the minimum cost to return to the start node after visiting all nodes
        // all nodes visited
        let full_mask = (1 << n) - 1;

        // get the minimum cost
        let minimal_cost = (0..n)
            .filter(|&j| j != start_idx) // exclude start node
            .filter(|&j| dp[full_mask][j] != i64::MAX) // only consider valid paths
            .map(|j| dp[full_mask][j] + problem.adjacency_matrix[j][start_idx] as i64) // add cost to return to start node
            .min() // find the minimum cost
            .ok_or(SolverError::NoSolution)?;

        let mut last_node = (0..n)
            .filter(|&j| j != start_idx)
            .filter(|&j| dp[full_mask][j] != i64::MAX)
            .min_by_key(|&j| dp[full_mask][j] + problem.adjacency_matrix[j][start_idx] as i64)
            .unwrap(); // we know there is at least one valid path, so unwrap is safe here

        // reconstruct the path
        let mut tour: Vec<usize> = Vec::new();
        let mut current_mask = full_mask;

        while last_node != start_idx {
            tour.push(last_node + 1); // convert back to 1-based indexing
            let previous_node = parent[current_mask][last_node];
            current_mask ^= 1 << last_node; // remove last_node from the mask
            last_node = previous_node;
        }
        tour.push(start_node); // add the start node at the end to complete the tour
        tour.reverse(); // reverse the tour to get the correct order

        Ok(TspSolution {
            tour,
            cost: minimal_cost,
        })
    }
}
