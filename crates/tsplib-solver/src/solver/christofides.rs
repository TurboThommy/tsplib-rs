use std::collections::HashSet;

use tsplib_core::models::{TspSolution, TsplibInstance};

use crate::{PerfectMatchingAlgorithm, TspSolver, errors::SolverError, matcher::GreedyMatching};

pub struct Christofides {}

impl Christofides {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for Christofides {
    fn default() -> Self {
        Self::new()
    }
}

impl TspSolver for Christofides {
    fn try_solve_with_context(
        &self,
        problem: &TsplibInstance,
        start_node: usize,
        ctx: tsplib_core::context::ExecutionContext,
    ) -> Result<tsplib_core::models::TspSolution, crate::errors::SolverError> {
        // currently the christofides implementation does not support fixed edges
        if problem.fixed_edges.is_some() {
            return Err(SolverError::FixedEdgesNotSupported);
        }

        // check problem validity
        self.try_check_problem_validity(problem, start_node)?;

        // check for cancellation before starting the MST computation, as it can be expensive for large instances
        if ctx.is_cancelled() {
            return Err(SolverError::Cancelled);
        }

        // get the minimum spanning tree
        // TODO: make the mst algorithm configurable (e.g. via ctx)
        let mst = problem.try_get_mst_kruskal()?;

        // check for cancellation after MST computation again
        if ctx.is_cancelled() {
            return Err(SolverError::Cancelled);
        }

        // find the odd degree vertices in the MST
        let odd_vertices = mst
            .get_degrees()
            .iter()
            .filter(|(_, degree)| !degree.is_multiple_of(2))
            .map(|(&node_id, _)| node_id)
            .collect::<Vec<_>>();

        // compute a perfect matching on the odd degree vertices
        // TODO: make the perfect matching algorithm configurable (e.g. via ctx)
        let matcher = GreedyMatching::new();
        let matching = matcher.try_compute(&odd_vertices, problem)?;

        // check for cancellation after perfect matching computation
        if ctx.is_cancelled() {
            return Err(SolverError::Cancelled);
        }

        // combine the edges of the MST and the perfect matching to create a multigraph
        let mut multigraph = mst.clone();
        multigraph.edges.extend(matching);

        // check for cancellation before finding the Eulerian tour
        if ctx.is_cancelled() {
            return Err(SolverError::Cancelled);
        }

        // find an Eulerian tour in the multigraph
        let eulerian_circuit = multigraph.try_get_eulerian_circuit()?;

        // check for cancellation before shortcutting the Eulerian circuit
        if ctx.is_cancelled() {
            return Err(SolverError::Cancelled);
        }

        // find the tsp tour
        let tsp_tour = shortcut_eulerian_circuit(&eulerian_circuit);

        // rotate the tour so that it starts with the specified start node
        let tsp_tour = try_rotate_tour_to_start_node(&tsp_tour, start_node)?;

        // compute the total cost of the tour
        let tour_cost = try_calculate_tour_cost(&tsp_tour, problem)?;

        Ok(TspSolution {
            tour: tsp_tour,
            cost: tour_cost,
        })
    }
}

/// Shortcuts an Eulerian circuit to create a TSP tour by skipping already visited nodes while traversing the circuit.
///
/// # Arguments
/// * `circuit` - A slice of node IDs representing the Eulerian circuit
///
/// # Returns
/// * `Vec<usize>` - A vector of node IDs representing the TSP tour
fn shortcut_eulerian_circuit(circuit: &[usize]) -> Vec<usize> {
    let mut visited: HashSet<usize> = HashSet::new();
    let mut tour = Vec::new();

    // iterate through the Eulerian circuit and add nodes to the tour, skipping already visited nodes
    for &node in circuit {
        if visited.insert(node) {
            tour.push(node);
        }
    }

    // close the tour by returning to the starting node
    if let Some(&first_node) = tour.first() {
        tour.push(first_node);
    }

    tour
}

/// Calculates the total cost of a TSP tour by summing the distances between consecutive nodes in the tour.
///
/// # Arguments
/// * `tour` - A slice of node IDs representing the TSP tour
/// * `problem` - The TSP instance containing the distance information
///
/// # Returns
/// * `Result<i64, SolverError>` - The total cost of the tour, or an error if distance retrieval fails for any edge in the tour.
fn try_calculate_tour_cost(tour: &[usize], problem: &TsplibInstance) -> Result<i64, SolverError> {
    tour.windows(2)
        .map(|window| {
            let u = window[0];
            let v = window[1];
            problem
                .try_get_distance(u, v)
                .map(i64::from)
                .map_err(Into::into)
        })
        .sum::<Result<i64, SolverError>>()
}

/// Rotates a TSP tour so that it starts with the specified start node.
///
/// # Arguments
/// * `tour` - A slice of node IDs representing the TSP tour.
/// * `start_node` - The node ID that the tour should start with.
///
/// # Returns
/// * `Result<Vec<usize>, SolverError>` - The rotated tour starting with the specified start node,
///   or an error if the start node is not found in the tour.
fn try_rotate_tour_to_start_node(
    tour: &[usize],
    start_node: usize,
) -> Result<Vec<usize>, SolverError> {
    let mut rotated_tour = tour.to_vec();

    // if the tour is closed (first and last node are the same),
    // remove the duplicate last node before rotation
    if rotated_tour.first() == rotated_tour.last() {
        rotated_tour.pop();
    }

    // find the position of the start node in the tour
    let pos = rotated_tour
        .iter()
        .position(|&node| node == start_node)
        .ok_or(SolverError::InvalidStartNode)?;

    // rotate the tour so that it starts with the specified start node
    rotated_tour.rotate_left(pos);

    // close the tour by returning to the starting node
    rotated_tour.push(start_node);

    Ok(rotated_tour)
}
