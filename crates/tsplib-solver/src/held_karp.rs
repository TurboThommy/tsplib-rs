//! A module containing the implementation of the Held-Karp algorithm for solving the Traveling Salesman Problem (TSP).

use std::collections::{HashMap, HashSet};

use tsplib_core::{
    context::ExecutionContext,
    models::{TspSolution, TsplibInstance},
};

use crate::{TspSolver, errors::SolverError};

/// The Held-Karp algorithm is a dynamic programming approach to solve the TSP problem.
pub struct HeldKarp {
    /// The maximum dimension (number of nodes) that the Held-Karp algorithm can handle.
    /// The table used by the algorithm grows exponentially with the number of nodes,
    /// the memory usage is O(n * 2^n) * size of an integer
    max_dimension: usize,
}

type DpTable = Vec<Vec<i64>>;
type ParentTable = Vec<Vec<usize>>;

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

    /// Builds the dynamic programming (DP) and parent tables for the Held-Karp algorithm based on the given problem instance and fixed edge constraints.
    ///
    /// # Arguments
    /// * `problem` - A reference to the `ProblemInstance` representing the TSP problem to be solved.
    /// * `n` - The number of nodes in the problem instance.
    /// * `start_idx` - The index of the starting node (0-based).
    /// * `fixed_edge_map` - A `HashMap` mapping each node ID to its fixed edge target (if it has one).
    /// * `fixed_edge_targets` - A `HashSet` containing the IDs of all nodes that are targets of fixed edges.
    /// * `ctx` - An `ExecutionContext` providing additional information and resources for the solver (e.g., time limits, logging, etc.).
    ///
    /// # Returns
    /// * `Result<(DpTable, ParentTable), SolverError>` - On success, returns a tuple containing the DP table and the parent table used for reconstructing the optimal tour.
    ///   On failure, returns a `SolverError` indicating the reason for the failure
    ///   (e.g., distance retrieval error, invalid problem instance, etc.).
    fn try_build_tables(
        &self,
        problem: &TsplibInstance,
        n: usize,
        start_idx: usize,
        fixed_edge_map: &HashMap<usize, usize>,
        fixed_edge_targets: &HashSet<usize>,
        ctx: ExecutionContext,
    ) -> Result<(DpTable, ParentTable), SolverError> {
        // number of subsets of nodes = 2^n
        let size = 1 << n;

        // dp table with size 2^n x n (subsets x number of nodes)
        let mut dp = vec![vec![i64::MAX; n]; size];
        if ctx.is_cancelled() {
            return Err(SolverError::Cancelled);
        }

        // table to reconstruct the path, stores the previous node for each state
        let mut parent = vec![vec![usize::MAX; n]; size];
        if ctx.is_cancelled() {
            return Err(SolverError::Cancelled);
        }

        // the bitmask for the set of visited nodes, initially only the start node is visited
        let start_mask = 1 << start_idx;

        // base case: starting at the start_node with only that node visited has a cost of 0
        dp[start_mask][start_idx] = 0;

        // build dp by iterating over all subsets of nodes
        // start from start_mask (lower numbers represent subsets that don't include the start node, which can be skipped)
        for s in start_mask..size {
            // check for cancellation every 1000 iterations to avoid excessive overhead
            if s % 1000 == 0 && ctx.is_cancelled() {
                return Err(SolverError::Cancelled);
            }

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
                        if fixed_edge_targets.contains(&(k + 1)) {
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

        Ok((dp, parent))
    }

    /// Finds the minimal tour cost and the last node in the optimal tour after visiting all nodes, based on the completed DP table.
    ///
    /// # Arguments
    /// * `problem` - A reference to the `ProblemInstance` representing the TSP problem to be solved.
    /// * `dp` - A reference to the completed DP table containing the minimum costs for each subset of nodes and last node.
    /// * `n` - The number of nodes in the problem instance.
    /// * `start_idx` - The index of the starting node (0-based).
    /// * `full_mask` - The bitmask representing the subset of all nodes visited (i.e., when all nodes are visited).
    /// * `ctx` - An `ExecutionContext` providing additional information and resources for the solver (e.g., time limits, logging, etc.).
    ///
    /// # Returns
    /// * `Result<(i64, usize), SolverError>` - On success, returns a tuple containing the minimal tour cost and the index of the last node in the optimal tour.
    ///   On failure, returns a `SolverError` indicating the reason for the failure
    ///   (e.g., no solution found, distance retrieval error, etc.).
    fn try_find_minimal_tour(
        &self,
        problem: &TsplibInstance,
        dp: &DpTable,
        n: usize,
        start_idx: usize,
        full_mask: usize,
        ctx: ExecutionContext,
    ) -> Result<(i64, usize), SolverError> {
        // check for cancellation before finding the minimal tour
        if ctx.is_cancelled() {
            return Err(SolverError::Cancelled);
        }

        // find the minimum cost to return to the start node after visiting all nodes
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

        let (last_node, minimal_cost) = costs
            .iter()
            .min_by_key(|(_, cost)| *cost)
            .map(|&(j, cost)| (j, cost))
            .ok_or(SolverError::NoSolution)?;

        Ok((minimal_cost, last_node))
    }

    /// Reconstructs the optimal tour based on the parent table generated during the DP table construction.
    ///
    /// # Arguments
    /// * `parent` - A reference to the parent table containing the previous node for each state in the DP table.
    /// * `start_node` - The ID of the starting node (1-based).
    /// * `start_idx` - The index of the starting node (0-based).
    /// * `full_mask` - The bitmask representing the subset of all nodes visited (i.e., when all nodes are visited).
    /// * `last_node` - The index of the last node in the optimal tour (0-based).
    ///
    /// # Returns
    /// * `Result<Vec<usize>, SolverError>` - On success, returns a vector containing the sequence of node IDs in the optimal tour, starting and ending at the specified start node.
    ///   On failure, returns a `SolverError` indicating the reason for the failure
    fn try_reconstruct_tour(
        &self,
        parent: &ParentTable,
        start_node: usize,
        start_idx: usize,
        full_mask: usize,
        mut last_node: usize,
        ctx: ExecutionContext,
    ) -> Result<Vec<usize>, SolverError> {
        // check for cancellation before reconstructing the tour
        if ctx.is_cancelled() {
            return Err(SolverError::Cancelled);
        }

        let mut tour: Vec<usize> = Vec::new();
        let mut current_mask = full_mask;

        // reconstruct the path
        while last_node != start_idx {
            tour.push(last_node + 1); // convert back to 1-based indexing
            let previous_node = parent[current_mask][last_node];
            current_mask ^= 1 << last_node; // remove last_node from the mask
            last_node = previous_node;

            // invalid entry in the parent table
            // this can only happen if the dp table was not properly filled
            if last_node == usize::MAX {
                return Err(SolverError::InvalidParentTable);
            }
        }
        tour.push(start_node); // add the start node at the end to complete the tour
        tour.reverse(); // reverse the tour to get the correct order

        Ok(tour)
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
    /// * `ctx` - An `ExecutionContext` providing additional information and resources for the solver (e.g., time limits, logging, etc.).
    ///
    /// # Returns
    /// * `Result<TspSolution, SolverError>` - On success, returns a `TspSolution` containing the optimal tour and its total cost.
    ///   On failure, returns a `SolverError` indicating the reason for the failure
    ///   (e.g., invalid start node, dimension exceeded, no solution found, etc.).
    fn try_solve_with_context(
        &self,
        problem: &TsplibInstance,
        start_node: usize,
        ctx: ExecutionContext,
    ) -> Result<TspSolution, SolverError> {
        // number of nodes in the problem
        let n = problem.nodes.len();

        // check if dimension is within limits
        if n > self.max_dimension {
            return Err(SolverError::DimensionExceeded);
        }

        // check if the problem instance and start node are valid
        // and get the fixed edge map and targets for quick lookup
        let (fixed_edge_map, fixed_edge_targets) =
            self.try_check_problem_validity(problem, start_node)?;

        // vectors use 0-based indexing, but TSP nodes are 1-based,
        // so the start_node index has to be adjusted
        let start_idx = start_node - 1;

        // the bitmask for the set of visited nodes when all nodes are visited
        let full_mask = (1 << n) - 1;

        // check for cancellation before starting the main computation
        if ctx.is_cancelled() {
            return Err(SolverError::Cancelled);
        }

        // build the dp and parent tables
        let (dp, parent) = self.try_build_tables(
            problem,
            n,
            start_idx,
            &fixed_edge_map,
            &fixed_edge_targets,
            ctx,
        )?;

        // find the minimal tour cost and the last node in the optimal tour
        let (minimal_cost, last_node) =
            self.try_find_minimal_tour(problem, &dp, n, start_idx, full_mask, ctx)?;

        // reconstruct the optimal tour based on the last node and the parent table
        let tour =
            self.try_reconstruct_tour(&parent, start_node, start_idx, full_mask, last_node, ctx)?;

        Ok(TspSolution {
            tour,
            cost: minimal_cost,
        })
    }
}
