//! LP relaxation of the TSP, optimized variant (`LpOptimized`).
//!
//! Same branch-and-bound / cutting-plane structure as `LinearProgram`, but the
//! LP engine is a **bounded-variable simplex**: the 0 <= x_e <= 1 bounds are
//! handled implicitly instead of as explicit constraint rows. The constraint
//! matrix therefore has only `node_count` rows (degree constraints) plus one
//! row per subtour cut, instead of `node_count*(node_count+1)/2` rows.
//!
//! Consequences:
//!  * the per-node tableau shrinks by a factor of ~`(node_count+1)/2`
//!    (e.g. 76 rows instead of 2926 for pr76), removing the memory wall and
//!    making each pivot far cheaper;
//!  * fixing an edge becomes a pure bound change (`l=u=0` or `l=u=1`) instead of
//!    appending rows / artificials;
//!  * a forced subtour (e.g. three fixed edges forming a triangle) is detected
//!    as LP-infeasibility and pruned, whereas the explicit-bound formulation
//!    returns a (meaningless) bound and relies on later branching.
//!
//! The engine was validated for objective-equivalence against `LinearProgram`
//! on thousands of random instances (root relaxation and full relaxation with
//! subtour cuts, with and without fixed edges).

use std::{
    collections::{HashMap, HashSet},
    f64,
};

use tsplib_core::{
    context::ExecutionContext,
    models::{TspSolution, TsplibInstance},
};

use crate::{
    SolverOptions, TspSolver,
    errors::{SimplexError, SolverError},
};

const EPS: f64 = 1e-9;

#[derive(Default)]
pub struct LpOptimized {}

impl LpOptimized {
    pub fn new() -> Self {
        LpOptimized {}
    }
}

#[derive(Debug, Clone)]
pub struct LpRelaxationResult {
    pub lower_bound: f64,
    pub edges: Vec<(usize, usize, f64)>,
}

impl TspSolver for LpOptimized {
    fn try_solve_with_context(
        &self,
        problem: &TsplibInstance,
        _start_node: usize,
        _ctx: ExecutionContext,
        _: SolverOptions,
    ) -> Result<TspSolution, SolverError> {
        let fixed_edges = initial_fixed_edges(problem);

        match branch_and_bound(problem, &fixed_edges)? {
            Some(solution) => Ok(solution),
            None => Err(SolverError::NoSolution),
        }
    }
}

// ----------------------------------------------------------------------------
// edge helpers (identical to LinearProgram)
// ----------------------------------------------------------------------------
fn edge_index(i: usize, j: usize) -> usize {
    assert!(i > j, "edge_index should only be called with i > j");
    (i - 1) * (i - 2) / 2 + (j - 1)
}

fn edge_col(u: usize, v: usize) -> usize {
    let (i, j) = if u > v { (u, v) } else { (v, u) };
    edge_index(i, j)
}

fn edge_key(u: usize, v: usize) -> (usize, usize) {
    let (i, j) = if u > v { (u, v) } else { (v, u) };
    (i, j)
}

fn index_to_edge(k: usize) -> (usize, usize) {
    let mut i = 2;
    while i * (i - 1) / 2 <= k {
        i += 1;
    }
    let j = k - (i - 1) * (i - 2) / 2 + 1;
    (i, j)
}

fn initial_fixed_edges(problem: &TsplibInstance) -> HashMap<(usize, usize), bool> {
    let mut fixed_edges = HashMap::new();
    if let Some(edges) = &problem.fixed_edges {
        for &(i, j) in edges.iter() {
            fixed_edges.insert(edge_key(i, j), true);
        }
    }
    fixed_edges
}

// ============================================================================
// bounded-variable simplex engine
// ============================================================================
const AT_LOWER: u8 = 0;
const AT_UPPER: u8 = 1;
const BASIC: u8 = 2;

// Generous iteration caps. With Dantzig pricing these LPs terminate well before
// this in practice (validated empirically); a cap is only a guard against an
// unexpected cycle, and is surfaced as a hard error (never as silent pruning).
const PRIMAL_CAP: usize = 2_000_000;
const DUAL_CAP: usize = 2_000_000;

#[derive(Clone)]
struct Bv {
    m: usize,
    n: usize,
    a: Vec<Vec<f64>>,  // m x n canonical tableau (basis columns = identity)
    basis: Vec<usize>, // len m
    lower: Vec<f64>,
    upper: Vec<f64>,
    d: Vec<f64>,    // reduced-cost row (canonical) for the current objective
    beta: Vec<f64>, // len m, value of each basic variable
    status: Vec<u8>,
}

impl Bv {
    fn nbval(&self, j: usize) -> f64 {
        match self.status[j] {
            AT_LOWER => self.lower[j],
            AT_UPPER => self.upper[j],
            _ => panic!("nbval called on basic variable {j}"),
        }
    }

    fn val(&self, j: usize) -> f64 {
        if self.status[j] == BASIC {
            let row = self.basis.iter().position(|&c| c == j).unwrap();
            self.beta[row]
        } else {
            self.nbval(j)
        }
    }

    fn edge_values(&self, e: usize) -> Vec<f64> {
        (0..e).map(|k| self.val(k)).collect()
    }

    // Gauss-Jordan pivot on (r, s): normalize row r, eliminate column s from the
    // other rows, update the reduced-cost row d. Does not touch beta/basis/status.
    fn pivot(&mut self, r: usize, s: usize) {
        let m = self.m;
        let n = self.n;
        let ars = self.a[r][s];
        for k in 0..n {
            self.a[r][k] /= ars;
        }
        let prow = self.a[r].clone();
        for i in 0..m {
            if i != r {
                let ais = self.a[i][s];
                if ais != 0.0 {
                    for (k, &p) in prow.iter().enumerate() {
                        self.a[i][k] -= ais * p;
                    }
                }
            }
        }
        let ds = self.d[s];
        if ds != 0.0 {
            for (k, &p) in prow.iter().enumerate() {
                self.d[k] -= ds * p;
            }
        }
    }

    // Primal simplex with bounded variables.
    fn primal(&mut self) -> Result<(), SimplexError> {
        let m = self.m;
        let n = self.n;
        let mut iters = 0;
        loop {
            iters += 1;
            if iters > PRIMAL_CAP {
                return Err(SimplexError::Unbounded);
            }

            // entering variable (Dantzig on bound-aware reduced cost).
            // pinned variables (upper - lower <= EPS) can never improve -> skip.
            let mut s = usize::MAX;
            let mut dir = 0i32;
            let mut best = EPS;
            for j in 0..n {
                if self.upper[j] - self.lower[j] <= EPS {
                    continue;
                }
                match self.status[j] {
                    AT_LOWER => {
                        if -self.d[j] > best {
                            best = -self.d[j];
                            s = j;
                            dir = 1;
                        }
                    }
                    AT_UPPER => {
                        if self.d[j] > best {
                            best = self.d[j];
                            s = j;
                            dir = -1;
                        }
                    }
                    _ => {}
                }
            }
            if s == usize::MAX {
                return Ok(()); // optimal
            }

            // ratio test (three-way: basic var -> lower, basic var -> upper,
            // entering var -> opposite bound = bound flip)
            let mut t = self.upper[s] - self.lower[s];
            let mut r = usize::MAX;
            let mut leave_to_upper = false;
            let df = dir as f64;
            for i in 0..m {
                let alpha = self.a[i][s] * df; // x_B(i) changes by -alpha * step
                if alpha > EPS {
                    let ti = (self.beta[i] - self.lower[self.basis[i]]) / alpha;
                    if ti < t - EPS {
                        t = ti;
                        r = i;
                        leave_to_upper = false;
                    }
                } else if alpha < -EPS {
                    let ub = self.upper[self.basis[i]];
                    if ub.is_finite() {
                        let ti = (self.beta[i] - ub) / alpha;
                        if ti < t - EPS {
                            t = ti;
                            r = i;
                            leave_to_upper = true;
                        }
                    }
                }
            }

            if !t.is_finite() {
                return Err(SimplexError::Unbounded);
            }

            let dval = df * t;
            if r == usize::MAX {
                // bound flip of the entering variable, no basis change
                for i in 0..m {
                    self.beta[i] -= self.a[i][s] * dval;
                }
                self.status[s] = if dir > 0 { AT_UPPER } else { AT_LOWER };
            } else {
                let leaving = self.basis[r];
                let entering_val = self.nbval(s) + dval;
                let col_s: Vec<f64> = (0..m).map(|i| self.a[i][s]).collect();
                for (i, &value) in col_s.iter().enumerate() {
                    if i != r {
                        self.beta[i] -= value * dval;
                    }
                }
                self.pivot(r, s);
                self.basis[r] = s;
                self.status[leaving] = if leave_to_upper { AT_UPPER } else { AT_LOWER };
                self.status[s] = BASIC;
                self.beta[r] = entering_val;
            }
        }
    }

    // Dual simplex with bounded variables. Restores primal feasibility while
    // preserving dual feasibility. Returns Err(Infeasible) when no entering
    // column exists (the LP is primal-infeasible -> the B&B node has no tour).
    fn dual(&mut self) -> Result<(), SimplexError> {
        let m = self.m;
        let n = self.n;
        let mut iters = 0;
        loop {
            iters += 1;
            if iters > DUAL_CAP {
                return Err(SimplexError::Unbounded); // surfaced as a hard error
            }

            // leaving row: the basic variable with the largest bound violation
            let mut r = usize::MAX;
            let mut worst = EPS;
            let mut need_increase = false;
            for i in 0..m {
                let bvar = self.basis[i];
                let v = self.beta[i];
                if v < self.lower[bvar] - EPS {
                    let viol = self.lower[bvar] - v;
                    if viol > worst {
                        worst = viol;
                        r = i;
                        need_increase = true;
                    }
                } else if v > self.upper[bvar] + EPS {
                    let viol = v - self.upper[bvar];
                    if viol > worst {
                        worst = viol;
                        r = i;
                        need_increase = false;
                    }
                }
            }
            if r == usize::MAX {
                return Ok(()); // primal feasible
            }
            let bvar = self.basis[r];
            let leave_val = if need_increase {
                self.lower[bvar]
            } else {
                self.upper[bvar]
            };

            // entering variable via the dual ratio test over sign-eligible,
            // non-pinned nonbasic columns
            let mut s = usize::MAX;
            let mut bestratio = f64::INFINITY;
            for j in 0..n {
                if self.status[j] == BASIC {
                    continue;
                }
                if self.upper[j] - self.lower[j] <= EPS {
                    continue;
                }
                let arj = self.a[r][j];
                if arj.abs() <= EPS {
                    continue;
                }
                let st = self.status[j];
                let eligible = if need_increase {
                    (st == AT_LOWER && arj < -EPS) || (st == AT_UPPER && arj > EPS)
                } else {
                    (st == AT_LOWER && arj > EPS) || (st == AT_UPPER && arj < -EPS)
                };
                if eligible {
                    let ratio = (self.d[j] / arj).abs();
                    if ratio < bestratio - EPS {
                        bestratio = ratio;
                        s = j;
                    }
                }
            }
            if s == usize::MAX {
                return Err(SimplexError::Infeasible); // node has no feasible tour
            }

            let ars = self.a[r][s];
            let tau = (self.beta[r] - leave_val) / ars;
            let col_s: Vec<f64> = (0..m).map(|i| self.a[i][s]).collect();
            let entering_val = self.nbval(s) + tau;
            for (i, &value) in col_s.iter().enumerate() {
                if i != r {
                    self.beta[i] -= value * tau;
                }
            }
            self.pivot(r, s);
            let leaving = bvar;
            self.basis[r] = s;
            self.status[leaving] = if need_increase { AT_LOWER } else { AT_UPPER };
            self.status[s] = BASIC;
            self.beta[r] = entering_val;
        }
    }

    // Add subtour cut: sum_{e in cut} x_e >= 2  <=>  -sum x_e + sigma = -2,
    // sigma >= 0. After this the basis is primal-infeasible (sigma < 0) and the
    // dual simplex must be run to restore feasibility.
    fn add_subtour_cut(&mut self, cut_edges: &[usize]) {
        let old_n = self.n;

        for row in self.a.iter_mut() {
            row.push(0.0);
        }
        self.d.push(0.0);
        self.lower.push(0.0);
        self.upper.push(f64::INFINITY);
        self.status.push(BASIC);

        let mut new_row: Vec<f64> = vec![0.0; old_n + 1];
        for &col in cut_edges {
            new_row[col] = -1.0;
        }
        new_row[old_n] = 1.0;
        // eliminate current basic variables so basis columns stay identity
        for i in 0..self.m {
            let f = new_row[self.basis[i]];
            if f.abs() > EPS {
                for (j, value) in new_row.iter_mut().enumerate() {
                    *value -= f * self.a[i][j];
                }
            }
        }
        self.a.push(new_row);
        self.basis.push(old_n);

        // value of sigma at the current solution = sum_{cut} x_e - 2
        let mut sig = -2.0;
        for &col in cut_edges {
            sig += self.val(col);
        }
        self.beta.push(sig);

        self.m += 1;
        self.n += 1;
    }
}

// canonicalize an objective row given the (identity-on-basis) tableau
fn bv_canonicalize(a: &[Vec<f64>], d: &mut [f64], basis: &[usize]) {
    let n = d.len();
    for (i, &col) in basis.iter().enumerate() {
        let f = d[col];
        if f.abs() > EPS {
            for j in 0..n {
                d[j] -= f * a[i][j];
            }
        }
    }
}

// Build the initial relaxation (degree = 2, 0 <= x_e <= 1, edges fixed via
// bounds), run phase 1 (feasibility) and phase 2 (optimize with big-M on
// artificials). Returns `None` if the degree system itself is infeasible.
fn build_relaxation(
    problem: &TsplibInstance,
    fixed_edges: &HashMap<(usize, usize), bool>,
) -> Result<Option<Bv>, SolverError> {
    let node_count = problem.nodes.len();
    let e = node_count * (node_count - 1) / 2;
    let m = node_count; // degree constraints only
    let ntot = e + node_count; // edges + artificials
    let art_off = e;

    let mut a = vec![vec![0.0; ntot]; m];
    let b = vec![2.0; m];
    let mut lower = vec![0.0; ntot];
    let mut upper = vec![0.0; ntot];
    let mut cost = vec![0.0; ntot];
    for value in upper.iter_mut().take(e) {
        *value = 1.0;
    }
    for ai in 0..node_count {
        upper[art_off + ai] = f64::INFINITY;
    }

    // degree rows + artificial placeholders
    for v in 1..=node_count {
        for w in 1..=node_count {
            if w != v {
                a[v - 1][edge_col(v, w)] += 1.0;
            }
        }
        a[v - 1][art_off + (v - 1)] = 1.0;
    }
    // costs
    for i in 1..=node_count {
        for j in 1..i {
            cost[edge_index(i, j)] = problem.try_get_distance(i, j)? as f64;
        }
    }
    // fixed edges = bound tightening
    for (&(i, j), &f) in fixed_edges.iter() {
        let k = edge_col(i, j);
        if f {
            lower[k] = 1.0;
            upper[k] = 1.0;
        } else {
            upper[k] = 0.0;
        }
    }

    // initial residual per row (all structural vars start at their lower bound);
    // negate a row if its residual is negative so the artificial stays >= 0.
    let mut beta = vec![0.0; m];
    for i in 0..m {
        let mut r = b[i];
        for (k, &lower_value) in lower.iter().enumerate().take(e) {
            r -= a[i][k] * lower_value;
        }
        if r < 0.0 {
            for value in a[i].iter_mut().take(e) {
                *value = -*value;
            }
            r = -r;
        }
        beta[i] = r;
    }

    let mut status = vec![AT_LOWER; ntot];
    let mut basis = vec![0usize; m];
    for i in 0..m {
        basis[i] = art_off + i;
        status[art_off + i] = BASIC;
    }

    // phase 1: minimize sum of artificials
    let mut d = vec![0.0; ntot];
    for ai in 0..node_count {
        d[art_off + ai] = 1.0;
    }
    bv_canonicalize(&a, &mut d, &basis);

    let mut bv = Bv {
        m,
        n: ntot,
        a,
        basis,
        lower,
        upper,
        d,
        beta,
        status,
    };
    bv.primal()?;

    let p1: f64 = (0..node_count).map(|ai| bv.val(art_off + ai)).sum();
    if p1 > EPS {
        return Ok(None); // degree system infeasible (e.g. too many fixed edges at a node)
    }

    // phase 2: real costs + big-M on artificials (kept movable, bounds [0, inf))
    let big_m = 1.0 + cost.iter().map(|v| v.abs()).sum::<f64>();
    let mut d2 = cost;
    for ai in 0..node_count {
        d2[art_off + ai] = big_m;
    }
    bv_canonicalize(&bv.a, &mut d2, &bv.basis);
    bv.d = d2;
    bv.primal()?;

    Ok(Some(bv))
}

// ============================================================================
// cut machinery (identical to LinearProgram)
// ============================================================================
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

fn build_weight_matrix(x: &[f64], node_count: usize) -> Vec<Vec<f64>> {
    let mut w = vec![vec![0.0; node_count]; node_count];
    for i in 1..=node_count {
        for j in 1..i {
            let val = x[edge_index(i, j)];
            if val > EPS {
                w[i - 1][j - 1] = val;
                w[j - 1][i - 1] = val;
            }
        }
    }
    w
}

fn min_cut(weights: &[Vec<f64>], node_count: usize) -> (f64, HashSet<usize>) {
    let mut w = weights.to_vec();
    let mut active = (0..node_count).collect::<Vec<usize>>();
    let mut members = (0..node_count)
        .map(|i| HashSet::from([i]))
        .collect::<Vec<HashSet<usize>>>();

    let mut best_weight = f64::INFINITY;
    let mut best_set = HashSet::new();

    while active.len() > 1 {
        let mut added = vec![false; node_count];
        let mut weight_to_a = vec![0.0; node_count];
        let mut prev = usize::MAX;
        let mut last = usize::MAX;
        let mut last_weight = 0.0;

        for _ in 0..active.len() {
            let mut selected = usize::MAX;
            let mut selected_weight = f64::NEG_INFINITY;
            for &v in &active {
                if !added[v] && weight_to_a[v] > selected_weight {
                    selected_weight = weight_to_a[v];
                    selected = v;
                }
            }
            added[selected] = true;
            prev = last;
            last = selected;
            last_weight = selected_weight;
            for &u in &active {
                if !added[u] {
                    weight_to_a[u] += w[selected][u];
                }
            }
        }

        let (t, s) = (last, prev);
        if last_weight < best_weight {
            best_weight = last_weight;
            best_set = members[t].clone();
        }
        let t_row = w[t].clone();
        for u in 0..node_count {
            if u != s && u != t {
                w[s][u] += t_row[u];
                w[u][s] += t_row[u];
            }
        }
        let t_members = std::mem::take(&mut members[t]);
        members[s].extend(t_members);
        active.retain(|&x| x != t);
    }

    (best_weight, best_set)
}

// ============================================================================
// relaxation + branch and bound (same structure as LinearProgram)
// ============================================================================
// Run the subtour-cut separation loop on an already-optimized relaxation.
// Returns Ok(true) when no violated subtour remains (node optimal/feasible),
// Ok(false) when adding a cut renders the node infeasible (prune).
fn cut_loop(bv: &mut Bv, node_count: usize, e: usize) -> Result<bool, SolverError> {
    loop {
        let x = bv.edge_values(e);
        let w = build_weight_matrix(&x, node_count);
        let (cut_weight, cut_set) = min_cut(&w, node_count);
        if cut_weight >= 2.0 - EPS {
            return Ok(true);
        }
        let s: HashSet<usize> = cut_set.iter().map(|&i| i + 1).collect(); // 1-based
        let cut = cut_edges_for_set(node_count, &s);
        bv.add_subtour_cut(&cut);
        match bv.dual() {
            Ok(()) => {}
            Err(SimplexError::Infeasible) => return Ok(false),
            Err(other) => return Err(other.into()),
        }
        bv.primal()?;
    }
}

// Cold solve of the root relaxation: build + phase1/phase2 + cut loop.
// Returns the optimized tableau, or None if the node is infeasible.
fn solve_root(
    problem: &TsplibInstance,
    fixed_edges: &HashMap<(usize, usize), bool>,
) -> Result<Option<Bv>, SolverError> {
    let node_count = problem.nodes.len();
    let e = node_count * (node_count - 1) / 2;
    let mut bv = match build_relaxation(problem, fixed_edges)? {
        Some(bv) => bv,
        None => return Ok(None),
    };
    if !cut_loop(&mut bv, node_count, e)? {
        return Ok(None);
    }
    Ok(Some(bv))
}

// Warm-start re-solve after a single bound change has been applied to a cloned
// parent tableau. The basis is dual-feasible but primal-infeasible; the dual
// simplex restores feasibility, then the cut loop re-separates subtours.
// Returns Ok(true) if feasible/optimal, Ok(false) if the node is infeasible.
fn resolve_child(bv: &mut Bv, node_count: usize, e: usize) -> Result<bool, SolverError> {
    match bv.dual() {
        Ok(()) => {}
        Err(SimplexError::Infeasible) => return Ok(false),
        Err(other) => return Err(other.into()),
    }
    bv.primal()?;
    cut_loop(bv, node_count, e)
}

// Read the LP lower bound and fractional edge set out of an optimized tableau.
fn extract_result(
    bv: &Bv,
    problem: &TsplibInstance,
    e: usize,
) -> Result<LpRelaxationResult, SolverError> {
    let x = bv.edge_values(e);
    let mut edges = Vec::new();
    let mut lower_bound = 0.0;
    for (k, &val) in x.iter().enumerate() {
        if val > EPS {
            let (i, j) = index_to_edge(k);
            lower_bound += problem.try_get_distance(i, j)? as f64 * val;
            edges.push((i, j, val));
        }
    }
    Ok(LpRelaxationResult { lower_bound, edges })
}

// Cold single-node relaxation solve (public API, used by tests / external
// callers). The branch-and-bound loop itself uses the warm-start path.
pub fn _try_solve_lp_relaxation(
    problem: &TsplibInstance,
    fixed_edges: &HashMap<(usize, usize), bool>,
) -> Result<Option<LpRelaxationResult>, SolverError> {
    let e = problem.nodes.len() * (problem.nodes.len() - 1) / 2;
    match solve_root(problem, fixed_edges)? {
        Some(bv) => Ok(Some(extract_result(&bv, problem, e)?)),
        None => Ok(None),
    }
}

// Reconstruct the fixed-edge map from a tableau's variable bounds (an edge with
// upper == 0 is fixed to 0, lower == 1 is fixed to 1). Used to cold-rebuild a
// node when a warm-start re-solve fails numerically.
fn recover_fixed_edges(bv: &Bv, e: usize) -> HashMap<(usize, usize), bool> {
    let mut fixed = HashMap::new();
    for k in 0..e {
        if bv.upper[k] <= EPS {
            fixed.insert(index_to_edge(k), false);
        } else if bv.lower[k] >= 1.0 - EPS {
            fixed.insert(index_to_edge(k), true);
        }
    }
    fixed
}

fn find_fractional_edge(edges: &[(usize, usize, f64)]) -> Option<(usize, usize)> {
    edges
        .iter()
        .find(|&&(_, _, val)| val > EPS && val < 1.0 - EPS)
        .map(|&(i, j, _)| (i, j))
}

fn reconstruct_tour(
    problem: &TsplibInstance,
    edges: &[(usize, usize)],
) -> Result<TspSolution, SolverError> {
    let node_count = problem.nodes.len();

    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); node_count + 1];
    for &(i, j) in edges {
        adjacency[i].push(j);
        adjacency[j].push(i);
    }

    for (v, neighbors) in adjacency.iter().enumerate().skip(1) {
        if neighbors.len() != 2 {
            return Err(SolverError::SimplexError(format!(
                "Expected degree 2 for vertex {v}, got {}",
                adjacency[v].len()
            )));
        }
    }

    let mut tour = Vec::with_capacity(node_count);
    let mut prev = 0;
    let mut current = 1;

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
    for w in 0..node_count {
        let from = tour[w];
        let to = tour[(w + 1) % node_count];
        cost += problem.try_get_distance(from, to)? as i64;
    }

    Ok(TspSolution { tour, cost })
}

fn branch_and_bound(
    problem: &TsplibInstance,
    initial_fixed: &HashMap<(usize, usize), bool>,
) -> Result<Option<TspSolution>, SolverError> {
    let node_count = problem.nodes.len();
    let e = node_count * (node_count - 1) / 2;
    let mut best: Option<(f64, Vec<(usize, usize)>)> = None;

    // Root is solved cold; every stack entry is an already-optimized tableau
    // (its relaxation solved, no violated subtour). Children inherit the parent
    // tableau and warm-start via the dual simplex after a single bound change.
    let root = match solve_root(problem, initial_fixed)? {
        Some(bv) => bv,
        None => return Ok(None),
    };
    let mut stack: Vec<Bv> = vec![root];

    while let Some(bv) = stack.pop() {
        let result = extract_result(&bv, problem, e)?;

        if let Some((best_cost, _)) = &best
            && result.lower_bound >= *best_cost - EPS
        {
            tracing::debug!(best_cost = ?best_cost, lower_bound = ?result.lower_bound, "Pruning branch with worse lower bound");
            continue;
        }

        match find_fractional_edge(&result.edges) {
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

            Some((i, j)) => {
                tracing::debug!(edge = ?(i, j), "Branching on fractional edge");
                let k = edge_col(i, j);

                // forbid: fix edge to 0 (l = u = 0)
                let mut forbid = bv.clone();
                forbid.lower[k] = 0.0;
                forbid.upper[k] = 0.0;
                match resolve_child(&mut forbid, node_count, e) {
                    Ok(true) => stack.push(forbid),
                    Ok(false) => {}
                    Err(_) => {
                        // warm-start failed (numerical drift): rebuild cold
                        let fixed = recover_fixed_edges(&forbid, e);
                        if let Some(b) = solve_root(problem, &fixed)? {
                            stack.push(b);
                        }
                    }
                }

                // force: fix edge to 1 (l = u = 1)
                let mut force = bv.clone();
                force.lower[k] = 1.0;
                force.upper[k] = 1.0;
                match resolve_child(&mut force, node_count, e) {
                    Ok(true) => stack.push(force),
                    Ok(false) => {}
                    Err(_) => {
                        let fixed = recover_fixed_edges(&force, e);
                        if let Some(b) = solve_root(problem, &fixed)? {
                            stack.push(b);
                        }
                    }
                }
            }
        }
    }

    match best {
        Some((_, edges)) => Ok(Some(reconstruct_tour(problem, &edges)?)),
        None => Ok(None),
    }
}
