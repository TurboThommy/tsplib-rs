//! This module contains the implementation of the LP relaxation of the TSP.
//! The LP relaxation is a linear programming formulation of the TSP that relaxes the integer constraints
//! on the decision variables, allowing them to take on fractional values.

use std::f64;

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
mod tests {
    use tsplib_core::{
        enums::{DistanceSource, EdgeWeightType, ProblemType},
        models::{Node, TsplibInstance},
    };

    use crate::{
        errors::SimplexError,
        solver::lp_relaxation::{
            assert_canonical, assert_dimensions, try_build_tableau, try_dual_simplex,
            try_primal_simplex,
        },
    };

    #[test]
    fn test_primal_simplex() {
        let mut a = vec![
            vec![1.0, 1.0, 1.0, 0.0, 0.0],
            vec![6.0, 9.0, 0.0, 1.0, 0.0],
            vec![0.0, 1.0, 0.0, 0.0, 1.0],
        ];

        let mut b = vec![100.0, 720.0, 60.0];
        let mut c = vec![-10.0, -20.0, 0.0, 0.0, 0.0];
        let mut basis = vec![2, 3, 4];

        let c_orig = c.clone();

        let x = try_primal_simplex(&mut a, &mut b, &mut c, &mut basis)
            .expect("Primal simplex should find an optimal solution");

        assert!((x[0] - 30.0).abs() < 1e-9); // x1
        assert!((x[1] - 60.0).abs() < 1e-9); // x2
        assert!((x[2] - 10.0).abs() < 1e-9); // slack variable for first constraint
        assert!((x[3].abs() < 1e-9)); // slack variable for second constraint
        assert!((x[4].abs() < 1e-9)); // slack variable for third constraint

        assert_eq!(basis, vec![2, 0, 1]);

        let obj: f64 = c_orig.iter().zip(&x).map(|(&c_i, x_i)| c_i * x_i).sum();
        assert!((obj + 1500.0).abs() < 1e-9); // min -1500 = max 1500
    }

    #[test]
    fn test_primal_simplex_unbounded() {
        let mut a = vec![vec![-1.0, 1.0]];
        let mut b = vec![1.0];
        let mut c = vec![-1.0, 0.0];
        let mut basis = vec![1];

        assert_eq!(
            try_primal_simplex(&mut a, &mut b, &mut c, &mut basis),
            Err(SimplexError::Unbounded)
        );
    }

    #[test]
    fn test_dual_simplex() {
        let mut a = vec![
            vec![-1.0, -1.0, 1.0, 0.0, 0.0],
            vec![-3.0, -1.0, 0.0, 1.0, 0.0],
            vec![1.0, 1.0, 0.0, 0.0, 1.0],
        ];

        let mut b = vec![-8.0, -12.0, 10.0];
        let mut c = vec![-2.0, -1.0, 0.0, 0.0, 0.0];
        let mut basis = vec![2, 3, 4];

        let c_orig = c.clone();

        let x = try_dual_simplex(&mut a, &mut b, &mut c, &mut basis)
            .expect("Dual simplex should find an optimal solution");

        assert!((x[0] - 10.0).abs() < 1e-9); // x1
        assert!(x[1].abs() < 1e-9); // x2
        assert!((x[2] - 2.0).abs() < 1e-9); // slack variable for first constraint
        assert!((x[3] - 18.0).abs() < 1e-9); // slack variable for second constraint
        assert!(x[4].abs() < 1e-9); // slack variable for third constraint

        assert_eq!(basis, vec![2, 3, 0]);

        let obj: f64 = c_orig.iter().zip(&x).map(|(&c_i, x_i)| c_i * x_i).sum();
        assert!((obj + 20.0).abs() < 1e-9);
    }

    #[test]
    fn test_dual_simplex_infeasible() {
        let mut a = vec![vec![1.0, 1.0, 1.0, 0.0], vec![1.0, 0.0, 0.0, 1.0]];
        let mut b = vec![-1.0, 5.0];
        let mut c = vec![1.0, 1.0, 0.0, 0.0];
        let mut basis = vec![2, 3];

        assert_eq!(
            try_dual_simplex(&mut a, &mut b, &mut c, &mut basis),
            Err(SimplexError::Infeasible)
        )
    }

    #[test]
    fn test_build_tableau() {
        let problem = TsplibInstance {
            problem_id: "test".to_string(),
            name: "test".to_string(),
            problem_type: ProblemType::TSP,
            nodes: vec![
                Node {
                    id: 1,
                    x: 0.0,
                    y: 0.0,
                    z: None,
                },
                Node {
                    id: 2,
                    x: 1.0,
                    y: 1.0,
                    z: None,
                },
                Node {
                    id: 3,
                    x: 2.0,
                    y: 2.0,
                    z: None,
                },
                Node {
                    id: 4,
                    x: 3.0,
                    y: 3.0,
                    z: None,
                },
            ],
            distance_source: DistanceSource::Geometric(EdgeWeightType::Euc2D),
            fixed_edges: None,
        };

        let tableau = try_build_tableau(&problem).expect("Tableau should be built successfully");
        let (a, b, c, basis) = tableau;

        // Check dimensions of the tableau
        // 4 degree constraints + 6 upper-bound constraints
        assert_eq!(a.len(), 10);
        // 16 variables (6 edges + 4 artificial variables + 6 slack variables)
        assert_eq!(a[0].len(), 16);
        assert_eq!(b.len(), 10);
        assert_eq!(c.len(), 16);
        assert_eq!(basis.len(), 10);

        // Check degree constraints
        let mut deg_1 = vec![0.0; 16];
        deg_1[0] = 1.0;
        deg_1[1] = 1.0;
        deg_1[3] = 1.0;
        deg_1[12] = 1.0;

        let mut deg_2 = vec![0.0; 16];
        deg_2[0] = 1.0;
        deg_2[2] = 1.0;
        deg_2[4] = 1.0;
        deg_2[13] = 1.0;

        let mut deg_3 = vec![0.0; 16];
        deg_3[1] = 1.0;
        deg_3[2] = 1.0;
        deg_3[5] = 1.0;
        deg_3[14] = 1.0;

        let mut deg_4 = vec![0.0; 16];
        deg_4[3] = 1.0;
        deg_4[4] = 1.0;
        deg_4[5] = 1.0;
        deg_4[15] = 1.0;

        assert_eq!(a[0], deg_1);
        assert_eq!(a[1], deg_2);
        assert_eq!(a[2], deg_3);
        assert_eq!(a[3], deg_4);

        // Check upper-bound constraints
        let mut edge_12 = vec![0.0; 16];
        edge_12[0] = 1.0;
        edge_12[6] = 1.0;

        let mut edge_13 = vec![0.0; 16];
        edge_13[1] = 1.0;
        edge_13[7] = 1.0;

        let mut edge_23 = vec![0.0; 16];
        edge_23[2] = 1.0;
        edge_23[8] = 1.0;

        let mut edge_14 = vec![0.0; 16];
        edge_14[3] = 1.0;
        edge_14[9] = 1.0;

        let mut edge_24 = vec![0.0; 16];
        edge_24[4] = 1.0;
        edge_24[10] = 1.0;

        let mut edge_34 = vec![0.0; 16];
        edge_34[5] = 1.0;
        edge_34[11] = 1.0;

        assert_eq!(a[4], edge_12);
        assert_eq!(a[5], edge_13);
        assert_eq!(a[6], edge_23);
        assert_eq!(a[7], edge_14);
        assert_eq!(a[8], edge_24);
        assert_eq!(a[9], edge_34);

        assert_dimensions(&a, &b, &c);
        assert_canonical(&a, &b, &c, &basis);
    }
}
