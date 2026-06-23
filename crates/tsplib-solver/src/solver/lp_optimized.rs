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
        ctx: ExecutionContext,
        _: SolverOptions,
    ) -> Result<TspSolution, SolverError> {
        let fixed_edges = initial_fixed_edges(problem);

        match branch_and_bound(problem, &fixed_edges, ctx)? {
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
// Consecutive degenerate pivots (zero-length steps making no progress) tolerated
// before declaring a stall/cycle and bailing to a cold rebuild. Legitimate solves
// make progress and never approach this; a Dantzig-rule cycle is unbounded.
const STALL_LIMIT: usize = 20_000;
const PIV: f64 = 1e-7; // pivot acceptance threshold for revised pricing / ratio test

// Sparse LU factorization of the basis (left-looking Gilbert-Peierls, partial
// pivoting). Replaces the dense O(m^2)-per-pivot inverse: triangular solves are
// O(nnz(L)+nnz(U)) and a refactorization is O(fill) instead of O(m^3), so it can
// be rebuilt often enough to keep the eta chain short.
#[derive(Clone)]
struct Lu {
    m: usize,
    lcol: Vec<Vec<(usize, f64)>>, // L columns (unit lower, orig rows, diagonal implicit)
    ucol: Vec<Vec<(usize, f64)>>, // U columns (upper, incl. pivot at pivrow[j])
    pivrow: Vec<usize>,           // pivrow[j] = original row chosen as pivot for column j
    pivval: Vec<f64>,             // pivot (diagonal) value for column j
    pinv: Vec<i64>,               // pinv[orig_row] = column where it pivots, else -1
}

// product-form update: B^{-1} = E_k .. E_1 B_ref^{-1}; each eta E_t replaces basis
// column `row` with the pivot column `alpha` (stored sparsely) captured at pivot time.
#[derive(Clone)]
struct Eta {
    row: usize,
    piv: f64,
    alpha: Vec<(usize, f64)>,
}

// Factor an m x m matrix given as sparse columns (orig_row, val). None if singular.
fn lu_factor(acols: &[Vec<(usize, f64)>], m: usize) -> Option<Lu> {
    let mut lcol: Vec<Vec<(usize, f64)>> = vec![Vec::new(); m];
    let mut ucol: Vec<Vec<(usize, f64)>> = vec![Vec::new(); m];
    let mut pivrow = vec![usize::MAX; m];
    let mut pivval = vec![0.0f64; m];
    let mut pinv = vec![-1i64; m];
    let mut x = vec![0.0f64; m];
    let mut onstack = vec![false; m];
    for j in 0..m {
        for &(i, v) in &acols[j] {
            x[i] += v;
        }
        let mut stack: Vec<(usize, usize)> = Vec::new();
        let mut reach: Vec<usize> = Vec::new();
        for &(s, _) in &acols[j] {
            if onstack[s] {
                continue;
            }
            stack.push((s, 0));
            onstack[s] = true;
            while let Some(&(row, ci)) = stack.last() {
                let k = pinv[row];
                if k < 0 {
                    reach.push(row);
                    stack.pop();
                    continue;
                }
                let kk = k as usize;
                if ci < lcol[kk].len() {
                    stack.last_mut().unwrap().1 += 1;
                    let (child, _) = lcol[kk][ci];
                    if !onstack[child] {
                        onstack[child] = true;
                        stack.push((child, 0));
                    }
                } else {
                    reach.push(row);
                    stack.pop();
                }
            }
        }
        for &r in &reach {
            onstack[r] = false;
        }
        let mut piv_rows: Vec<usize> = reach.iter().cloned().filter(|&r| pinv[r] >= 0).collect();
        piv_rows.sort_by_key(|&r| pinv[r]);
        for &i in &piv_rows {
            let xi = x[i];
            if xi != 0.0 {
                let k = pinv[i] as usize;
                for &(r, lval) in &lcol[k] {
                    x[r] -= xi * lval;
                }
            }
        }
        let mut pr = usize::MAX;
        let mut best = 0.0f64;
        for &r in &reach {
            if pinv[r] < 0 && x[r].abs() > best {
                best = x[r].abs();
                pr = r;
            }
        }
        if pr == usize::MAX || best < 1e-12 {
            for &r in &reach {
                x[r] = 0.0;
            }
            return None;
        }
        let pv = x[pr];
        let mut ucj: Vec<(usize, f64)> = Vec::new();
        for &r in &reach {
            if pinv[r] >= 0 && x[r] != 0.0 {
                ucj.push((r, x[r]));
            }
        }
        ucj.push((pr, pv));
        let mut lcj: Vec<(usize, f64)> = Vec::new();
        for &r in &reach {
            if pinv[r] < 0 && r != pr && x[r] != 0.0 {
                lcj.push((r, x[r] / pv));
            }
        }
        ucol[j] = ucj;
        lcol[j] = lcj;
        pivrow[j] = pr;
        pivval[j] = pv;
        pinv[pr] = j as i64;
        for &r in &reach {
            x[r] = 0.0;
        }
    }
    Some(Lu {
        m,
        lcol,
        ucol,
        pivrow,
        pivval,
        pinv,
    })
}

#[derive(Clone)]
struct Bv {
    m: usize,
    n: usize,
    cols: Vec<Vec<(usize, f64)>>, // sparse columns of A
    c: Vec<f64>,                  // objective
    lower: Vec<f64>,
    upper: Vec<f64>,
    b: Vec<f64>, // rhs
    basis: Vec<usize>,
    status: Vec<u8>,
    lu: Lu,             // sparse LU of the basis at last refactorization
    etas: Vec<Eta>,     // product-form updates applied since then
    xb: Vec<f64>,       // basic variable values
    since_refac: usize, // warm re-solves since the last exact refactorization
}

impl Bv {
    fn nbval(&self, j: usize) -> f64 {
        match self.status[j] {
            AT_UPPER => self.upper[j],
            _ => self.lower[j],
        }
    }
    fn val(&self, j: usize) -> f64 {
        if self.status[j] == BASIC {
            for r in 0..self.m {
                if self.basis[r] == j {
                    return self.xb[r];
                }
            }
            0.0
        } else {
            self.nbval(j)
        }
    }
    fn edge_values(&self, e: usize) -> Vec<f64> {
        let mut val = vec![0.0; e];
        for (j, vj) in val.iter_mut().enumerate() {
            *vj = self.nbval(j);
        }
        for r in 0..self.m {
            let bj = self.basis[r];
            if bj < e {
                val[bj] = self.xb[r];
            }
        }
        val
    }
    // base solve B_ref x = rhs via the LU factors (x indexed by basis position).
    fn ftran_lu(&self, rhs: &[f64]) -> Vec<f64> {
        let lu = &self.lu;
        let m = lu.m;
        let mut y = vec![0.0f64; m];
        for k in 0..m {
            y[k] = rhs[lu.pivrow[k]];
        }
        for k in 0..m {
            let yk = y[k];
            if yk != 0.0 {
                for &(r, lval) in &lu.lcol[k] {
                    y[lu.pinv[r] as usize] -= lval * yk;
                }
            }
        }
        let mut x = y;
        for j in (0..m).rev() {
            let xj = x[j] / lu.pivval[j];
            x[j] = xj;
            if xj != 0.0 {
                for &(r, uval) in &lu.ucol[j] {
                    if r != lu.pivrow[j] {
                        x[lu.pinv[r] as usize] -= uval * xj;
                    }
                }
            }
        }
        x
    }
    // base solve B_ref^T x = rhs via the LU factors (rhs in basis-position space,
    // x indexed by original constraint row).
    fn btran_lu(&self, rhs: &[f64]) -> Vec<f64> {
        let lu = &self.lu;
        let m = lu.m;
        let mut w = rhs.to_vec();
        for a in 0..m {
            let mut s = w[a];
            for &(r, val) in &lu.ucol[a] {
                if r != lu.pivrow[a] {
                    s -= val * w[lu.pinv[r] as usize];
                }
            }
            w[a] = s / lu.pivval[a];
        }
        let mut z = w;
        for a in (0..m).rev() {
            let mut s = z[a];
            for &(r, lval) in &lu.lcol[a] {
                s -= lval * z[lu.pinv[r] as usize];
            }
            z[a] = s;
        }
        let mut x = vec![0.0f64; m];
        for k in 0..m {
            x[lu.pivrow[k]] = z[k];
        }
        x
    }
    // apply the eta chain forward: u <- E_k .. E_1 u (chronological order)
    fn apply_etas_ftran(&self, u: &mut [f64]) {
        for eta in &self.etas {
            let r = eta.row;
            let tmp = u[r] / eta.piv;
            for &(i, av) in &eta.alpha {
                if i != r {
                    u[i] -= av * tmp;
                }
            }
            u[r] = tmp;
        }
    }
    // apply the eta chain backward: v <- v E_k .. E_1 (reverse chronological)
    fn apply_etas_btran(&self, v: &mut [f64]) {
        for eta in self.etas.iter().rev() {
            let r = eta.row;
            let mut s = 0.0;
            for &(i, av) in &eta.alpha {
                if i != r {
                    s += av * v[i];
                }
            }
            v[r] = (v[r] - s) / eta.piv;
        }
    }
    // record a product-form update for a pivot of `alpha = B^{-1} A_q` on row r.
    fn push_eta(&mut self, r: usize, alpha: &[f64]) {
        let mut sp = Vec::new();
        for (i, &av) in alpha.iter().enumerate() {
            if av != 0.0 {
                sp.push((i, av));
            }
        }
        self.etas.push(Eta {
            row: r,
            piv: alpha[r],
            alpha: sp,
        });
    }
    // xb = B^{-1} (b - N x_N)
    fn recompute_xb(&mut self) {
        let mut rhs = self.b.clone();
        for j in 0..self.n {
            if self.status[j] != BASIC {
                let v = self.nbval(j);
                if v != 0.0 {
                    for &(i, a) in &self.cols[j] {
                        rhs[i] -= a * v;
                    }
                }
            }
        }
        let mut u = self.ftran_lu(&rhs);
        self.apply_etas_ftran(&mut u);
        self.xb = u;
    }
    // Rebuild the sparse LU exactly from the current basis and drop the eta chain,
    // clearing the numerical drift that accumulates across long pivot chains.
    // Returns false if the basis is singular.
    fn refactorize(&mut self) -> bool {
        let acols: Vec<Vec<(usize, f64)>> =
            self.basis.iter().map(|&v| self.cols[v].clone()).collect();
        match lu_factor(&acols, self.m) {
            Some(lu) => {
                self.lu = lu;
                self.etas.clear();
                self.recompute_xb();
                true
            }
            None => false,
        }
    }
    // y = c_B B^{-1}
    fn multipliers(&self) -> Vec<f64> {
        let mut cb = vec![0.0f64; self.m];
        // for k in 0..self.m {
        //     cb[k] = self.c[self.basis[k]];
        // }
        for (k, cbk) in cb.iter_mut().enumerate() {
            *cbk = self.c[self.basis[k]];
        }
        self.apply_etas_btran(&mut cb);
        self.btran_lu(&cb)
    }
    fn reduced_cost(&self, j: usize, y: &[f64]) -> f64 {
        let mut d = self.c[j];
        for &(i, a) in &self.cols[j] {
            d -= y[i] * a;
        }
        d
    }
    // B^{-1} A_j (indexed by basis position)
    fn ftran(&self, j: usize) -> Vec<f64> {
        let mut a = vec![0.0f64; self.m];
        for &(i, x) in &self.cols[j] {
            a[i] += x;
        }
        let mut u = self.ftran_lu(&a);
        self.apply_etas_ftran(&mut u);
        u
    }
    // row r of B^{-1} (indexed by constraint row): rho = e_r B^{-1}
    fn btran_unit(&self, r: usize) -> Vec<f64> {
        let mut v = vec![0.0f64; self.m];
        v[r] = 1.0;
        self.apply_etas_btran(&mut v);
        self.btran_lu(&v)
    }
    // reduced costs d_j = c_j - y A_j for the current objective and basis
    fn init_d(&self) -> Vec<f64> {
        let y = self.multipliers();
        (0..self.n).map(|j| self.reduced_cost(j, &y)).collect()
    }
    // incremental reduced-cost update after a basis-change pivot: entering var q
    // enters at the row whose OLD inverse row is `rho`, pivot element `pivot`.
    // d_j -= (d_q / pivot) * (rho . A_j) for all j (q -> 0, leaving var -> -d_q/pivot).
    fn update_d(&self, d: &mut [f64], rho: &[f64], q: usize, pivot: f64) {
        let theta = d[q] / pivot;
        if theta != 0.0 {
            for (j, dj) in d.iter_mut().enumerate() {
                let mut arj = 0.0;
                for &(i, a) in &self.cols[j] {
                    arj += rho[i] * a;
                }
                if arj != 0.0 {
                    *dj -= theta * arj;
                }
            }
        }
        d[q] = 0.0;
    }
    fn primal(&mut self) -> Result<(), SimplexError> {
        let mut d = self.init_d();
        let mut iters = 0usize;
        let mut stall = 0usize;
        loop {
            iters += 1;
            if iters > PRIMAL_CAP {
                return Err(SimplexError::Unbounded);
            }
            let mut entering = usize::MAX;
            let mut dir = 0.0f64;
            let mut best = PIV;
            for (j, dj) in d.iter_mut().enumerate().take(self.n) {
                if self.upper[j] - self.lower[j] <= EPS {
                    continue; // pinned (fixed edge / artificial): cannot enter
                }
                match self.status[j] {
                    AT_LOWER => {
                        if -*dj > best {
                            best = -*dj;
                            entering = j;
                            dir = 1.0;
                        }
                    }
                    AT_UPPER => {
                        if *dj > best {
                            best = *dj;
                            entering = j;
                            dir = -1.0;
                        }
                    }
                    _ => {}
                }
            }
            if entering == usize::MAX {
                return Ok(());
            }
            let alpha = self.ftran(entering);
            let mut t = f64::INFINITY;
            let range = self.upper[entering] - self.lower[entering];
            if range.is_finite() {
                t = range;
            }
            let mut leave = usize::MAX;
            let mut leave_to_upper = false;
            for (r, &alpha_r) in alpha.iter().enumerate().take(self.m) {
                let a = dir * alpha_r;
                if a > PIV {
                    let room = self.xb[r] - self.lower[self.basis[r]];
                    let step = room / a;
                    if step < t - EPS {
                        t = step;
                        leave = r;
                        leave_to_upper = false;
                    }
                } else if a < -PIV {
                    let room = self.upper[self.basis[r]] - self.xb[r];
                    let step = room / (-a);
                    if step < t - EPS {
                        t = step;
                        leave = r;
                        leave_to_upper = true;
                    }
                }
            }
            if !t.is_finite() {
                return Err(SimplexError::Unbounded);
            }
            if t <= EPS {
                stall += 1;
                if stall > STALL_LIMIT {
                    // prolonged zero-progress pivoting: treat as a cycle and bail
                    // to a cold rebuild rather than grinding to the iteration cap.
                    return Err(SimplexError::Unbounded);
                }
            } else {
                stall = 0;
            }
            let delta = dir * t;
            for (r, xr) in self.xb.iter_mut().enumerate() {
                *xr -= alpha[r] * delta;
            }
            if leave == usize::MAX {
                self.status[entering] = if dir > 0.0 { AT_UPPER } else { AT_LOWER };
            } else {
                let leaving_var = self.basis[leave];
                let entering_val = self.nbval(entering) + delta;
                let pivot = alpha[leave];
                let rho = self.btran_unit(leave);
                self.push_eta(leave, &alpha);
                self.basis[leave] = entering;
                self.status[leaving_var] = if leave_to_upper { AT_UPPER } else { AT_LOWER };
                self.status[entering] = BASIC;
                self.xb[leave] = entering_val;
                self.update_d(&mut d, &rho, entering, pivot);
                if self.etas.len() >= ETA_LIMIT && !self.refactorize() {
                    return Err(SimplexError::Unbounded);
                }
            }
        }
    }
    fn dual(&mut self) -> Result<(), SimplexError> {
        let mut d = self.init_d();
        let mut iters = 0usize;
        let mut stall = 0usize;
        loop {
            iters += 1;
            if iters > DUAL_CAP {
                return Err(SimplexError::Unbounded);
            }
            let mut r = usize::MAX;
            let mut worst = EPS;
            let mut need_increase = false;
            for i in 0..self.m {
                let bvar = self.basis[i];
                let v = self.xb[i];
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
                return Ok(());
            }
            let bvar = self.basis[r];
            let leave_val = if need_increase {
                self.lower[bvar]
            } else {
                self.upper[bvar]
            };
            let rho = self.btran_unit(r);
            let mut s = usize::MAX;
            let mut bestratio = f64::INFINITY;
            for (j, _) in self.cols.iter().enumerate() {
                if self.status[j] == BASIC {
                    continue;
                }
                if self.upper[j] - self.lower[j] <= EPS {
                    continue;
                }
                let mut arj = 0.0;
                for &(i, a) in &self.cols[j] {
                    arj += rho[i] * a;
                }
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
                    let ratio = (d[j] / arj).abs();
                    if ratio < bestratio - EPS {
                        bestratio = ratio;
                        s = j;
                    }
                }
            }
            if s == usize::MAX {
                return Err(SimplexError::Infeasible);
            }
            if bestratio <= EPS {
                stall += 1;
                if stall > STALL_LIMIT {
                    // dual-degenerate cycling: bail to a cold rebuild. Must NOT be
                    // Infeasible (resolve_child would wrongly prune the node).
                    return Err(SimplexError::Unbounded);
                }
            } else {
                stall = 0;
            }
            let alpha = self.ftran(s);
            let ars = alpha[r];
            let tau = (self.xb[r] - leave_val) / ars;
            let entering_val = self.nbval(s) + tau;
            for (i, xi) in self.xb.iter_mut().enumerate() {
                if i != r {
                    *xi -= alpha[i] * tau;
                }
            }
            self.push_eta(r, &alpha);
            self.basis[r] = s;
            self.status[bvar] = if need_increase { AT_LOWER } else { AT_UPPER };
            self.status[s] = BASIC;
            self.xb[r] = entering_val;
            self.update_d(&mut d, &rho, s, ars);
            if self.etas.len() >= ETA_LIMIT && !self.refactorize() {
                return Err(SimplexError::Unbounded);
            }
        }
    }
    // fix variable k to a value (branching): pin its bounds. For a nonbasic var
    // the basic values shift by (B^{-1} A_k) * delta — an O(m) rank-1 update
    // instead of a full O(m^2) recompute. A basic var keeps its xb entry; the
    // dual simplex then repairs the now-violated [val,val] bound.
    fn fix_var(&mut self, k: usize, val: f64) {
        if self.status[k] == BASIC {
            self.lower[k] = val;
            self.upper[k] = val;
        } else {
            let old = self.nbval(k);
            self.lower[k] = val;
            self.upper[k] = val;
            let delta = val - old;
            if delta != 0.0 {
                let alpha = self.ftran(k);
                for (xi, &ai) in self.xb.iter_mut().zip(alpha.iter()) {
                    *xi -= ai * delta;
                }
            }
        }
    }
    // generic cut row  sum coeff*x {>=,<=} rhs  with a fresh basic slack. The new
    // row/column extend the basis; a sparse refactorization rebuilds the LU at the
    // new dimension (cheap) and recomputes xb. A violated cut leaves the slack
    // primal-infeasible / dual-feasible for the dual simplex to repair.
    fn add_row(&mut self, coeffs: &[(usize, f64)], rhs: f64, ge: bool) {
        let m_old = self.m;
        let sigma = if ge { -1.0 } else { 1.0 };
        for &(col, co) in coeffs {
            self.cols[col].push((m_old, co));
        }
        let slack = self.n;
        self.cols.push(vec![(m_old, sigma)]);
        self.c.push(0.0);
        self.lower.push(0.0);
        self.upper.push(f64::INFINITY);
        self.status.push(BASIC);
        self.b.push(rhs);
        self.basis.push(slack);
        self.m += 1;
        self.n += 1;
        self.xb.push(0.0); // overwritten by recompute_xb in refactorize
        self.refactorize();
    }
    // subtour elimination  x(delta(S)) >= 2
    fn add_subtour_cut(&mut self, cut_edges: &[usize]) {
        let coeffs: Vec<(usize, f64)> = cut_edges.iter().map(|&k| (k, 1.0)).collect();
        self.add_row(&coeffs, 2.0, true);
    }
    // blossom / general <= cut
    fn add_le_cut(&mut self, coeffs: &[(usize, f64)], rhs: f64) {
        self.add_row(coeffs, rhs, false);
    }
}

fn build_relaxation(
    problem: &TsplibInstance,
    fixed_edges: &HashMap<(usize, usize), bool>,
) -> Result<Option<Bv>, SolverError> {
    let node_count = problem.nodes.len();
    let e = node_count * (node_count - 1) / 2;
    let m = node_count; // degree constraints only
    let art_off = e;
    let ntot = e + node_count; // edges + artificials

    // edge bounds; fixed edges tighten them
    let mut lower = vec![0.0; ntot];
    let mut upper = vec![1.0; ntot];
    for ai in 0..node_count {
        upper[art_off + ai] = f64::INFINITY;
    }
    for (&(i, j), &f) in fixed_edges.iter() {
        let k = edge_col(i, j);
        if f {
            lower[k] = 1.0;
            upper[k] = 1.0;
        } else {
            upper[k] = 0.0;
        }
    }

    // residual per degree row (structural vars at lower bound); negate a row if
    // its residual is negative so the artificial stays >= 0.
    let mut resid = vec![2.0f64; m];
    for i in 2..=node_count {
        for j in 1..i {
            let l = lower[edge_index(i, j)];
            if l != 0.0 {
                resid[i - 1] -= l;
                resid[j - 1] -= l;
            }
        }
    }
    let sign: Vec<f64> = resid
        .iter()
        .map(|&r| if r < 0.0 { -1.0 } else { 1.0 })
        .collect();

    // sparse columns: edges then artificials
    let mut cols: Vec<Vec<(usize, f64)>> = Vec::with_capacity(ntot);
    let mut c = vec![0.0; ntot];
    for i in 2..=node_count {
        for j in 1..i {
            let k = edge_index(i, j);
            cols.push(vec![(i - 1, sign[i - 1]), (j - 1, sign[j - 1])]);
            c[k] = problem.try_get_distance(i, j)? as f64;
        }
    }
    for ai in 0..node_count {
        cols.push(vec![(ai, 1.0)]); // artificial column (identity)
    }
    let b: Vec<f64> = (0..m).map(|i| sign[i] * 2.0).collect();

    let mut status = vec![AT_LOWER; ntot];
    let mut basis = vec![0usize; m];
    for i in 0..m {
        basis[i] = art_off + i;
        status[art_off + i] = BASIC;
    }
    let mut bv = Bv {
        m,
        n: ntot,
        cols,
        c,
        lower,
        upper,
        b,
        basis,
        status,
        lu: Lu {
            m,
            lcol: Vec::new(),
            ucol: Vec::new(),
            pivrow: Vec::new(),
            pivval: Vec::new(),
            pinv: Vec::new(),
        },
        etas: Vec::new(),
        xb: vec![0.0; m],
        since_refac: 0,
    };
    bv.refactorize(); // factor the initial (identity) basis and set xb

    // phase 1: minimize sum of artificials
    let saved_c = bv.c.clone();
    for j in 0..ntot {
        bv.c[j] = if j >= art_off { 1.0 } else { 0.0 };
    }
    bv.primal()?;
    let p1: f64 = (0..node_count).map(|ai| bv.val(art_off + ai)).sum();
    if p1 > EPS {
        return Ok(None); // degree system infeasible (e.g. too many fixed edges at a node)
    }

    // phase 2: real costs, artificials pinned to zero
    bv.c = saved_c;
    for j in art_off..ntot {
        bv.upper[j] = 0.0;
        if bv.status[j] != BASIC {
            bv.status[j] = AT_LOWER;
        }
    }
    bv.recompute_xb();
    bv.primal()?;
    Ok(Some(bv))
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

fn build_sparse_adj(x: &[f64], node_count: usize) -> Vec<Vec<(usize, f64)>> {
    let mut adj: Vec<Vec<(usize, f64)>> = vec![Vec::new(); node_count];
    for i in 1..=node_count {
        for j in 1..i {
            let val = x[edge_index(i, j)];
            if val > EPS {
                adj[i - 1].push((j - 1, val));
                adj[j - 1].push((i - 1, val));
            }
        }
    }
    adj
}

// Global minimum cut via Stoer-Wagner on the sparse support graph. The TSP
// relaxation support is near 2-regular (~n edges, not n^2), so a heap-driven
// maximum-adjacency ordering runs in ~O(n^2 log n) instead of the dense O(n^3).
// `adj0[v]` lists (neighbor, weight) for the symmetric support graph.
fn min_cut(adj0: &[Vec<(usize, f64)>], node_count: usize) -> (f64, HashSet<usize>) {
    use std::cmp::Ordering;
    use std::collections::BinaryHeap;

    // f64 wrapper so it can live in a max-heap (total order on finite weights)
    struct Key(f64, usize);
    impl PartialEq for Key {
        fn eq(&self, o: &Self) -> bool {
            self.0 == o.0 && self.1 == o.1
        }
    }
    impl Eq for Key {}
    impl PartialOrd for Key {
        fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
            Some(self.cmp(o))
        }
    }
    impl Ord for Key {
        fn cmp(&self, o: &Self) -> Ordering {
            self.0.total_cmp(&o.0).then(self.1.cmp(&o.1))
        }
    }

    let mut adj: Vec<HashMap<usize, f64>> = vec![HashMap::new(); node_count];
    for (u, lst) in adj0.iter().enumerate() {
        for &(v, w) in lst {
            if w > 0.0 {
                *adj[u].entry(v).or_insert(0.0) += w;
            }
        }
    }
    let mut active = vec![true; node_count];
    let mut members: Vec<HashSet<usize>> = (0..node_count).map(|i| HashSet::from([i])).collect();
    let mut num_active = node_count;
    let mut best_weight = f64::INFINITY;
    let mut best_set = HashSet::new();

    while num_active > 1 {
        // maximum-adjacency ordering driven by a lazy-deletion max-heap
        let mut added = vec![false; node_count];
        let mut key = vec![0.0f64; node_count];
        let mut heap: BinaryHeap<Key> = BinaryHeap::new();
        let seed = (0..node_count).find(|&i| active[i]).unwrap();
        added[seed] = true;
        let mut order_prev = usize::MAX;
        let mut order_last = seed;
        let mut last_key = 0.0;
        let mut count_added = 1;
        for (&v, &w) in &adj[seed] {
            if active[v] && !added[v] {
                key[v] += w;
                heap.push(Key(key[v], v));
            }
        }
        while count_added < num_active {
            let mut sel = usize::MAX;
            while let Some(Key(k, v)) = heap.pop() {
                if !added[v] && active[v] && k == key[v] {
                    sel = v;
                    last_key = k;
                    break;
                }
            }
            if sel == usize::MAX {
                // remaining nodes are disconnected from A (key 0): cut of phase 0
                sel = (0..node_count).find(|&i| active[i] && !added[i]).unwrap();
                last_key = 0.0;
            }
            added[sel] = true;
            order_prev = order_last;
            order_last = sel;
            count_added += 1;
            for (&v, &w) in &adj[sel] {
                if active[v] && !added[v] {
                    key[v] += w;
                    heap.push(Key(key[v], v));
                }
            }
        }
        let t = order_last;
        let s = order_prev;
        if last_key < best_weight {
            best_weight = last_key;
            best_set = members[t].clone();
        }
        // merge t into s (sum parallel edges, drop the s-t link)
        let t_adj: Vec<(usize, f64)> = adj[t].iter().map(|(&v, &w)| (v, w)).collect();
        for (v, w) in t_adj {
            adj[v].remove(&t);
            if v != s {
                *adj[s].entry(v).or_insert(0.0) += w;
                *adj[v].entry(s).or_insert(0.0) += w;
            }
        }
        adj[s].remove(&t);
        adj[t].clear();
        let tm = std::mem::take(&mut members[t]);
        members[s].extend(tm);
        active[t] = false;
        num_active -= 1;
    }

    (best_weight, best_set)
}

// ============================================================================
// relaxation + branch and bound (same structure as LinearProgram)
// ============================================================================
const CUT_EPS: f64 = 1e-3; // minimum violation to add a cut (guards vs. numeric noise / cycling)
const MAX_BLOSSOM_CUTS: usize = 40; // safety cap on blossom rounds per node
const REFACTOR_INTERVAL: usize = 24; // refactorize B^-1 every N cut-loop rounds
const REFACTOR_DEPTH: usize = 16; // refactorize after this many inherited warm re-solves
const ETA_LIMIT: usize = 48; // refactorize the sparse LU once the eta chain reaches this length
const CUT_LOOP_FACTOR: usize = 6; // per-node cut-round cap = FACTOR * node_count + base
const CUT_LOOP_BASE: usize = 400;
// Minimum violation required to separate a cut. Separating cuts with arbitrarily
// tiny violations causes "tailing off": a hard fractional point spawns thousands
// of marginal subtour cuts, each costing an O(n^3) min-cut + re-solve, which
// stalls the node for minutes. Integral subtours always have violation 2 (their
// cut value is 0), so they are still separated; only marginal fractional
// violations are skipped and left to branching.
const MIN_CUT_VIOLATION: f64 = 0.02;
// Blossom separation is skipped entirely for instances smaller than this. Small
// TSPs solve quickly under subtour cuts alone, and the extra cuts there only slow
// the per-node LP without shrinking the tree (pr76: ~21s without vs ~87s with).
// Large instances are the opposite -- blossoms collapse the tree by orders of
// magnitude (a280: ~80min without vs ~2.5s with). Set very high to disable
// blossoms globally (recovers the pure subtour-cut behaviour everywhere).
const BLOSSOM_MIN_N: usize = 120;
// Only separate *new* blossoms at tree depth <= CUT_DEPTH; deeper nodes still
// inherit every cut their ancestors added (cuts ride the cloned tableau). Default
// is unlimited; lower it to bound the number of cut rows -- hence memory -- on
// very large instances.
const CUT_DEPTH: u32 = u32::MAX;

// Cut aging: once a node carries more than CUT_AGING_FACTOR * node_count
// constraint rows (degree rows + inherited cut rows), its children are rebuilt
// cold from their fixed edges instead of warm-started. The cold rebuild
// re-separates only the cuts actually violated at that node, so the inherited
// cuts that are no longer active are dropped -- bounding m, and with it the
// O(m^2) basis-inverse memory and O(m^3) refactorization cost on deep paths.
const CUT_AGING_FACTOR: usize = 4;

// How often to log search progress (incumbent, global lower bound, optimality
// gap). Purely informational: the search always runs to a proven optimum and is
// interrupted only by ExecutionContext cancellation.
const PROGRESS_INTERVAL: u64 = 256;

// size + depth gate. cut_depth is taken as a parameter so that CUT_DEPTH = u32::MAX
// stays lint-clean (no constant `x <= T::MAX` comparison).
fn blossoms_on(node_count: usize, node_depth: u32, cut_depth: u32) -> bool {
    node_count >= BLOSSOM_MIN_N && node_depth <= cut_depth
}

// union-find root with path halving
fn uf_find(parent: &mut [usize], mut a: usize) -> usize {
    while parent[a] != a {
        parent[a] = parent[parent[a]];
        a = parent[a];
    }
    a
}

// Heuristic separation of 2-matching (blossom) inequalities:
//   sum_{e in E(H)} x_e + sum_i x_{T_i} <= |H| + (s-1)/2,   s = #teeth, odd, >= 3
// which is valid for *every* tour (so it can never cut off the optimum). Handle
// candidates are the connected components of the graph of fractional edges; the
// teeth are the x=1 edges leaving a handle, made pairwise vertex-disjoint with an
// odd count. Returns the coefficient list and rhs of one violated inequality.
fn separate_blossom(x: &[f64], node_count: usize) -> Option<(Vec<(usize, f64)>, f64)> {
    let e = x.len();

    // (1) connected components over fractional edges
    let mut parent: Vec<usize> = (0..=node_count).collect();
    for (k, &xk) in x.iter().enumerate().take(e) {
        if xk > EPS && xk < 1.0 - EPS {
            let (i, j) = index_to_edge(k);
            let ri = uf_find(&mut parent, i);
            let rj = uf_find(&mut parent, j);
            if ri != rj {
                parent[ri] = rj;
            }
        }
    }
    let mut root_of = vec![0usize; node_count + 1];
    let mut members: Vec<Vec<usize>> = vec![Vec::new(); node_count + 1];
    for (v, root) in root_of.iter_mut().enumerate().skip(1).take(node_count) {
        let r = uf_find(&mut parent, v);
        *root = r;
        members[r].push(v);
    }
    let mut has_frac = vec![false; node_count + 1];
    for (k, &xk) in x.iter().enumerate().take(e) {
        if xk > EPS && xk < 1.0 - EPS {
            let (i, _) = index_to_edge(k);
            has_frac[root_of[i]] = true;
        }
    }

    // (2) x=1 boundary edges per fractional component: (col, handle endpoint, outside endpoint)
    let mut boundary: Vec<Vec<(usize, usize, usize)>> = vec![Vec::new(); node_count + 1];
    for (k, &xk) in x.iter().enumerate().take(e) {
        if xk >= 1.0 - EPS {
            let (i, j) = index_to_edge(k);
            let ri = root_of[i];
            let rj = root_of[j];
            if ri != rj {
                if has_frac[ri] {
                    boundary[ri].push((k, i, j));
                }
                if has_frac[rj] {
                    boundary[rj].push((k, j, i));
                }
            }
        }
    }

    // (3) per fractional component, assemble a blossom and test the violation
    for r in 1..=node_count {
        if !has_frac[r] || members[r].len() < 2 {
            continue;
        }
        // vertex-disjoint teeth (greedy)
        let mut used_h = vec![false; node_count + 1];
        let mut used_out = vec![false; node_count + 1];
        let mut teeth: Vec<usize> = Vec::new();
        for &(col, h, out) in &boundary[r] {
            if !used_h[h] && !used_out[out] {
                used_h[h] = true;
                used_out[out] = true;
                teeth.push(col);
            }
        }
        if teeth.len().is_multiple_of(2) {
            teeth.pop(); // force odd count
        }
        let s = teeth.len();
        if s < 3 {
            continue;
        }
        // coefficients: all edges inside H, plus the teeth, each with weight 1
        let hset = &members[r];
        let mut coeffs: Vec<(usize, f64)> = Vec::new();
        let mut lhs = 0.0;
        for a in 0..hset.len() {
            for b in (a + 1)..hset.len() {
                let col = edge_col(hset[a], hset[b]);
                coeffs.push((col, 1.0));
                lhs += x[col];
            }
        }
        for &col in &teeth {
            coeffs.push((col, 1.0));
            lhs += x[col];
        }
        let rhs = hset.len() as f64 + ((s - 1) / 2) as f64;
        if lhs - rhs > CUT_EPS {
            return Some((coeffs, rhs));
        }
    }
    None
}

// Run the subtour-cut separation loop on an already-optimized relaxation.
// Returns Ok(true) when no violated subtour remains (node optimal/feasible),
// Ok(false) when adding a cut renders the node infeasible (prune).
fn cut_loop(
    bv: &mut Bv,
    node_count: usize,
    e: usize,
    do_blossom: bool,
) -> Result<bool, SolverError> {
    let mut blossoms = 0usize;
    let mut rounds = 0usize;
    let cap = CUT_LOOP_FACTOR * node_count + CUT_LOOP_BASE;
    loop {
        // periodic exact refactorization keeps min-cut separation working on
        // accurate edge values instead of drifted ones (which would otherwise
        // report phantom violations and loop forever)
        if rounds > 0 && rounds.is_multiple_of(REFACTOR_INTERVAL) && !bv.refactorize() {
            return Err(SolverError::SimplexError(
                "singular basis during cut separation".into(),
            ));
        }
        rounds += 1;
        if rounds > cap {
            // pathological separation (e.g. residual numerical noise): bail out so
            // the caller rebuilds this node cold from a clean basis
            return Err(SolverError::SimplexError("cut loop iteration cap".into()));
        }
        let x = bv.edge_values(e);
        let w = build_sparse_adj(&x, node_count);
        let (cut_weight, cut_set) = min_cut(&w, node_count);

        if cut_weight < 2.0 - MIN_CUT_VIOLATION {
            // violated subtour: separate exactly (always, every node, no cap)
            let s: HashSet<usize> = cut_set.iter().map(|&i| i + 1).collect(); // 1-based
            let cut = cut_edges_for_set(node_count, &s);
            bv.add_subtour_cut(&cut);
        } else if !do_blossom || blossoms >= MAX_BLOSSOM_CUTS {
            // subtours satisfied and blossom separation disabled here / capped: done
            return Ok(true);
        } else {
            match separate_blossom(&x, node_count) {
                Some((coeffs, rhs)) => {
                    bv.add_le_cut(&coeffs, rhs);
                    blossoms += 1;
                }
                None => return Ok(true), // neither subtour nor blossom violated => optimal
            }
        }

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
    if !cut_loop(
        &mut bv,
        node_count,
        e,
        blossoms_on(node_count, 0, CUT_DEPTH),
    )? {
        return Ok(None);
    }
    Ok(Some(bv))
}

// Warm-start re-solve after a single bound change has been applied to a cloned
// parent tableau. The basis is dual-feasible but primal-infeasible; the dual
// simplex restores feasibility, then the cut loop re-separates subtours.
// Returns Ok(true) if feasible/optimal, Ok(false) if the node is infeasible.
fn resolve_child(
    bv: &mut Bv,
    node_count: usize,
    e: usize,
    do_blossom: bool,
) -> Result<bool, SolverError> {
    // clear inherited numerical drift periodically (deep warm-start chains only)
    bv.since_refac += 1;
    if bv.since_refac >= REFACTOR_DEPTH {
        if !bv.refactorize() {
            return Err(SolverError::SimplexError(
                "singular basis at node entry".into(),
            ));
        }
        bv.since_refac = 0;
    }
    match bv.dual() {
        Ok(()) => {}
        Err(SimplexError::Infeasible) => return Ok(false),
        Err(other) => return Err(other.into()),
    }
    bv.primal()?;
    cut_loop(bv, node_count, e, do_blossom)
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
#[allow(dead_code)]
pub fn try_solve_lp_relaxation(
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

// ---- reliability (pseudocost + strong-branching) branching ----
const RELIABILITY: u32 = 8; // observations per direction before a var is "reliable"
const MAX_STRONG: usize = 6; // max strong-branch probes per node
const LOOKAHEAD: usize = 4; // stop probing after this many non-improving probes
// Strong branching only pays off near the root, where it shapes the tree. With
// ~n^2/2 edge variables but only a few thousand tree nodes, per-edge pseudocosts
// rarely become "reliable", so without this gate we would probe at essentially
// every node -- including the huge proof tail -- and the probing cost dwarfs the
// tree reduction.
//
// Empirically on pr76, *any* strong-branch probing is a net loss: the LP solves a
// probe costs are not repaid, because the instance's difficulty is the proof tail
// (many nodes with lower_bound ~ optimum), which is driven by bound quality, not
// by the branching choice. Pure pseudocost selection (this default, STRONG_DEPTH = 0
// => probing disabled) learns its estimates from the children that are solved
// anyway, adds no extra LP solves, and slightly beats first-fractional. Strong
// branching is retained for instance classes where nodes are cheap relative to the
// tree size: raise STRONG_DEPTH to enable it for the top that many levels (the
// 2^(d+1) bound on shallow nodes keeps total probing bounded regardless of tail
// size). On pr76, STRONG_DEPTH = 8 / MAX_STRONG = 6 was the least-bad probing config.
const STRONG_DEPTH: u32 = 0;
const SB_EPS: f64 = 1e-6; // floor for the product score
const INFEAS_SCORE: f64 = 1e12; // pseudo-degradation assigned to an infeasible child

// Per-edge, per-direction pseudocost statistics (running average of the
// objective degradation per unit of fractionality), plus global averages used
// as a fallback for not-yet-observed variables.
struct Pseudocosts {
    down_sum: Vec<f64>,
    down_cnt: Vec<u32>,
    up_sum: Vec<f64>,
    up_cnt: Vec<u32>,
    tot_down_sum: f64,
    tot_down_cnt: u32,
    tot_up_sum: f64,
    tot_up_cnt: u32,
}

impl Pseudocosts {
    fn new(e: usize) -> Self {
        Pseudocosts {
            down_sum: vec![0.0; e],
            down_cnt: vec![0; e],
            up_sum: vec![0.0; e],
            up_cnt: vec![0; e],
            tot_down_sum: 0.0,
            tot_down_cnt: 0,
            tot_up_sum: 0.0,
            tot_up_cnt: 0,
        }
    }
    fn psi_down(&self, k: usize) -> Option<f64> {
        if self.down_cnt[k] > 0 {
            Some(self.down_sum[k] / self.down_cnt[k] as f64)
        } else {
            None
        }
    }
    fn psi_up(&self, k: usize) -> Option<f64> {
        if self.up_cnt[k] > 0 {
            Some(self.up_sum[k] / self.up_cnt[k] as f64)
        } else {
            None
        }
    }
    fn avg_down(&self) -> f64 {
        if self.tot_down_cnt > 0 {
            self.tot_down_sum / self.tot_down_cnt as f64
        } else {
            1.0
        }
    }
    fn avg_up(&self) -> f64 {
        if self.tot_up_cnt > 0 {
            self.tot_up_sum / self.tot_up_cnt as f64
        } else {
            1.0
        }
    }
    // record a degradation `delta` over fractional amount `frac` in one direction
    fn observe_down(&mut self, k: usize, delta: f64, frac: f64) {
        let z = delta / frac.max(EPS);
        self.down_sum[k] += z;
        self.down_cnt[k] += 1;
        self.tot_down_sum += z;
        self.tot_down_cnt += 1;
    }
    fn observe_up(&mut self, k: usize, delta: f64, frac: f64) {
        let z = delta / frac.max(EPS);
        self.up_sum[k] += z;
        self.up_cnt[k] += 1;
        self.tot_up_sum += z;
        self.tot_up_cnt += 1;
    }
    fn reliable(&self, k: usize) -> bool {
        self.down_cnt[k] >= RELIABILITY && self.up_cnt[k] >= RELIABILITY
    }
}

// Result of solving one child during a strong-branch probe.
enum Probe {
    Feasible(Box<Bv>),
    Infeasible,
}

// Clone the parent, fix edge `k` to 0 or 1, and re-solve (warm dual + cuts).
fn probe_child(
    parent: &Bv,
    k: usize,
    to_one: bool,
    node_count: usize,
    e: usize,
    do_blossom: bool,
) -> Result<Probe, SolverError> {
    let mut c = parent.clone();
    if to_one {
        c.fix_var(k, 1.0);
    } else {
        c.fix_var(k, 0.0);
    }
    if resolve_child(&mut c, node_count, e, do_blossom)? {
        Ok(Probe::Feasible(Box::new(c)))
    } else {
        Ok(Probe::Infeasible)
    }
}

fn probe_lb(p: &Probe, problem: &TsplibInstance, e: usize) -> Result<Option<f64>, SolverError> {
    match p {
        Probe::Feasible(c) => Ok(Some(extract_result(c, problem, e)?.lower_bound)),
        Probe::Infeasible => Ok(None),
    }
}

// The chosen branching edge, plus its already-solved children when they came
// from a strong-branch probe (so the caller need not re-solve them).
struct Choice {
    k: usize,
    f: f64,
    down: Option<Probe>,
    up: Option<Probe>,
}

// Pick a branching edge by reliability branching: pseudocost score for reliable
// edges, real strong-branch score (which also seeds pseudocosts) for unreliable
// ones, capped at MAX_STRONG probes with a lookahead early-stop.
#[allow(clippy::too_many_arguments)]
fn select_branch(
    bv: &Bv,
    problem: &TsplibInstance,
    parent_lb: f64,
    pseudo: &mut Pseudocosts,
    node_count: usize,
    e: usize,
    depth: u32,
    strong_depth: u32,
) -> Result<Choice, SolverError> {
    // probing is enabled only in the top `strong_depth` levels (0 => disabled);
    // comparing two runtime values keeps this lint-clean when strong_depth == 0
    let allow_strong = depth < strong_depth;
    let x = bv.edge_values(e);

    // pre-score all fractional candidates by pseudocost (no outstanding borrow
    // of `pseudo` is kept into the probing loop, which mutates it)
    let adown = pseudo.avg_down();
    let aup = pseudo.avg_up();
    let mut scored: Vec<(usize, f64, f64)> = Vec::new();
    for (k, &xk) in x.iter().enumerate().take(e) {
        let f = xk;
        if f > EPS && f < 1.0 - EPS {
            let pd = pseudo.psi_down(k).unwrap_or(adown);
            let pu = pseudo.psi_up(k).unwrap_or(aup);
            let s = (pd * f).max(SB_EPS) * (pu * (1.0 - f)).max(SB_EPS);
            scored.push((k, f, s));
        }
    }
    scored.sort_by(|a, b| b.2.total_cmp(&a.2));

    let mut best_k = scored[0].0;
    let mut best_f = scored[0].1;
    let mut best_score = scored[0].2;
    let mut best_children: Option<(Probe, Probe)> = None;

    let mut probes = 0usize;
    let mut since_improve = 0usize;
    // Strong branching only near the root; deeper nodes keep the cheap estimate.
    if allow_strong {
        let probe_blossom = blossoms_on(node_count, depth + 1, CUT_DEPTH);
        for &(k, f, _) in &scored {
            if probes >= MAX_STRONG {
                break;
            }
            if pseudo.reliable(k) {
                continue; // estimate is trustworthy; no probe needed
            }
            let dp = probe_child(bv, k, false, node_count, e, probe_blossom)?;
            let up = probe_child(bv, k, true, node_count, e, probe_blossom)?;
            let dlb = probe_lb(&dp, problem, e)?;
            let ulb = probe_lb(&up, problem, e)?;
            if let Some(b) = dlb {
                pseudo.observe_down(k, (b - parent_lb).max(0.0), f);
            }
            if let Some(b) = ulb {
                pseudo.observe_up(k, (b - parent_lb).max(0.0), 1.0 - f);
            }
            let dd = dlb
                .map(|b| (b - parent_lb).max(0.0))
                .unwrap_or(INFEAS_SCORE);
            let du = ulb
                .map(|b| (b - parent_lb).max(0.0))
                .unwrap_or(INFEAS_SCORE);
            let score = dd.max(SB_EPS) * du.max(SB_EPS);
            probes += 1;
            if score > best_score {
                best_score = score;
                best_k = k;
                best_f = f;
                best_children = Some((dp, up));
                since_improve = 0;
            } else {
                since_improve += 1;
                if since_improve >= LOOKAHEAD {
                    break;
                }
            }
        }
    }

    let (down, up) = match best_children {
        Some((d, u)) => (Some(d), Some(u)),
        None => (None, None),
    };
    Ok(Choice {
        k: best_k,
        f: best_f,
        down,
        up,
    })
}

// Primal heuristic: nearest-neighbour construction + 2-opt local search. Yields
// a valid Hamiltonian tour whose cost is an upper bound on the optimum, used to
// seed the incumbent so bound-pruning is effective from the very first node.
// Only sound when no edges are pre-fixed (the tour ignores such constraints).
fn h_rng(seed: &mut u64) -> u64 {
    let mut x = *seed;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *seed = x;
    x
}

fn h_tour_cost(tour: &[usize], n: usize, dmat: &[i64], stride: usize) -> i64 {
    let mut c = 0i64;
    for w in 0..n {
        c += dmat[tour[w] * stride + tour[(w + 1) % n]];
    }
    c
}

// 2-opt + Or-opt local search to a local optimum (first-improvement 2-opt, best
// single Or-opt relocation per pass).
// k nearest neighbours of every city (1-based, 0 = sentinel/unused slot),
// sorted ascending by distance. Flattened: nb[c*k .. c*k+k].
fn h_build_neighbors(n: usize, dmat: &[i64], stride: usize, k: usize) -> (Vec<u32>, usize) {
    let kk = k.min(n - 1).max(1);
    let mut nb = vec![0u32; (n + 1) * kk];
    let mut cand: Vec<(i64, u32)> = Vec::with_capacity(n);
    for c in 1..=n {
        cand.clear();
        for j in 1..=n {
            if j != c {
                cand.push((dmat[c * stride + j], j as u32));
            }
        }
        if cand.len() > kk {
            cand.select_nth_unstable(kk - 1);
            cand.truncate(kk);
        }
        cand.sort_unstable();
        for (idx, &(_, j)) in cand.iter().enumerate() {
            nb[c * kk + idx] = j;
        }
    }
    (nb, kk)
}

// reverse tour[min(a,b)+1 ..= max(a,b)], keeping the position index in sync
fn h_2opt_move(tour: &mut [usize], pos: &mut [usize], a: usize, b: usize) {
    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
    let mut x = lo + 1;
    let mut y = hi;
    while x < y {
        tour.swap(x, y);
        pos[tour[x]] = x;
        pos[tour[y]] = y;
        x += 1;
        y -= 1;
    }
}

// Local search using candidate (nearest-neighbour) lists with don't-look bits:
// 2-opt on both tour edges of each active city plus a neighbour-guided Or-opt
// (segments of length 1..3). A pass is O(n*k) instead of O(n^2), which lets the
// caller afford far more ILS restarts.
fn h_local_search(
    tour: &mut Vec<usize>,
    n: usize,
    dmat: &[i64],
    stride: usize,
    nb: &[u32],
    k: usize,
) {
    let d = |a: usize, b: usize| -> i64 { dmat[a * stride + b] };
    let mut pos = vec![0usize; n + 1];
    for (p, &c) in tour.iter().enumerate() {
        pos[c] = p;
    }
    let mut dlb = vec![false; n + 1];
    let mut queue: Vec<usize> = tour.clone();
    let mut qh = 0usize;
    let mut guard = 0u64;
    let cap = 200u64 * n as u64 + 1000;

    while qh < queue.len() {
        guard += 1;
        if guard > cap {
            break; // safety net: never spin even if a future move oscillates
        }
        let c1 = queue[qh];
        qh += 1;
        if dlb[c1] {
            continue;
        }
        let mut found = false;

        // ---- 2-opt around c1, breaking either of its two tour edges ----
        let i = pos[c1];
        let succ = tour[(i + 1) % n];
        let pred = tour[(i + n - 1) % n];
        let d_succ = d(c1, succ);
        let d_pred = d(pred, c1);
        let dmax = d_succ.max(d_pred);
        for idx in 0..k {
            let c2u = nb[c1 * k + idx];
            if c2u == 0 {
                break;
            }
            let c2 = c2u as usize;
            if c2 == c1 {
                continue;
            }
            let dc = d(c1, c2);
            if dc >= dmax {
                break; // neighbours sorted ascending: no further gain possible
            }
            let j = pos[c2];
            if dc < d_succ && c2 != succ {
                let c2s = tour[(j + 1) % n];
                if c2s != c1 && d_succ + d(c2, c2s) - dc - d(succ, c2s) > 0 {
                    h_2opt_move(&mut tour[..], &mut pos[..], i, j);
                    for c in [c1, succ, c2, c2s] {
                        dlb[c] = false;
                        queue.push(c);
                    }
                    found = true;
                    break;
                }
            }
            if dc < d_pred && c2 != pred {
                let c2p = tour[(j + n - 1) % n];
                if c2p != c1 && d_pred + d(c2p, c2) - dc - d(pred, c2p) > 0 {
                    h_2opt_move(
                        &mut tour[..],
                        &mut pos[..],
                        (i + n - 1) % n,
                        (j + n - 1) % n,
                    );
                    for c in [c1, pred, c2, c2p] {
                        dlb[c] = false;
                        queue.push(c);
                    }
                    found = true;
                    break;
                }
            }
        }

        // ---- Or-opt: relocate a short segment starting at c1 next to a neighbour ----
        if !found {
            'or: for seglen in 1..=3usize {
                if n < seglen + 2 {
                    break;
                }
                let i = pos[c1];
                if i + seglen > n {
                    break; // skip wrap-spanning segment (covered from other cities)
                }
                let s0 = c1;
                let s1 = tour[i + seglen - 1];
                let prev = tour[(i + n - 1) % n];
                let nxt = tour[(i + seglen) % n];
                if prev == s1 || nxt == s0 {
                    continue;
                }
                let remove_gain = d(prev, s0) + d(s1, nxt) - d(prev, nxt);
                if remove_gain <= 0 {
                    continue;
                }
                for idx in 0..k {
                    let au = nb[s0 * k + idx];
                    if au == 0 {
                        break;
                    }
                    let a = au as usize;
                    let pj = pos[a];
                    if pj >= i && pj < i + seglen {
                        continue;
                    }
                    let pj1 = (pj + 1) % n;
                    if pj1 >= i && pj1 < i + seglen {
                        continue;
                    }
                    let b = tour[pj1];
                    if b == s0 {
                        continue;
                    }
                    let add = d(a, s0) + d(s1, b) - d(a, b);
                    if remove_gain - add > 0 {
                        let seg: Vec<usize> = tour[i..i + seglen].to_vec();
                        let mut rest: Vec<usize> = Vec::with_capacity(n - seglen);
                        for (p, &c) in tour.iter().enumerate() {
                            if !(i..i + seglen).contains(&p) {
                                rest.push(c);
                            }
                        }
                        let posa = rest.iter().position(|&c| c == a).unwrap();
                        let mut nt = Vec::with_capacity(n);
                        nt.extend_from_slice(&rest[..=posa]);
                        nt.extend_from_slice(&seg);
                        nt.extend_from_slice(&rest[posa + 1..]);
                        *tour = nt;
                        for (p, &c) in tour.iter().enumerate() {
                            pos[c] = p;
                        }
                        for c in [prev, s0, s1, nxt, a, b] {
                            dlb[c] = false;
                            queue.push(c);
                        }
                        found = true;
                        break 'or;
                    }
                }
            }
        }

        if !found {
            dlb[c1] = true;
        }
        if qh > 4 * n && qh * 2 > queue.len() {
            queue.drain(0..qh);
            qh = 0;
        }
    }
}

// double-bridge 4-opt perturbation: split into A|B|C|D, reconnect as A|C|B|D.
// This kick cannot be undone by a single 2-opt move, so it escapes 2-opt optima.
fn h_double_bridge(tour: &[usize], n: usize, seed: &mut u64) -> Vec<usize> {
    if n < 8 {
        return tour.to_vec();
    }
    let mut p = [
        (h_rng(seed) as usize) % (n - 1) + 1,
        (h_rng(seed) as usize) % (n - 1) + 1,
        (h_rng(seed) as usize) % (n - 1) + 1,
    ];
    p.sort_unstable();
    let (p1, p2, p3) = (p[0], p[1], p[2]);
    if p1 == p2 || p2 == p3 {
        return tour.to_vec();
    }
    let mut nt = Vec::with_capacity(n);
    nt.extend_from_slice(&tour[0..p1]);
    nt.extend_from_slice(&tour[p2..p3]);
    nt.extend_from_slice(&tour[p1..p2]);
    nt.extend_from_slice(&tour[p3..n]);
    nt
}

// Primal heuristic: nearest-neighbour construction, 2-opt + Or-opt local search,
// then iterated local search with double-bridge kicks. Yields a valid tour whose
// cost is an upper bound on the optimum, used to seed the incumbent so that
// bound-pruning is effective from the very first node. Only sound when no edges
// are pre-fixed (the tour ignores such constraints).
type PrimalHeuristicResult = Result<Option<(f64, Vec<(usize, usize)>)>, SolverError>;
fn primal_heuristic(problem: &TsplibInstance) -> PrimalHeuristicResult {
    let n = problem.nodes.len();
    if n < 3 {
        return Ok(None);
    }
    let stride = n + 1;
    let mut dmat = vec![0i64; stride * stride];
    for i in 1..=n {
        for j in 1..=n {
            if i != j {
                dmat[i * stride + j] = problem.try_get_distance(i, j)? as i64;
            }
        }
    }

    // nearest-neighbour construction from node 1
    let mut visited = vec![false; n + 1];
    let mut tour = Vec::with_capacity(n);
    let mut cur = 1usize;
    visited[1] = true;
    tour.push(1usize);
    for _ in 1..n {
        let mut best_j = 0usize;
        let mut bd = i64::MAX;
        for j in 1..=n {
            if !visited[j] && dmat[cur * stride + j] < bd {
                bd = dmat[cur * stride + j];
                best_j = j;
            }
        }
        visited[best_j] = true;
        tour.push(best_j);
        cur = best_j;
    }

    let (nb, k) = h_build_neighbors(n, &dmat, stride, 10);
    h_local_search(&mut tour, n, &dmat, stride, &nb, k);
    let mut best_tour = tour.clone();
    let mut best_cost = h_tour_cost(&tour, n, &dmat, stride);

    // iterated local search with double-bridge kicks. Candidate-list local search
    // is ~100-500x cheaper per restart than the old full O(n^2) sweep, so the
    // budget is large; this runs once at the root, so a second or two is cheap.
    let iters = if n < 12 {
        0
    } else if n <= 80 {
        2000
    } else if n <= 150 {
        5000
    } else if n <= 600 {
        15000
    } else {
        10000
    };
    let mut seed = 0x9E3779B97F4A7C15u64 ^ (n as u64).wrapping_mul(0x2545F4914F6CDD1D);
    let mut cur_tour = best_tour.clone();
    let mut cur_cost = best_cost;
    for _ in 0..iters {
        let mut trial = h_double_bridge(&cur_tour, n, &mut seed);
        h_local_search(&mut trial, n, &dmat, stride, &nb, k);
        let tc = h_tour_cost(&trial, n, &dmat, stride);
        if tc < cur_cost {
            cur_tour = trial;
            cur_cost = tc;
            if tc < best_cost {
                best_cost = tc;
                best_tour = cur_tour.clone();
            }
        }
    }

    let mut edges = Vec::with_capacity(n);
    for w in 0..n {
        let a = best_tour[w];
        let b = best_tour[(w + 1) % n];
        edges.push(if a > b { (a, b) } else { (b, a) });
    }
    Ok(Some((best_cost as f64, edges)))
}

fn branch_and_bound(
    problem: &TsplibInstance,
    initial_fixed: &HashMap<(usize, usize), bool>,
    ctx: ExecutionContext,
) -> Result<Option<TspSolution>, SolverError> {
    let node_count = problem.nodes.len();
    let e = node_count * (node_count - 1) / 2;
    // seed the incumbent with a heuristic tour so pruning works from node 0
    // (only sound when nothing is pre-fixed, else the tour may violate it)
    let mut best: Option<(f64, Vec<(usize, usize)>)> = if initial_fixed.is_empty() {
        primal_heuristic(problem)?
    } else {
        None
    };
    let mut pseudo = Pseudocosts::new(e);

    // Root is solved cold; every stack entry is an already-optimized tableau
    // (its relaxation solved, no violated subtour). Children inherit the parent
    // tableau and warm-start via the dual simplex after a single bound change.
    // The f64 carried with each entry is that node's LP lower bound; the minimum
    // over all open nodes is a valid global lower bound on the optimum.
    let root = match solve_root(problem, initial_fixed)? {
        Some(bv) => bv,
        None => return Ok(None),
    };
    let root_lb = extract_result(&root, problem, e)?.lower_bound;
    let mut stack: Vec<(Bv, u32, f64)> = vec![(root, 0, root_lb)];
    let mut processed: u64 = 0;

    while let Some((bv, depth, node_lb)) = stack.pop() {
        processed += 1;

        // cooperative cancellation via the execution context: the only early exit
        if processed.is_multiple_of(64) && ctx.is_cancelled() {
            return Err(SolverError::Cancelled);
        }
        // periodic, informational progress log: incumbent, global lower bound
        // (the minimum LP bound over this node and all open nodes is a valid lower
        // bound on the optimum) and the resulting gap. The search always continues
        // to a proven optimum.
        if processed.is_multiple_of(PROGRESS_INTERVAL) {
            let mut global_lb = node_lb;
            for (_, _, lb) in &stack {
                if *lb < global_lb {
                    global_lb = *lb;
                }
            }
            if let Some((inc, _)) = &best {
                let gap = (*inc - global_lb) / (*inc).abs().max(1.0);
                tracing::info!(
                    incumbent = *inc,
                    global_lb,
                    gap_pct = gap * 100.0,
                    open = stack.len(),
                    nodes = processed,
                    "search progress"
                );
            }
        }

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

            Some(_) => {
                let choice = select_branch(
                    &bv,
                    problem,
                    result.lower_bound,
                    &mut pseudo,
                    node_count,
                    e,
                    depth,
                    STRONG_DEPTH,
                )?;
                let k = choice.k;
                let f = choice.f;
                tracing::debug!(edge = ?index_to_edge(k), depth, "Branching on fractional edge");

                // down child: fix edge to 0 (l = u = 0)
                match choice.down {
                    // already solved during the strong-branch probe (pseudocost
                    // for this direction was recorded there)
                    Some(Probe::Feasible(d)) => {
                        let lb = extract_result(&d, problem, e)?.lower_bound;
                        stack.push((*d, depth + 1, lb));
                    }
                    Some(Probe::Infeasible) => {}
                    // chosen edge was not probed (reliable / probe cap / deep): solve now
                    None => {
                        let mut d = bv.clone();
                        d.fix_var(k, 0.0);
                        let warm = if bv.m > node_count.saturating_mul(CUT_AGING_FACTOR) {
                            Err(SolverError::SimplexError("cut aging: rebuild lean".into()))
                        } else {
                            resolve_child(
                                &mut d,
                                node_count,
                                e,
                                blossoms_on(node_count, depth + 1, CUT_DEPTH),
                            )
                        };
                        match warm {
                            Ok(true) => {
                                let lb = extract_result(&d, problem, e)?.lower_bound;
                                pseudo.observe_down(k, (lb - result.lower_bound).max(0.0), f);
                                stack.push((d, depth + 1, lb));
                            }
                            Ok(false) => {}
                            Err(_) => {
                                // warm-start failed (numerical drift) or too many
                                // inherited cuts: rebuild cold, dropping stale cuts
                                let fixed = recover_fixed_edges(&d, e);
                                if let Some(b) = solve_root(problem, &fixed)? {
                                    let lb = extract_result(&b, problem, e)?.lower_bound;
                                    stack.push((b, depth + 1, lb));
                                }
                            }
                        }
                    }
                }

                // up child: fix edge to 1 (l = u = 1)
                match choice.up {
                    Some(Probe::Feasible(u)) => {
                        let lb = extract_result(&u, problem, e)?.lower_bound;
                        stack.push((*u, depth + 1, lb));
                    }
                    Some(Probe::Infeasible) => {}
                    None => {
                        let mut u = bv.clone();
                        u.fix_var(k, 1.0);
                        let warm = if bv.m > node_count.saturating_mul(CUT_AGING_FACTOR) {
                            Err(SolverError::SimplexError("cut aging: rebuild lean".into()))
                        } else {
                            resolve_child(
                                &mut u,
                                node_count,
                                e,
                                blossoms_on(node_count, depth + 1, CUT_DEPTH),
                            )
                        };
                        match warm {
                            Ok(true) => {
                                let lb = extract_result(&u, problem, e)?.lower_bound;
                                pseudo.observe_up(k, (lb - result.lower_bound).max(0.0), 1.0 - f);
                                stack.push((u, depth + 1, lb));
                            }
                            Ok(false) => {}
                            Err(_) => {
                                let fixed = recover_fixed_edges(&u, e);
                                if let Some(b) = solve_root(problem, &fixed)? {
                                    let lb = extract_result(&b, problem, e)?.lower_bound;
                                    stack.push((b, depth + 1, lb));
                                }
                            }
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
