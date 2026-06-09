#include "PerfectMatching.h"

extern "C" int blossom_v_solve(
    int node_count,
    int edge_count,
    const int *from,
    const int *to,
    const int *weight,
    int *out_mate)
{
    // Validate input parameters
    if (node_count <= 0 || edge_count < 0 || !from || !to || !weight || !out_mate)
    {
        // Return 1 to indicate invalid input parameters
        return 1;
    }

    // Create a PerfectMatching instance
    PerfectMatching pm(node_count, edge_count);

    // Disable verbose output
    pm.options.verbose = false;

    // Add edges to the PerfectMatching instance
    for (int i = 0; i < edge_count; ++i)
    {
        pm.AddEdge(from[i], to[i], weight[i]);
    }

    // Solve the perfect matching problem
    pm.Solve();

    // Retrieve the matching results
    for (int i = 0; i < node_count; ++i)
    {
        out_mate[i] = pm.GetMatch(i);
    }

    // Return 0 to indicate success
    return 0;
}