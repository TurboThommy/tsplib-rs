//! A module containing the implementation of the Held-Karp algorithm for solving the Traveling Salesman Problem (TSP).

use std::collections::{HashMap, HashSet};

use tsplib_core::models::{ProblemInstance, TspSolution};

use crate::{TspSolver, errors::SolverError};

/// The Held-Karp algorithm is a dynamic programming approach to solve the TSP problem.
pub struct HeldKarp {
    /// The maximum dimension (number of nodes) that the Held-Karp algorithm can handle.
    /// The table used by the algorithm grows exponentially with the number of nodes,
    /// the memory usage is O(n * 2^n) * size of an integer
    max_dimension: usize,
}

impl HeldKarp {
    /// Creates a new instance of the HeldKarp solver with the specified maximum dimension.
    /// The maximum dimension is necessary to prevent excessive memory usage, as the Held-Karp algorithm has exponential space complexity.
    /// # Arguments
    /// * `max_dimension` - The maximum number of nodes that the Held-Karp algorithm can handle.
    ///   Must be less than or equal to 64 due to bitmask limitations.
    ///
    /// # Returns
    /// * `Result<Self, SolverError>` - On success, returns an instance of the HeldKarp solver.
    ///   On failure, returns a `SolverError` indicating the reason for the failure
    ///   (e.g., if the provided maximum dimension exceeds the allowed limit).
    pub fn try_new(max_dimension: usize) -> Result<Self, SolverError> {
        if max_dimension > 64 {
            return Err(SolverError::HeldKarpInvalidDimension(max_dimension));
        }
        Ok(Self { max_dimension })
    }
}

impl TspSolver for HeldKarp {
    /// Solves the TSP problem using the Held-Karp algorithm, starting from the specified node.
    /// The algorithm uses dynamic programming to find the optimal tour by building up solutions for subsets of nodes.
    /// It also respects fixed edges if they exist in the problem instance.
    ///
    /// # Arguments
    /// * `problem` - A reference to the `ProblemInstance` representing the TSP problem to be solved.
    /// * `start_node` - The ID of the node from which the tour should start.
    ///
    /// # Returns
    /// * `Result<TspSolution, SolverError>` - On success, returns a `TspSolution` containing the optimal tour and its total cost.
    ///   On failure, returns a `SolverError` indicating the reason for the failure
    ///   (e.g., invalid start node, dimension exceeded, no solution found, etc.).
    fn try_solve(
        &self,
        problem: &ProblemInstance,
        start_node: usize,
    ) -> Result<TspSolution, SolverError> {
        // number of nodes in the problem
        let n = problem.nodes.len();

        // check if dimension is within limits
        if n > self.max_dimension {
            return Err(SolverError::DimensionExceeded);
        }

        // TODO: refactor: move the ckecks for start_node and fixed edges to a separate function, since they are needed for all solvers
        // check if start_node is valid
        if !problem.nodes.iter().any(|n| n.id == start_node) {
            return Err(SolverError::InvalidStartNode);
        }

        // collect all fixed edges and their targets for quick lookup
        let fixed_edges = problem.fixed_edges.iter().flatten().collect::<Vec<_>>();
        let fixed_edges_targets = fixed_edges
            .iter()
            .map(|(_, to)| *to)
            .collect::<HashSet<usize>>();

        let fixed_edge_map = fixed_edges
            .iter()
            .map(|(from, to)| (*from, *to))
            .collect::<HashMap<usize, usize>>();

        // check if start_node is target of a fixed edge
        if fixed_edges_targets.contains(&start_node) {
            return Err(SolverError::StartNodeIsFixedEdgeTarget(start_node));
        }

        // check if any node has multiple fixed edges
        let max_fixed_edges = fixed_edges
            .iter()
            .fold(HashMap::new(), |mut acc, (from, _)| {
                *acc.entry(*from).or_insert(0) += 1;
                acc
            })
            .into_iter()
            .find(|(_, count)| *count > 1);

        if let Some((node_id, _)) = max_fixed_edges {
            return Err(SolverError::MultipleFixedEdges(node_id));
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

                    // check for fixed edge from j to k
                    if let Some(&forced_k) = fixed_edge_map.get(&(j + 1)) {
                        // if there is a fixed edge from j to forced_k, we can only consider that edge
                        if k != forced_k - 1 {
                            continue;
                        }
                    } else {
                        // if there is no fixed edge from j, we cannot consider nodes that are targets of fixed edges
                        if fixed_edges_targets.contains(&(k + 1)) {
                            continue;
                        }
                    }

                    // new subset with node k added
                    let s_next = s | (1 << k);

                    // calculate the cost to extend the path from j to k
                    let cost = dp[s][j] + problem.try_get_distance(j + 1, k + 1)? as i64;

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
        let costs = (0..n)
            .filter(|&j| j != start_idx)
            .filter(|&j| dp[full_mask][j] != i64::MAX)
            .map(|j| {
                Ok::<_, SolverError>((
                    j,
                    dp[full_mask][j] + problem.try_get_distance(j + 1, start_idx + 1)? as i64,
                ))
            })
            .collect::<Result<Vec<_>, SolverError>>()?;

        let minimal_cost = costs
            .iter()
            .map(|(_, cost)| *cost)
            .min()
            .ok_or(SolverError::NoSolution)?;

        let mut last_node = costs
            .iter()
            .min_by_key(|(_, cost)| *cost)
            .map(|(j, _)| *j)
            .unwrap();

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
