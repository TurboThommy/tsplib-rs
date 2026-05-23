use tsplib_core::models::TsplibInstance;

use crate::{PerfectMatchingAlgorithm, TspSolver, errors::SolverError, matcher::GreedyMatching};

pub struct Christofides {}

impl Christofides {
    pub fn new() -> Self {
        Christofides {}
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
            .filter(|(_, degree)| *degree % 2 == 1)
            .map(|(&node_id, _)| node_id)
            .collect::<Vec<_>>();

        // compute a perfect matching on the odd degree vertices
        // TODO: make the perfect matching algorithm configurable (e.g. via ctx)
        let matcher = GreedyMatching::new();
        let _matching = matcher.try_compute(&odd_vertices, problem)?;

        todo!()
    }
}
