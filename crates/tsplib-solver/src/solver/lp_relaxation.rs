//! This module contains the implementation of the LP relaxation of the TSP.
//! The LP relaxation is a linear programming formulation of the TSP that relaxes the integer constraints
//! on the decision variables, allowing them to take on fractional values.

use std::{cmp::Ordering, f64};

use crate::errors::SimplexError;

pub struct LpRelaxation {}

// A, b, c, B are the standard inputs, according to rust style guidelines we should use snake case for variable names
// so A will be a, b will be b, c will be c and B will be basis
//
// a -> m x n; matrix of coefficients for the constraints
// b -> m; vector of constants for the constraints
// c -> n; vector of coefficients for the objective function
// basis -> m; vector of indices of the basic variables (initially, this should be the indices of the slack variables)
fn try_primal_simplex(
    mut a: Vec<Vec<f64>>,
    mut b: Vec<f64>,
    mut c: Vec<f64>,
    mut basis: Vec<usize>,
) -> Result<(Vec<f64>, Vec<usize>), SimplexError> {
    let m = b.len();
    let n = c.len();
    const EPS: f64 = 1e-9;

    assert_eq!(a.len(), m, "Number of rows in A should match length of b");
    assert!(
        a.iter().all(|row| row.len() == n),
        "All rows in A should have the same number of columns as length of c"
    );

    // Check preconditions (basis should be valid)
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
        let mut seen = basis.clone();
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

    for (i, &b_i) in b.iter().enumerate() {
        assert!(
            b_i >= -EPS,
            "b[{i}] = {b_i} < 0, starting base is not primal feasible"
        );
    }

    // Main loop of the primal simplex algorithm
    while c.iter().any(|&x| x < -EPS) {
        let (s, c_s) =
            c.iter()
                .enumerate()
                .fold(
                    (usize::MAX, f64::INFINITY),
                    |(a_i, a), (i, &x)| match PartialOrd::partial_cmp(&a, &x) {
                        None => (usize::MAX, f64::NAN),
                        Some(Ordering::Less) => (a_i, a),
                        Some(_) => (i, x),
                    },
                );

        assert!(
            s != usize::MAX,
            "No valid entering variable found, this should not happen if the algorithm is implemented correctly"
        );

        let (z, _) = (0..m)
            .filter_map(|j| {
                let a_js = a[j][s];
                (a_js > EPS).then(|| (j, b[j] / a_js))
            })
            .fold((usize::MAX, f64::INFINITY), |(a_i, a), (b_i, b)| {
                if b < a { (b_i, b) } else { (a_i, a) }
            });

        if z == usize::MAX {
            return Err(SimplexError::Unbounded);
        }

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

        c = c
            .iter()
            .zip(a[z].iter())
            .map(|(&x, &y)| x - c_s * y)
            .collect();

        basis[l] = s;
    }

    let mut x = vec![0.0; n];
    for &i in &basis {
        if let Some(j) = (0..m).find(|&j| (a[j][i] - 1.0).abs() < EPS) {
            x[i] = b[j];
        }
    }
    Ok((x, basis))
}

#[cfg(test)]
#[test]
fn test_primal_simplex() {
    let a = vec![
        vec![1.0, 1.0, 1.0, 0.0, 0.0],
        vec![6.0, 9.0, 0.0, 1.0, 0.0],
        vec![0.0, 1.0, 0.0, 0.0, 1.0],
    ];

    let b = vec![100.0, 720.0, 60.0];
    let c = vec![-10.0, -20.0, 0.0, 0.0, 0.0];
    let basis = vec![2, 3, 4];

    let c_orig = c.clone();

    let (x, basis) =
        try_primal_simplex(a, b, c, basis).expect("Primal simplex should find an optimal solution");

    tracing::debug!("Optimal solution: {:?}", x);
    tracing::debug!("Optimal basis: {:?}", basis);

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
    let a = vec![vec![-1.0, 1.0]];
    let b = vec![1.0];
    let c = vec![-1.0, 0.0];
    let basis = vec![1];

    assert_eq!(
        try_primal_simplex(a, b, c, basis),
        Err(SimplexError::Unbounded)
    );
}
