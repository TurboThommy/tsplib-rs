//! This module contains the implementation of the LP relaxation of the TSP.
//! The LP relaxation is a linear programming formulation of the TSP that relaxes the integer constraints
//! on the decision variables, allowing them to take on fractional values.

use std::{
    collections::{HashMap, HashSet},
    f64,
};

use tsplib_core::{
    context::ExecutionContext,
    models::{TspSolution, TsplibInstance},
};

use crate::{
    Christofides, SolverOptions, TspSolver,
    errors::{SimplexError, SolverError},
};

const EPS: f64 = 1e-9;

#[derive(Default)]
pub struct LinearProgram {}

#[derive(Debug, Clone)]
pub struct LpRelaxationResult {
    pub lower_bound: f64,
    pub edges: Vec<(usize, usize, f64)>,
}

type Tableau = (Vec<Vec<f64>>, Vec<f64>, Vec<f64>, Vec<usize>, Vec<usize>);

impl TspSolver for LinearProgram {
    fn try_solve_with_context(
        &self,
        problem: &TsplibInstance,
        start_node: usize,
        ctx: ExecutionContext,
        _: SolverOptions,
    ) -> Result<TspSolution, SolverError> {
        let n = problem.nodes.len();
        tracing::info!(
            node_count = n,
            start_node,
            "Starting Linear Programming TSP solver"
        );

        // check problem validity
        self.try_check_problem_validity(problem, start_node)?;

        // check for cancellation before starting the LP relaxation
        if ctx.is_cancelled() {
            tracing::debug!("Linear Programming TSP solver cancelled before starting");
            return Err(SolverError::Cancelled);
        }

        let fixed_edges = initial_fixed_edges(problem);

        let mut solution = match branch_and_bound(problem, &fixed_edges, ctx)? {
            Some(solution) => Ok(solution),
            None => Err(SolverError::NoSolution),
        }?;

        try_rotate_tour_to_start_node(&mut solution, start_node)?;

        tracing::info!(
            tour_length = solution.tour.len(),
            cost = solution.cost,
            "Finished Linear Programming TSP solver"
        );

        Ok(solution)
    }
}

impl LinearProgram {
    pub fn new() -> Self {
        LinearProgram {}
    }
}

/// Asserts that the dimensions of the input matrices and vectors are consistent for the linear programming problem.
///
/// # Arguments
/// * `a` - A reference to a 2D vector representing the constraint coefficients matrix.
/// * `b` - A reference to a vector representing the right-hand side constants of the constraints.
/// * `c` - A reference to a vector representing the coefficients of the objective function.
fn assert_dimensions(a: &[Vec<f64>], b: &[f64], c: &[f64]) {
    let m = b.len();
    let n = c.len();

    assert_eq!(a.len(), m, "Number of rows in A should match length of b");
    assert!(
        a.iter().all(|row| row.len() == n),
        "All rows in A should have the same number of columns as length of c"
    );
}

/// Asserts that the current tableau is in canonical form with respect to the given basis.
///
/// # Arguments
/// * `a` - A reference to a 2D vector representing the constraint coefficients matrix.
/// * `b` - A reference to a vector representing the right-hand side constants of the constraints.
/// * `c` - A reference to a vector representing the coefficients of the objective function.
/// * `basis` - A reference to a vector containing the indices of the basic variables.
fn assert_canonical(a: &[Vec<f64>], b: &[f64], c: &[f64], basis: &[usize]) {
    let m = b.len();
    let n = c.len();

    assert_eq!(
        basis.len(),
        m,
        "Basis should have the same length as the number of constraints"
    );
    assert!(
        basis.iter().all(|&col| col < n),
        "Basis indices should be valid column indices"
    );

    // Check for duplicate indices in the basis
    {
        let mut seen = basis.to_vec();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), m, "Basis should not contain duplicate indices");
    }

    // Check that the basis columns form an identity matrix in A and that the corresponding cost coefficients in C are zero
    for (i, &col) in basis.iter().enumerate() {
        for (row, a_row) in a.iter().enumerate() {
            let expected = if row == i { 1.0 } else { 0.0 };
            assert!(
                (a_row[col] - expected).abs() < EPS,
                "Basis column {col} does not form an identity matrix: a[{row}][{col}] = {} (expected {expected})",
                a_row[col]
            );
        }
        assert!(
            c[col].abs() < EPS,
            "Basis variable {col} has non-zero cost coefficient: c[{col}] = {}",
            c[col]
        );
    }
}

/// Performs a pivot operation on the tableau to update the basis and maintain feasibility.
///
/// # Arguments
/// * `a` - A mutable reference to a 2D vector representing the constraint coefficients matrix.
/// * `b` - A mutable reference to a vector representing the right-hand side constants of the constraints.
/// * `c` - A mutable reference to a vector representing the coefficients of the objective function.
/// * `basis` - A mutable reference to a vector containing the indices of the basic variables.
/// * `z` - The index of the leaving variable (row index).
/// * `s` - The index of the entering variable (column index).
fn pivot(
    a: &mut [Vec<f64>],
    b: &mut [f64],
    c: &mut [f64],
    basis: &mut [usize],
    z: usize,
    s: usize,
) {
    let m = b.len();
    let n = c.len();
    let a_zs = a[z][s];

    let l = basis
        .iter()
        .position(|&col| (a[z][col] - 1.0).abs() < EPS)
        .unwrap();

    // Normalize the pivot row
    a[z] = a[z].iter().map(|&x| x / a_zs).collect();
    b[z] /= a_zs;

    // Update the other rows to eliminate the entering variable
    for j in 0..m {
        if j != z {
            let a_js = a[j][s];

            a[j] = a[j]
                .iter()
                .zip(a[z].iter())
                .map(|(&x, &y)| x - a_js * y)
                .collect();

            b[j] -= a_js * b[z];
        }
    }

    // Update the cost vector to reflect the new basis
    let c_s = c[s];
    for i in 0..n {
        c[i] -= c_s * a[z][i];
    }

    // Update the basis to reflect the new entering variable
    basis[l] = s;
}

/// Helper function to compute the index of the edge (i, j) in a 1D array representation of the upper triangular part of an adjacency matrix.
fn edge_index(i: usize, j: usize) -> usize {
    assert!(i > j, "edge_index should only be called with i < j");
    (i - 1) * (i - 2) / 2 + (j - 1)
}

/// Helper function to compute the column index for the edge (u, v) in the tableau, ensuring that u > v for consistent indexing.
fn edge_col(u: usize, v: usize) -> usize {
    let (i, j) = if u > v { (u, v) } else { (v, u) };
    edge_index(i, j)
}

/// Helper function to compute the key for the edge (u, v) in the tableau, ensuring that u > v for consistent indexing.
fn edge_key(u: usize, v: usize) -> (usize, usize) {
    let (i, j) = if u > v { (u, v) } else { (v, u) };
    (i, j)
}

/// Initializes a HashMap of fixed edges from the given TSP problem instance.
/// The keys are tuples representing the edges (i, j), and the values are booleans indicating whether the edge is fixed to 1 (true) or 0 (false).
///
/// # Arguments
/// * `problem` - A reference to the TSP problem instance containing the fixed edges information.
///
/// # Returns
/// * `HashMap<(usize, usize), bool>` - A HashMap where the keys are edges (i, j) and the values indicate whether the edge is fixed to 1 (true) or 0 (false).
fn initial_fixed_edges(problem: &TsplibInstance) -> HashMap<(usize, usize), bool> {
    let mut fixed_edges = HashMap::new();
    if let Some(edges) = &problem.fixed_edges {
        for &(i, j) in edges.iter() {
            fixed_edges.insert(edge_key(i, j), true);
        }
    }
    fixed_edges
}

/// Builds the initial tableau for the LP relaxation of the TSP problem, incorporating any fixed edges specified in the input.
///
/// # Arguments
/// * `problem` - A reference to the TSP problem instance containing the nodes and distances.
/// * `fixed_edges` - A reference to a HashMap containing edges that are fixed to either 1 (true) or 0 (false).
///
/// # Returns
/// * `Result<Tableau, SolverError>` - Returns a Result containing the constructed tableau (A, b, c, basis, artificial_cols) if successful,
///   or a SolverError if there was an issue with the problem instance or fixed edges.
fn try_build_tableau(
    problem: &TsplibInstance,
    fixed_edges: &HashMap<(usize, usize), bool>,
) -> Result<Tableau, SolverError> {
    let node_count = problem.nodes.len();

    // number of edges in a complete graph with node_count nodes
    let e = node_count * (node_count - 1) / 2;

    // number of constraints: node_count degree constraints + e upper-bound constraints
    let m = node_count * (node_count + 1) / 2;

    // total number of columns: e edge variables + e slack variables + node_count artificial variables
    let n = node_count * node_count;

    let slack_offset = e;
    let artificial_offset = 2 * e;
    let bound_row_offset = node_count;

    let mut a = vec![vec![0.0; n]; m];
    let mut b = vec![0.0; m];
    let mut c = vec![0.0; n];
    let mut basis = vec![0; m];
    let mut artificial_cols: Vec<usize> = Vec::new();

    // degree constraints
    for v in 1..=node_count {
        for w in 1..=node_count {
            if w == v {
                continue;
            }

            // variable for edge (v, w)
            a[v - 1][edge_col(v, w)] = 1.0;
        }

        // artificial variable for degree constraint
        a[v - 1][artificial_offset + (v - 1)] = 1.0;
        b[v - 1] = 2.0;
        basis[v - 1] = artificial_offset + (v - 1);
        artificial_cols.push(artificial_offset + (v - 1));
    }

    // upper-bound constraints (x_ij <= 1)
    for i in 1..=node_count {
        for j in 1..i {
            let k = edge_index(i, j);

            // variable for edge (i, j)
            a[bound_row_offset + k][k] = 1.0;
            // slack variable for the edge (i, j) upper bound constraint
            a[bound_row_offset + k][slack_offset + k] = 1.0;

            b[bound_row_offset + k] = 1.0;

            // index of slack variable in basis
            basis[bound_row_offset + k] = slack_offset + k;
            // cost coefficient for edge (i, j)
            c[k] = problem.try_get_distance(i, j)? as f64;
        }
    }

    // remove edges fixed to 0 by setting their upper bound constraints to x_ij <= 0
    // that cancels out the x_ij variable and forces it to be 0 in any feasible solution^
    for (&(i, j), _) in fixed_edges.iter().filter(|&(_, &fixed)| !fixed) {
        let k = edge_col(i, j);
        b[bound_row_offset + k] = 0.0;
    }

    // add new constraint for each edge fixed to 1
    for (&(i, j), _) in fixed_edges.iter().filter(|&(_, &fixed)| fixed) {
        let k = edge_col(i, j);
        let new_col = a[0].len();

        // add new column for the artificial variable
        for row in a.iter_mut() {
            row.push(0.0);
        }
        c.push(0.0);

        // add new row for the constraint
        let mut new_row = vec![0.0; new_col + 1];
        new_row[k] = 1.0;
        new_row[new_col] = 1.0;
        a.push(new_row);
        b.push(1.0);
        basis.push(new_col);

        artificial_cols.push(new_col);
    }

    Ok((a, b, c, basis, artificial_cols))
}

/// Canonicalizes the cost vector `c` with respect to the current basis, ensuring that the cost coefficients of the basic variables are zero.
///
/// # Arguments
/// * `a` - A reference to the constraint coefficients matrix.
/// * `c` - A mutable reference to the cost vector to be canonicalized.
/// * `basis` - A reference to the vector containing the indices of the basic variables.
fn canonicalize_cost(a: &[Vec<f64>], c: &mut [f64], basis: &[usize]) {
    let n = c.len();
    for (i, &col) in basis.iter().enumerate() {
        let factor = c[col];
        if factor.abs() > EPS {
            for j in 0..n {
                c[j] -= factor * a[i][j];
            }
        }
    }
}

/// Adds a subtour cut to the linear programming tableau to eliminate subcycles in the current solution.
///
/// # Arguments
/// * `a` - A mutable reference to the constraint coefficients matrix.
/// * `b` - A mutable reference to the right-hand side constants vector.
/// * `c` - A mutable reference to the cost vector.
/// * `basis` - A mutable reference to the vector containing the indices of the basic variables.
/// * `cut_edges` - A slice containing the indices of the edges that form the subtour cut.
fn add_subtour_cut(
    a: &mut Vec<Vec<f64>>,
    b: &mut Vec<f64>,
    c: &mut Vec<f64>,
    basis: &mut Vec<usize>,
    cut_edges: &[usize],
) {
    let m = b.len();
    let n = c.len(); // == index of new slack column

    // add new column to each row as well as c
    for row in a.iter_mut() {
        row.push(0.0);
    }
    c.push(0.0);

    // add new row for subtour cut: -sum_cut x + s = -2
    let mut new_row = vec![0.0; n + 1];
    for &col in cut_edges {
        new_row[col] = -1.0;
    }
    new_row[n] = 1.0; // coefficient for new slack variable
    let mut new_rhs = -2.0; // right-hand side for subtour cut

    // eliminate basis variables from the new row
    for i in 0..m {
        let f: f64 = new_row[basis[i]];
        if f.abs() > EPS {
            for j in 0..new_row.len() {
                new_row[j] -= f * a[i][j];
            }

            new_rhs -= f * b[i];
        }
    }

    // add new row to tableau
    a.push(new_row);
    b.push(new_rhs);

    // new slack variable becomes basic
    basis.push(n);
}

/// Computes the indices of the edges that cross the cut defined by the set `s` in a graph with `node_count` nodes.
///
/// # Arguments
/// * `node_count` - The number of nodes in the graph.
/// * `s` - A reference to the set of nodes that define the cut.
///
/// # Returns
/// * `Vec<usize>` - A vector containing the indices of the edges that cross the cut.
fn cut_edges_for_set(node_count: usize, s: &HashSet<usize>) -> Vec<usize> {
    let mut cols = Vec::new();
    for i in 1..=node_count {
        for j in 1..i {
            // if one of the nodes is in the set and the other is not, then the edge (i, j) crosses the cut
            if s.contains(&i) ^ s.contains(&j) {
                cols.push(edge_index(i, j));
            }
        }
    }
    cols
}

/// Computes the minimum cut of a weighted undirected graph represented by an adjacency matrix.
///
/// # Arguments
/// * `weights` - A reference to a 2D vector representing the weights of the edges in the graph. `weights[i][j]` is the weight of the edge between nodes `i` and `j`.
/// * `node_count` - The number of nodes in the graph.
///
/// # Returns
/// * `(f64, HashSet<usize>)` - A tuple containing the weight of the minimum cut and a set of nodes that are on one side of the cut.
fn min_cut(weights: &[Vec<f64>], node_count: usize) -> (f64, HashSet<usize>) {
    let mut w = weights.to_vec();
    let mut active = (0..node_count).collect::<Vec<usize>>();

    // for each supernode keep track of the original nodes that have been merged into it
    let mut members = (0..node_count)
        .map(|i| HashSet::from([i]))
        .collect::<Vec<HashSet<usize>>>();

    let mut best_weight = f64::INFINITY;
    let mut best_set = HashSet::new();

    // n-1 iterations of merging the most tightly connected pair
    // search for the most tightly connected pair over all active nodes
    while active.len() > 1 {
        // track which nodes have been added to the growing set
        let mut added = vec![false; node_count];

        // connection strength to the growing set
        // (sum of weights from the vertex to all vertices in the growing set)
        let mut weight_to_a = vec![0.0; node_count];

        let mut prev = usize::MAX;
        let mut last = usize::MAX;
        let mut last_weight = 0.0;

        for _ in 0..active.len() {
            let mut selected = usize::MAX;
            let mut selected_weight = f64::NEG_INFINITY;

            // find the most tightly connected vertex to the growing set which is still active
            for &v in &active {
                if !added[v] && weight_to_a[v] > selected_weight {
                    selected_weight = weight_to_a[v];
                    selected = v;
                }
            }

            // selected is the most tightly connected vertex to the growing set, add it to the set
            added[selected] = true;
            prev = last;
            last = selected;
            last_weight = selected_weight;

            // update connection strength for remaining vertices
            for &u in &active {
                if !added[u] {
                    weight_to_a[u] += w[selected][u];
                }
            }
        }

        // after growing the set, last and prev are the most tightly connected pair of vertices (or supernodes)
        let (t, s) = (last, prev);
        if last_weight < best_weight {
            best_weight = last_weight;
            best_set = members[t].clone();
        }

        // merge t into s, update weights and members
        let t_row = w[t].clone();
        for u in 0..node_count {
            if u != s && u != t {
                w[s][u] += t_row[u];
                w[u][s] += t_row[u];
            }
        }
        // remove t from active set
        let t_members = std::mem::take(&mut members[t]);
        // merge members of t into s
        members[s].extend(t_members);
        // remove t from active set
        active.retain(|&x| x != t);
    }

    (best_weight, best_set)
}

/// Builds a weight matrix from the solution vector `x`, where `x` contains the values of the decision variables corresponding to the edges in the TSP.
///
/// # Arguments
/// * `x` - A slice containing the values of the decision variables corresponding to the edges in the TSP.
/// * `node_count` - The number of nodes in the TSP instance.
///
/// # Returns
/// * `Vec<Vec<f64>>` - A 2D vector representing the weights of the edges in the graph.
fn build_weight_matrix(x: &[f64], node_count: usize) -> Vec<Vec<f64>> {
    let mut w = vec![vec![0.0; node_count]; node_count];
    for i in 1..=node_count {
        for j in 1..i {
            let val = x[edge_index(i, j)];
            if val > EPS {
                // convert to 0-based indexing
                w[i - 1][j - 1] = val;
                w[j - 1][i - 1] = val;
            }
        }
    }
    w
}

/// Attempts to solve the initial linear programming tableau for the TSP problem, incorporating any fixed edges specified in the input.
/// This includes adding artificial variables for the degree constraints and solving the tableau using the primal simplex method to drive the artificial variables to zero.
///
/// # Arguments
/// * `problem` - A reference to the TSP problem instance containing the nodes and distances.
/// * `fixed_edges` - A reference to a HashMap containing edges that are fixed to either 1 (true) or 0 (false).
///
/// # Returns
/// * `Result<Option<(Tableau, Vec<f64>)>, SolverError>` - The result of the operation.
fn try_solve_initial(
    problem: &TsplibInstance,
    fixed_edges: &HashMap<(usize, usize), bool>,
) -> Result<Option<(Tableau, Vec<f64>)>, SolverError> {
    // Build the initial tableau
    let (mut a, mut b, c_real, mut basis, artificial_cols) =
        try_build_tableau(problem, fixed_edges)?;

    // Canonicalize the cost vector: artificial variables should have 1, the rest should have 0
    let n = a[0].len();
    let mut c = vec![0.0; n];
    for &col in &artificial_cols {
        c[col] = 1.0;
    }
    canonicalize_cost(&a, &mut c, &basis);

    // Solve initial tableau using primal simplex to minimize the sum of artificial variables
    let x1 = try_primal_simplex(&mut a, &mut b, &mut c, &mut basis)?;

    // Check validity: sum of artificial variables should be 0
    let artificial_sum: f64 = artificial_cols.iter().map(|&col| x1[col]).sum();
    if artificial_sum > EPS {
        return Ok(None);
    }

    // Build the tableau for the original cost function
    let mut c = c_real;
    let big_m = 1.0 + c.iter().map(|v| v.abs()).sum::<f64>();
    for &col in &artificial_cols {
        c[col] = big_m;
    }
    canonicalize_cost(&a, &mut c, &basis);

    // solve the tableau with the original cost function using primal simplex
    // note: the solution can still contain subcycles
    let x = try_primal_simplex(&mut a, &mut b, &mut c, &mut basis)?;

    Ok(Some(((a, b, c, basis, artificial_cols), x)))
}

/// Attempts to solve the LP relaxation of the TSP problem, incorporating any fixed edges specified in the input.
/// This includes iteratively adding subtour cuts to eliminate subcycles in the solution until a feasible solution without subcycles is found.
///
/// # Arguments
/// * `problem` - A reference to the TSP problem instance containing the nodes and distances.
/// * `fixed_edges` - A reference to a HashMap containing edges that are fixed to either 1 (true) or 0 (false).
/// * `ctx` - An execution context that can be used to check for cancellation of the operation.
///
/// # Returns
/// * `Result<Option<LpRelaxationResult>, SolverError>` - The result of the operation, containing the lower bound and edges of the solution if successful,
///   or a SolverError if there was an issue with the problem instance or fixed edges.
pub fn try_solve_lp_relaxation(
    problem: &TsplibInstance,
    fixed_edges: &HashMap<(usize, usize), bool>,
    ctx: ExecutionContext,
) -> Result<Option<LpRelaxationResult>, SolverError> {
    let node_count = problem.nodes.len();
    let e = node_count * (node_count - 1) / 2;

    // initial simplex to find a basic feasible solution which may contain subcycles
    let ((mut a, mut b, mut c, mut basis, _), mut x) =
        match try_solve_initial(problem, fixed_edges)? {
            Some(result) => result,
            None => return Ok(None),
        };

    // remove subcycles by adding subtour cuts iteratively until a solution without subcycles is found
    loop {
        // build weight matrix from current solution
        let w = build_weight_matrix(&x, node_count);

        if ctx.is_cancelled() {
            tracing::debug!("LP relaxation cancelled");
            return Err(SolverError::Cancelled);
        }

        // find minimum cut
        let (cut_weight, cut_set) = min_cut(&w, node_count);

        // if cut weight is at least 2, there are no subcycles left
        if cut_weight >= 2.0 - EPS {
            break;
        }

        // add subtour cut for the cut set
        let s: HashSet<usize> = cut_set.iter().map(|&i| i + 1).collect(); // convert to 1-based indexing
        let cut = cut_edges_for_set(node_count, &s);
        add_subtour_cut(&mut a, &mut b, &mut c, &mut basis, &cut);
        x = try_dual_simplex(&mut a, &mut b, &mut c, &mut basis)?;
    }

    let mut edges = Vec::new();
    let mut lower_bound = 0.0;
    // compute the lower bound and collect edges with non-zero values in the solution
    for (k, &val) in x.iter().take(e).enumerate() {
        if val > EPS {
            let (i, j) = index_to_edge(k);
            lower_bound += problem.try_get_distance(i, j)? as f64 * val;
            edges.push((i, j, val));
        }
    }

    Ok(Some(LpRelaxationResult { lower_bound, edges }))
}

/// Finds a fractional edge in the solution vector `edges`, which contains tuples of the form (i, j, value).
/// A fractional edge is defined as an edge whose value is strictly between 0 and 1 (i.e., 0 < value < 1).
///
/// # Arguments
/// * `edges` - A slice of tuples representing the edges and their corresponding values in the solution vector.
///
/// # Returns
/// * `Option<(usize, usize)>` - Returns Some((i, j)) if a fractional edge is found, where (i, j) are the indices of the edge. Returns None if no fractional edge is found.
fn find_fractional_edge(edges: &[(usize, usize, f64)]) -> Option<(usize, usize)> {
    edges
        .iter()
        .filter(|&&(_, _, val)| val > EPS && val < 1.0 - EPS)
        .min_by(|a, b| (a.2 - 0.5).abs().total_cmp(&(b.2 - 0.5).abs()))
        .map(|&(i, j, _)| (i, j))
}

/// Reconstructs a TSP tour from the given edges, ensuring that each vertex has degree 2 and forming a valid tour.
/// The function traverses the adjacency list formed by the edges to create a tour and calculates its cost based on the distances in the TSP problem instance.
///
/// # Arguments
/// * `problem` - A reference to the TSP problem instance containing the nodes and distances.
/// * `edges` - A slice of tuples representing the edges in the tour, where each tuple is of the form (i, j) indicating an edge between vertices i and j.
///
/// # Returns
/// * `Result<TspSolution, SolverError>` - Returns a TspSolution containing the reconstructed tour and its cost if successful,
///   or a SolverError if the edges do not form a valid tour (e.g., if any vertex does not have degree 2).
fn reconstruct_tour(
    problem: &TsplibInstance,
    edges: &[(usize, usize)],
) -> Result<TspSolution, SolverError> {
    let node_count = problem.nodes.len();

    // adjacency list: each vertex should have degree 2, so neighbors are stored in an array of length 2
    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); node_count + 1];
    for &(i, j) in edges {
        adjacency[i].push(j);
        adjacency[j].push(i);
    }

    // check that each vertex has degree 2
    for (v, neighbors) in adjacency.iter().enumerate().skip(1) {
        if neighbors.len() != 2 {
            return Err(SolverError::SimplexError(format!(
                "Expected degree 2 for vertex {v}, got {}",
                adjacency[v].len()
            )));
        }
    }

    // construct the tour by traversing the adjacency list
    let mut tour = Vec::with_capacity(node_count);
    let mut prev = 0; // start without previous node (0 does not exist on 1-based indexing)
    let mut current = 1; // start at vertex 1

    // traverse the tour until all nodes have been visited
    for _ in 0..node_count {
        tour.push(current);
        let next = if adjacency[current][0] != prev {
            adjacency[current][0]
        } else {
            adjacency[current][1]
        };

        prev = current;
        current = next;
    }

    let mut cost: i64 = 0;
    // compute the total cost of the tour by summing the distances between consecutive nodes
    for w in 0..node_count {
        let from = tour[w];
        let to = tour[(w + 1) % node_count];
        cost += problem.try_get_distance(from, to)? as i64;
    }

    Ok(TspSolution { tour, cost })
}

/// Converts a tour represented as a sequence of vertex indices into a vector of edges.
/// The function takes a slice of vertex indices representing the tour and returns a vector of tuples,
/// where each tuple represents an edge between two consecutive vertices in the tour.
/// The last vertex is connected back to the first vertex to complete the tour.
///
/// # Arguments
/// * `tour` - A slice of usize representing the sequence of vertex indices in the tour.
///
/// # Returns
/// * `Vec<(usize, usize)>` - A vector of tuples representing the edges in the tour,
///   where each tuple is of the form (i, j) indicating an edge between vertices i and j.
fn tour_to_edges(tour: &[usize]) -> Vec<(usize, usize)> {
    let mut edges: Vec<(usize, usize)> = tour.windows(2).map(|w| (w[0], w[1])).collect();

    if let (Some(&first), Some(&last)) = (tour.first(), tour.last()) {
        edges.push((last, first));
    }

    edges
}

/// Performs the branch-and-bound algorithm to solve the Traveling Salesman Problem (TSP) using linear programming relaxation and subtour elimination.
/// The algorithm explores the solution space by branching on fractional edges and bounding using the lower bound obtained from the LP relaxation.
/// The best integer solution found during the search is returned as the final result.
///
/// # Arguments
/// * `problem` - A reference to the TSP problem instance containing the nodes and distances.
/// * `initial_fixed` - A reference to a HashMap containing edges that are initially fixed to either 1 (true) or 0 (false).
/// * `ctx` - An execution context that can be used to check for cancellation of the operation.
///
/// # Returns
/// * `Result<Option<TspSolution>, SolverError>` - The result of the operation, containing the best TSP solution found if successful,
///   or a SolverError if there was an issue with the problem instance or if the operation was cancelled.
fn branch_and_bound(
    problem: &TsplibInstance,
    initial_fixed: &HashMap<(usize, usize), bool>,
    ctx: ExecutionContext,
) -> Result<Option<TspSolution>, SolverError> {
    tracing::debug!(
        initial_fixed_edges = ?initial_fixed,
        "Starting branch and bound"
    );

    // get initial solution from a heuristic to use as upper bound
    let seed =
        Christofides::new().try_solve_with_context(problem, 1, ctx, SolverOptions::default());

    let mut best: Option<(f64, Vec<(usize, usize)>)> = match seed {
        Ok(tour) => Some((tour.cost as f64, tour_to_edges(&tour.tour))),
        Err(_) => None,
    };

    // let mut best: Option<(f64, Vec<(usize, usize)>)> = None;
    let mut stack: Vec<HashMap<(usize, usize), bool>> = vec![initial_fixed.clone()];

    let mut iteration_count = 1;

    while let Some(fixed_edges) = stack.pop() {
        if ctx.is_cancelled() {
            tracing::debug!("Branch and bound cancelled");
            return Err(SolverError::Cancelled);
        }

        tracing::debug!(
            iteration_count = iteration_count,
            "Branch & Bound iteration"
        );
        iteration_count += 1;

        // solve the LP relaxation with the current fixed edges
        let result = match try_solve_lp_relaxation(problem, &fixed_edges, ctx)? {
            Some(result) => result,
            None => continue,
        };

        // bound: if the lower bound is worse than the best solution found so far, skip branching on this node (prune)
        if let Some((best_cost, _)) = &best
            && (result.lower_bound - EPS).ceil() >= *best_cost
        {
            tracing::trace!(best_cost = ?best_cost, lower_bound = ?result.lower_bound, "Pruning branch with worse lower bound");
            continue;
        }

        // find fractional edges
        match find_fractional_edge(&result.edges) {
            // no fractional edges means there might be a new best integer solution
            None => {
                let tour_edges = result
                    .edges
                    .iter()
                    .map(|&(i, j, _)| (i, j))
                    .collect::<Vec<(usize, usize)>>();
                if best
                    .as_ref()
                    .is_none_or(|(best_cost, _)| result.lower_bound < *best_cost)
                {
                    best = Some((result.lower_bound, tour_edges));
                }
            }

            // branch on the fractional edge (i, j) by creating two new subproblems
            Some((i, j)) => {
                tracing::trace!(edge = ?(i, j), "Branching on fractional edge");
                let mut forbid = fixed_edges.clone();
                forbid.insert(edge_key(i, j), false);
                stack.push(forbid);

                let mut force = fixed_edges.clone();
                force.insert(edge_key(i, j), true);
                stack.push(force);
            }
        }
    }

    // Reconstruct the best solution found
    match best {
        Some((_, edges)) => Ok(Some(reconstruct_tour(problem, &edges)?)),
        None => Ok(None),
    }
}

/// Attempts to solve the linear programming problem using the primal simplex method.
/// The function iteratively improves the solution until an optimal solution is found or the problem is determined to be unbounded.
///
/// # Arguments
/// * `a` - A mutable reference to the constraint coefficients matrix (m x n).
/// * `b` - A mutable reference to the right-hand side constants vector (m).
/// * `c` - A mutable reference to the cost vector (n).
/// * `basis` - A mutable reference to the vector (m) containing the indices of the basic variables.
///
/// # Returns
/// * `Result<Vec<f64>, SimplexError>` - Returns a Result containing the optimal solution vector if successful, or a SimplexError if the problem is unbounded or infeasible.
fn try_primal_simplex(
    a: &mut [Vec<f64>],
    b: &mut [f64],
    c: &mut [f64],
    basis: &mut [usize],
) -> Result<Vec<f64>, SimplexError> {
    let m = b.len();
    let n = c.len();

    assert_dimensions(a, b, c);
    assert_canonical(a, b, c, basis);
    for (i, &b_i) in b.iter().enumerate() {
        assert!(
            b_i >= -EPS,
            "b[{i}] = {b_i} < 0, starting base is not primal feasible"
        );
    }

    // Main loop of the primal simplex algorithm
    while c.iter().any(|&x| x < -EPS) {
        // Find the entering variable (most negative coefficient in c)
        let s = c
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(i, _)| i)
            .unwrap();

        // Find the leaving variable
        let z = (0..m)
            .filter_map(|j| {
                let a_js = a[j][s];
                (a_js > EPS).then(|| (j, b[j] / a_js))
            })
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(j, _)| j);

        // If no leaving variable is found, the problem is unbounded
        let z = match z {
            Some(j) => j,
            None => return Err(SimplexError::Unbounded),
        };

        // Perform the pivot operation to update the tableau
        pivot(a, b, c, basis, z, s);
    }

    let mut x = vec![0.0; n];
    // Construct the solution vector from the basic variables
    for &i in basis.iter() {
        if let Some(j) = (0..m).find(|&j| (a[j][i] - 1.0).abs() < EPS) {
            x[i] = b[j];
        }
    }
    Ok(x)
}

/// Attempts to solve the linear programming problem using the dual simplex method.
/// The function iteratively improves the solution until an optimal solution is found or the problem is determined to be infeasible.
///
/// # Arguments
/// * `a` - A mutable reference to the constraint coefficients matrix (m x n).
/// * `b` - A mutable reference to the right-hand side constants vector (m).
/// * `c` - A mutable reference to the cost vector (n).
/// * `basis` - A mutable reference to the vector (m) containing the indices of the basic variables.
///
/// # Returns
/// * `Result<Vec<f64>, SimplexError>` - Returns a Result containing the optimal solution vector if successful, or a SimplexError if the problem is infeasible.
fn try_dual_simplex(
    a: &mut [Vec<f64>],
    b: &mut [f64],
    c: &mut [f64],
    basis: &mut [usize],
) -> Result<Vec<f64>, SimplexError> {
    let n = c.len();

    assert_dimensions(a, b, c);
    assert_canonical(a, b, c, basis);

    // Main loop of the dual simplex algorithm
    while b.iter().any(|&x| x < -EPS) {
        // Find the leaving variable (most negative value in b)
        let z = b
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(i, _)| i)
            .unwrap();

        // Find the entering variable
        let s = (0..n)
            .filter_map(|i| {
                let a_zj = a[z][i];
                (a_zj < -EPS).then(|| (i, c[i] / a_zj))
            })
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(i, _)| i);

        // If no entering variable is found, the problem is infeasible
        let s = match s {
            Some(i) => i,
            None => return Err(SimplexError::Infeasible),
        };

        // Perform the pivot operation to update the tableau
        pivot(a, b, c, basis, z, s);
    }

    // primal simplex is used to find the optimal solution after dual feasibility is restored
    try_primal_simplex(a, b, c, basis)
}

/// Converts an edge index `k` into its corresponding edge (i, j) in a complete graph with `n` nodes.
/// The edges are indexed in a specific order, and this function computes the corresponding vertex indices for the given edge index.
///
/// # Arguments
/// * `k` - The index of the edge in the complete graph.
///
/// # Returns
/// * `(usize, usize)` - A tuple containing the vertex indices (i, j) corresponding to the edge index `k`.
fn index_to_edge(k: usize) -> (usize, usize) {
    let mut i = 2;
    while i * (i - 1) / 2 <= k {
        i += 1;
    }
    let j = k - (i - 1) * (i - 2) / 2 + 1;
    (i, j)
}

/// Rotates a TSP tour so that it starts with the specified start node.
///
/// # Arguments
/// * `solution` - A mutable reference to the TSP solution.
/// * `start_node` - The node ID that the tour should start with.
///
/// # Returns
/// * `Result<(), SolverError>` - Returns `Ok(())` if the rotation was successful, or a `SolverError` if the start node is not found in the tour.
fn try_rotate_tour_to_start_node(
    solution: &mut TspSolution,
    start_node: usize,
) -> Result<(), SolverError> {
    // if the tour is closed (first and last node are the same),
    // remove the duplicate last node before rotation
    if solution.tour.first() == solution.tour.last() {
        solution.tour.pop();
    }

    // find the position of the start node in the tour
    let pos = solution
        .tour
        .iter()
        .position(|&node| node == start_node)
        .ok_or(SolverError::InvalidStartNode)?;

    // rotate the tour so that it starts with the specified start node
    solution.tour.rotate_left(pos);

    // close the tour by returning to the starting node
    solution.tour.push(start_node);

    Ok(())
}

#[cfg(test)]
mod tests;
