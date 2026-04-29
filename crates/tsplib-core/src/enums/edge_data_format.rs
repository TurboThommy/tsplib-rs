use std::fmt;

/// EdgeDataFormat specifies how the edge data is formatted in the problem instance.
#[derive(Debug)]
pub enum EdgeDataFormat {
    // EDGE_LIST, The graph is given by an edge list
    EdgeList,

    // ADJ_LIST, The graph is given as an adjacency list
    AdjList,
}

impl fmt::Display for EdgeDataFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            EdgeDataFormat::EdgeList => "EDGE_LIST",
            EdgeDataFormat::AdjList => "ADJ_LIST",
        };
        write!(f, "{}", s)
    }
}
