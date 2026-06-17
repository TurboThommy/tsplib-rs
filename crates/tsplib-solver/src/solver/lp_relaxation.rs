//! This module contains the implementation of the LP relaxation of the TSP.
//! The LP relaxation is a linear programming formulation of the TSP that relaxes the integer constraints
//! on the decision variables, allowing them to take on fractional values.

use std::{collections::HashSet, f64};

use tsplib_core::models::TsplibInstance;

use crate::errors::{SimplexError, SolverError};

const EPS: f64 = 1e-9;

pub struct LpRelaxation {}

type Tableau = (Vec<Vec<f64>>, Vec<f64>, Vec<f64>, Vec<usize>);

fn assert_dimensions(a: &[Vec<f64>], b: &[f64], c: &[f64]) {
    let m = b.len();
    let n = c.len();

    assert_eq!(a.len(), m, "Number of rows in A should match length of b");
    assert!(
        a.iter().all(|row| row.len() == n),
        "All rows in A should have the same number of columns as length of c"
    );
}

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
    {
        let mut seen = basis.to_vec();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), m, "Basis should not contain duplicate indices");
    }

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

    a[z] = a[z].iter().map(|&x| x / a_zs).collect();
    b[z] /= a_zs;

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

    let c_s = c[s];
    for i in 0..n {
        c[i] -= c_s * a[z][i];
    }

    basis[l] = s;
}

fn edge_index(i: usize, j: usize) -> usize {
    assert!(i > j, "edge_index should only be called with i < j");
    (i - 1) * (i - 2) / 2 + (j - 1)
}

fn edge_col(u: usize, v: usize) -> usize {
    let (i, j) = if u > v { (u, v) } else { (v, u) };
    edge_index(i, j)
}

fn try_build_tableau(problem: &TsplibInstance) -> Result<Tableau, SolverError> {
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

    Ok((a, b, c, basis))
}

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

fn cut_edges_for_set(node_count: usize, s: &HashSet<usize>) -> Vec<usize> {
    let mut cols = Vec::new();
    for i in 1..=node_count {
        for j in 1..i {
            if s.contains(&i) ^ s.contains(&j) {
                cols.push(edge_index(i, j));
            }
        }
    }
    cols
}

fn try_solve_initial(problem: &TsplibInstance) -> Result<(Tableau, Vec<f64>), SolverError> {
    let node_count = problem.nodes.len();
    let e = node_count * (node_count - 1) / 2;
    let n = node_count * node_count;
    let artificial_offset = 2 * e;

    // Build the initial tableau
    let (mut a, mut b, c_real, mut basis) = try_build_tableau(problem)?;

    // Canonicalize the cost vector: artificial variables should have 1, the rest should have 0
    let mut c = vec![0.0; n];
    for v in 0..node_count {
        c[artificial_offset + v] = 1.0;
    }
    canonicalize_cost(&a, &mut c, &basis);

    // Solve initial tableau using primal simplex to minimize the sum of artificial variables
    let x1 = try_primal_simplex(&mut a, &mut b, &mut c, &mut basis)?;

    // Check validity: sum of artificial variables should be 0
    let artificial_sum = (0..node_count).map(|v| x1[artificial_offset + v]).sum();
    if artificial_sum > EPS {
        return Err(SolverError::LpRelaxationInfeasible(artificial_sum));
    }

    // Build the tableau for the original cost function
    let mut c = c_real;
    let big_m = 1.0 + c.iter().map(|v| v.abs()).sum::<f64>();
    for v in 0..node_count {
        c[artificial_offset + v] = big_m;
    }
    canonicalize_cost(&a, &mut c, &basis);

    // solve the tableau with the original cost function using dual simplex
    // note: the solution can still contain subcycles
    let x = try_primal_simplex(&mut a, &mut b, &mut c, &mut basis)?;

    Ok(((a, b, c, basis), x))
}

// A, b, c, B are the standard inputs, according to rust style guidelines we should use snake case for variable names
// so A will be a, b will be b, c will be c and B will be basis
//
// a -> m x n; matrix of coefficients for the constraints
// b -> m; vector of constants for the constraints
// c -> n; vector of coefficients for the objective function
// basis -> m; vector of indices of the basic variables (initially, this should be the indices of the slack variables)
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
        let s = c
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(i, _)| i)
            .unwrap();

        let z = (0..m)
            .filter_map(|j| {
                let a_js = a[j][s];
                (a_js > EPS).then(|| (j, b[j] / a_js))
            })
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(j, _)| j);

        let z = match z {
            Some(j) => j,
            None => return Err(SimplexError::Unbounded),
        };

        pivot(a, b, c, basis, z, s);
    }

    let mut x = vec![0.0; n];
    for &i in basis.iter() {
        if let Some(j) = (0..m).find(|&j| (a[j][i] - 1.0).abs() < EPS) {
            x[i] = b[j];
        }
    }
    Ok(x)
}

fn try_dual_simplex(
    a: &mut [Vec<f64>],
    b: &mut [f64],
    c: &mut [f64],
    basis: &mut [usize],
) -> Result<Vec<f64>, SimplexError> {
    let n = c.len();

    assert_dimensions(a, b, c);
    assert_canonical(a, b, c, basis);

    while b.iter().any(|&x| x < -EPS) {
        let z = b
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(i, _)| i)
            .unwrap();

        let s = (0..n)
            .filter_map(|i| {
                let a_zj = a[z][i];
                (a_zj < -EPS).then(|| (i, c[i] / a_zj))
            })
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(i, _)| i);

        let s = match s {
            Some(i) => i,
            None => return Err(SimplexError::Infeasible),
        };

        pivot(a, b, c, basis, z, s);
    }

    try_primal_simplex(a, b, c, basis)
}

#[cfg(test)]
mod tests;
