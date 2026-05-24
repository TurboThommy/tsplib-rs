//! Extern C function declarations for the Blossom V library,
//! which provides an interface to solve the minimum weight perfect matching problem using the Blossom V algorithm.
unsafe extern "C" {
    /// Solves the minimum weight perfect matching problem using the Blossom V algorithm.
    pub fn blossom_v_solve(
        node_count: i32,
        edge_count: i32,
        from: *const i32,
        to: *const i32,
        weight: *const i32,
        out_mate: *mut i32,
    ) -> i32;
}
