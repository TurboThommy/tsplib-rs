use tsplib_core::models::{Edge, Node};

use super::*;

fn test_graph(adjacency: Vec<Vec<usize>>) -> EdmondsGraph {
    let nodes = (0..adjacency.len())
        .map(|id| Node {
            id,
            x: 0.0,
            y: 0.0,
            z: None,
        })
        .collect::<Vec<_>>();

    let mut edges = Vec::new();

    for u in 0..adjacency.len() {
        for &v in &adjacency[u] {
            if u < v {
                edges.push(Edge { u, v, weight: 1 });
            }
        }
    }

    EdmondsGraph::from_graph(&Graph { nodes, edges })
}

#[test]
fn augment_path_toggles_matching_edges() {
    let mut state = MatchingState::new(6);

    state.match_edge(1, 2);
    state.match_edge(3, 4);

    state.augment_path(&[0, 1, 2, 3, 4, 5]);

    assert_eq!(state.mate[0], Some(1));
    assert_eq!(state.mate[1], Some(0));

    assert_eq!(state.mate[2], Some(3));
    assert_eq!(state.mate[3], Some(2));

    assert_eq!(state.mate[4], Some(5));
    assert_eq!(state.mate[5], Some(4));
}

#[test]
fn find_simple_augmenting_path_without_blossom() {
    let graph = test_graph(vec![
        vec![1],    // 0
        vec![0, 2], // 1
        vec![1, 3], // 2
        vec![2, 4], // 3
        vec![3, 5], // 4
        vec![4],    // 5
    ]);

    let mut matching = MatchingState::new(6);
    matching.match_edge(1, 2);
    matching.match_edge(3, 4);

    let path = search_alternating_tree(&graph, &matching, 0).expect("search should succeed");

    match path {
        SearchResult::AugmentingPath(vertices) => {
            assert_eq!(vertices, vec![0, 1, 2, 3, 4, 5]);

            matching.augment_path(&vertices);
        }
        _ => panic!("expected blossom"),
    }

    assert_eq!(matching.mate[0], Some(1));
    assert_eq!(matching.mate[1], Some(0));
    assert_eq!(matching.mate[2], Some(3));
    assert_eq!(matching.mate[3], Some(2));
    assert_eq!(matching.mate[4], Some(5));
    assert_eq!(matching.mate[5], Some(4));
}

#[test]
fn find_lca_in_alternating_tree() {
    let parent = vec![
        None,    // 0 (root)
        Some(0), // 1
        Some(1), // 2
        Some(0), // 3
        Some(3), // 4
    ];

    let lca = find_lca(2, 4, &parent);

    assert_eq!(lca, Some(0));
}

#[test]
fn reconstruct_blossom_cycle() {
    let parent = vec![
        None,    // 0 root/lca
        Some(0), // 1
        Some(1), // 2 = u
        Some(0), // 3
        Some(3), // 4 = v
    ];

    let cycle =
        try_reconstruct_blossom_cycle(2, 4, 0, &parent).expect("should reconstruct blossom cycle");

    assert_eq!(cycle, vec![2, 1, 0, 3, 4]);
}

#[test]
fn detect_simple_blossom_cycles() {
    let graph = test_graph(vec![
        vec![1, 3], // 0 root
        vec![0, 2], // 1
        vec![1, 4], // 2
        vec![0, 4], // 3
        vec![2, 3], // 4
    ]);

    let mut matching = MatchingState::new(5);
    matching.match_edge(1, 2);
    matching.match_edge(3, 4);

    let result = search_alternating_tree(&graph, &matching, 0).expect("search should succeed");

    match result {
        SearchResult::Blossom { cycle, base, edge } => {
            assert_eq!(cycle.len(), 5);
            assert_eq!(base, 0);
            assert!(matches!(edge, (2, 4) | (4, 2)));
            assert!(cycle.contains(&0));
            assert!(cycle.contains(&1));
            assert!(cycle.contains(&2));
            assert!(cycle.contains(&3));
            assert!(cycle.contains(&4));
        }
        _ => panic!("expected blossom"),
    }
}

#[test]
fn shrink_graph_contracts_blossom_cycle() {
    let graph = test_graph(vec![
        vec![1, 3, 5], // 0
        vec![0, 2],    // 1
        vec![1, 4],    // 2
        vec![0, 4],    // 3
        vec![2, 3, 6], // 4
        vec![0],       // 5 external
        vec![4],       // 6 external
    ]);

    let blossom = Blossom::new(0, vec![2, 1, 0, 3, 4]);
    let shrunk = shrink_graph(&graph, &blossom);

    assert_eq!(shrunk.graph.adjacency.len(), 3);
    assert_eq!(shrunk.blossom_node, 2);

    assert_eq!(shrunk.original_to_shrunk[5], 0);
    assert_eq!(shrunk.original_to_shrunk[6], 1);

    for node in 0..=4 {
        assert_eq!(shrunk.original_to_shrunk[node], shrunk.blossom_node);
    }

    assert!(shrunk.graph.adjacency[shrunk.blossom_node].contains(&0));
    assert!(shrunk.graph.adjacency[shrunk.blossom_node].contains(&1));
    assert!(shrunk.graph.adjacency[0].contains(&shrunk.blossom_node));
    assert!(shrunk.graph.adjacency[1].contains(&shrunk.blossom_node));
}

#[test]
fn shrink_matching_maps_external_matching_edge_to_blossom_node() {
    let graph = test_graph(vec![
        vec![1, 3, 5], // 0
        vec![0, 2],    // 1
        vec![1, 4],    // 2
        vec![0, 4],    // 3
        vec![2, 3, 6], // 4
        vec![0],       // 5 external
        vec![4],       // 6 external
    ]);

    let blossom = Blossom::new(0, vec![2, 1, 0, 3, 4]);
    let shrunk = shrink_graph(&graph, &blossom);

    let mut matching = MatchingState::new(7);
    matching.match_edge(1, 2); // inside blossom
    matching.match_edge(3, 4); // inside blossom
    matching.match_edge(0, 5); // blossom matched to outside

    let shrunk_matching = shrink_matching(&matching, &shrunk);

    let external_5 = shrunk.original_to_shrunk[5];

    assert_eq!(shrunk_matching.mate[shrunk.blossom_node], Some(external_5));
    assert_eq!(shrunk_matching.mate[external_5], Some(shrunk.blossom_node));
}

#[test]
fn can_shrink_detected_blossom() {
    let graph = test_graph(vec![
        vec![1, 3], // 0 root / blossom base
        vec![0, 2], // 1
        vec![1, 4], // 2
        vec![0, 4], // 3
        vec![2, 3], // 4
    ]);

    let mut matching = MatchingState::new(5);
    matching.match_edge(1, 2);
    matching.match_edge(3, 4);

    let result = search_alternating_tree(&graph, &matching, 0).expect("search should succeed");

    let SearchResult::Blossom { cycle, base, .. } = result else {
        panic!("expected blossom");
    };

    let blossom = Blossom::new(base, cycle);
    let shrunk = shrink_graph(&graph, &blossom);
    let shrunk_matching = shrink_matching(&matching, &shrunk);

    assert_eq!(blossom.base, 0);
    assert_eq!(blossom.cycle.len(), 5);

    assert_eq!(shrunk.graph.adjacency.len(), 1);
    assert_eq!(shrunk.blossom_node, 0);

    for node in 0..5 {
        assert_eq!(shrunk.original_to_shrunk[node], shrunk.blossom_node);
    }

    assert!(shrunk.graph.adjacency[shrunk.blossom_node].is_empty());
    assert!(shrunk_matching.mate[shrunk.blossom_node].is_none());
}

#[test]
fn edmonds_finds_augmenting_path_without_blossom() {
    let graph = test_graph(vec![
        vec![1],    // 0
        vec![0, 2], // 1
        vec![1, 3], // 2
        vec![2, 4], // 3
        vec![3, 5], // 4
        vec![4],    // 5
    ]);

    let mut matching = MatchingState::new(6);
    matching.match_edge(1, 2);
    matching.match_edge(3, 4);

    let path = try_find_augmenting_path_edmonds(&graph, &matching, 0)
        .expect("search should succeed")
        .expect("augmenting path should exist");

    assert_eq!(path, vec![0, 1, 2, 3, 4, 5]);
}

#[test]
fn edmonds_detects_blossom() {
    let graph = test_graph(vec![
        vec![1, 3, 5], // 0 base
        vec![0, 2],    // 1
        vec![1, 4],    // 2
        vec![0, 4],    // 3
        vec![2, 3],    // 4
        vec![0, 6],    // 5
        vec![5],       // 6 exposed
    ]);

    let mut matching = MatchingState::new(7);
    matching.match_edge(1, 2);
    matching.match_edge(3, 4);
    matching.match_edge(0, 5);

    let result = search_alternating_tree(&graph, &matching, 6).expect("search should succeed");

    let SearchResult::Blossom { cycle, base, .. } = result else {
        panic!("expected blossom before shrinking");
    };

    let blossom = Blossom::new(base, cycle);
    let shrunk = shrink_graph(&graph, &blossom);
    let shrunk_matching = shrink_matching(&matching, &shrunk);
    let shrunk_root = shrunk.original_to_shrunk[6];

    let shrunk_result = search_alternating_tree(&shrunk.graph, &shrunk_matching, shrunk_root)
        .expect("shrunk search should succeed");

    assert!(matches!(shrunk_result, SearchResult::None));
}

#[test]
fn blossom_cycle_index() {
    let blossom = Blossom::new(0, vec![2, 1, 0, 3, 4]);

    assert_eq!(blossom.cycle_index(2), Some(0));
    assert_eq!(blossom.cycle_index(0), Some(2));
    assert_eq!(blossom.cycle_index(4), Some(4));
    assert_eq!(blossom.cycle_index(42), None);
}

#[test]
fn blossom_cycle_paths_between() {
    let blossom = Blossom::new(0, vec![2, 1, 0, 3, 4]);

    let (forward, backward) = blossom
        .cycle_paths_between(2, 4)
        .expect("path should exist");

    assert_eq!(forward, vec![2, 1, 0, 3, 4]);
    assert_eq!(backward, vec![2, 4]);
}

#[test]
fn detects_alternating_path() {
    let mut matching = MatchingState::new(5);
    matching.match_edge(1, 2);
    matching.match_edge(3, 4);

    assert!(is_alternating_path(&[0, 1, 2, 3, 4], &matching));
    assert!(!is_alternating_path(&[0, 1, 3, 4], &matching));
}

#[test]
fn chooses_alternating_blossom_path() {
    let blossom = Blossom::new(0, vec![2, 1, 0, 3, 4]);

    let mut matching = MatchingState::new(5);
    matching.match_edge(1, 2);
    matching.match_edge(3, 4);

    let path = try_choose_alternating_blossom_path(&blossom, 0, 4, &matching)
        .expect("should choose alternating blossom path");

    assert_eq!(path, vec![0, 3, 4]);
}

#[test]
fn expand_blossom_between_entry_and_exit() {
    let blossom = Blossom::new(0, vec![2, 1, 0, 3, 4]);

    let mut matching = MatchingState::new(5);
    matching.match_edge(1, 2);
    matching.match_edge(3, 4);

    let expanded =
        try_expand_blossom_node(&blossom, 0, 4, &matching).expect("should expand blossom");

    assert_eq!(expanded, vec![0, 3, 4]);
}

#[test]
fn expands_path_through_blossom() {
    let graph = test_graph(vec![
        vec![1, 3, 5], // 0
        vec![0, 2],    // 1
        vec![1, 4],    // 2
        vec![0, 4, 6], // 3
        vec![2, 3],    // 4
        vec![0],       // 5 external
        vec![3],       // 6 external
    ]);

    let blossom = Blossom::new(0, vec![2, 1, 0, 3, 4]);
    let shrunk = shrink_graph(&graph, &blossom);

    let mut matching = MatchingState::new(7);
    matching.match_edge(1, 2);
    matching.match_edge(3, 4);

    let path = vec![
        shrunk.original_to_shrunk[5],
        shrunk.blossom_node,
        shrunk.original_to_shrunk[6],
    ];

    let expanded = try_expand_path_through_blossom(&path, &graph, &shrunk, &blossom, &matching)
        .expect("path should expand");

    assert_eq!(expanded, vec![5, 0, 3, 6]);
}

#[test]
fn edmonds_expands_augmenting_path_through_blossom() {
    let graph = test_graph(vec![
        vec![1, 3, 5], // 0 base
        vec![0, 2],    // 1
        vec![1, 4],    // 2
        vec![0, 4, 7], // 3
        vec![2, 3],    // 4
        vec![0, 6],    // 5
        vec![5],       // 6 exposed root
        vec![3],       // 7 exposed target
    ]);

    let mut matching = MatchingState::new(8);
    matching.match_edge(1, 2);
    matching.match_edge(3, 4);
    matching.match_edge(0, 5);

    let path = try_find_augmenting_path_edmonds(&graph, &matching, 6)
        .expect("search should succeed")
        .expect("augmenting path should exist");

    assert_eq!(path, vec![6, 5, 0, 3, 7]);
}

#[test]
fn computes_maximum_matching_without_blossom() {
    let graph = test_graph(vec![vec![1], vec![0, 2], vec![1, 3], vec![2]]);

    let matching = try_compute_maximum_matching(&graph).expect("matching should compute");

    assert_eq!(matching.mate[0], Some(1));
    assert_eq!(matching.mate[1], Some(0));
    assert_eq!(matching.mate[2], Some(3));
    assert_eq!(matching.mate[3], Some(2));
}

#[test]
fn computes_maximum_matching_with_blossom() {
    let graph = test_graph(vec![
        vec![1, 2],    // 0
        vec![0, 2],    // 1
        vec![0, 1, 3], // 2
        vec![2],       // 3
    ]);

    let matching = try_compute_maximum_matching(&graph).expect("matching should compute");

    let matched_edges = matching
        .mate
        .iter()
        .enumerate()
        .filter(|(u, mate)| mate.is_some_and(|v| *u < v))
        .count();

    assert_eq!(matched_edges, 2);
}

#[test]
fn validates_matching_state() {
    let mut matching = MatchingState::new(4);

    matching.match_edge(0, 1);
    matching.match_edge(2, 3);

    assert!(matching.is_valid());
}

#[test]
fn matching_cardinality() {
    let mut matching = MatchingState::new(6);

    matching.match_edge(0, 1);
    matching.match_edge(2, 3);
    matching.match_edge(4, 5);

    assert_eq!(matching.cardinality(), 3);
}

#[test]
fn computes_maximum_matching_on_cycle() {
    let graph = test_graph(vec![vec![1, 2], vec![0, 3], vec![0, 3], vec![1, 2]]);

    let matching = try_compute_maximum_matching(&graph).expect("matching should compute");

    assert!(matching.is_valid());
    assert_eq!(matching.cardinality(), 2);
}

#[test]
fn computes_maximum_matching_on_odd_cycle() {
    let graph = test_graph(vec![
        vec![1, 4],
        vec![0, 2],
        vec![1, 3],
        vec![2, 4],
        vec![3, 0],
    ]);

    let matching = try_compute_maximum_matching(&graph).expect("matching should compute");

    assert!(matching.is_valid());
    assert_eq!(matching.cardinality(), 2);
}

#[test]
fn shrink_graph_preserves_minimum_external_edge_weight() {
    let nodes = (0..6)
        .map(|id| Node {
            id,
            x: 0.0,
            y: 0.0,
            z: None,
        })
        .collect::<Vec<_>>();

    let edges = vec![
        // blossom cycle
        Edge {
            u: 0,
            v: 1,
            weight: 1,
        },
        Edge {
            u: 1,
            v: 2,
            weight: 1,
        },
        Edge {
            u: 2,
            v: 4,
            weight: 1,
        },
        Edge {
            u: 4,
            v: 3,
            weight: 1,
        },
        Edge {
            u: 3,
            v: 0,
            weight: 1,
        },
        // external edges to node 5
        Edge {
            u: 0,
            v: 5,
            weight: 30,
        },
        Edge {
            u: 1,
            v: 5,
            weight: 10,
        },
        Edge {
            u: 2,
            v: 5,
            weight: 20,
        },
    ];

    let graph = EdmondsGraph::from_graph(&Graph { nodes, edges });

    let blossom = Blossom::new(0, vec![2, 1, 0, 3, 4]);
    let shrunk = shrink_graph(&graph, &blossom);

    let external_5 = shrunk.original_to_shrunk[5];

    assert_eq!(
        shrunk.graph.weight(shrunk.blossom_node, external_5),
        Some(10)
    );
}

#[test]
fn dual_state_can_store_values() {
    let mut duals = DualState::new(4);

    duals.try_set(2, 17).unwrap();

    assert_eq!(duals.get(2), Some(17));
}

#[test]
fn dual_state_can_increment_values() {
    let mut duals = DualState::new(4);

    duals.try_set(2, 17).unwrap();
    duals.try_add(2, 5).unwrap();

    assert_eq!(duals.get(2), Some(22));
}

#[test]
fn slack_is_weight_minus_duals() {
    let graph = EdmondsGraph::from_graph(&Graph {
        nodes: vec![
            Node {
                id: 0,
                x: 0.0,
                y: 0.0,
                z: None,
            },
            Node {
                id: 1,
                x: 0.0,
                y: 0.0,
                z: None,
            },
        ],
        edges: vec![Edge {
            u: 0,
            v: 1,
            weight: 10,
        }],
    });

    let mut duals = DualState::new(2);

    duals.try_set(0, 6).unwrap();
    duals.try_set(1, 4).unwrap();

    assert_eq!(duals.try_slack(&graph, 0, 1).unwrap(), 10);
}

#[test]
fn tight_edge_has_zero_slack() {
    let graph = EdmondsGraph::from_graph(&Graph {
        nodes: vec![
            Node {
                id: 0,
                x: 0.0,
                y: 0.0,
                z: None,
            },
            Node {
                id: 1,
                x: 0.0,
                y: 0.0,
                z: None,
            },
        ],
        edges: vec![Edge {
            u: 0,
            v: 1,
            weight: 10,
        }],
    });

    let mut duals = DualState::new(2);

    duals.try_set(0, 8).unwrap();
    duals.try_set(1, 12).unwrap();

    assert_eq!(duals.try_slack(&graph, 0, 1).unwrap(), 0);
}

#[test]
fn detects_tight_edge() {
    let graph = EdmondsGraph::from_graph(&Graph {
        nodes: vec![
            Node {
                id: 0,
                x: 0.0,
                y: 0.0,
                z: None,
            },
            Node {
                id: 1,
                x: 0.0,
                y: 0.0,
                z: None,
            },
        ],
        edges: vec![Edge {
            u: 0,
            v: 1,
            weight: 10,
        }],
    });

    let mut duals = DualState::new(2);

    duals.try_set(0, 8).unwrap();
    duals.try_set(1, 12).unwrap();

    assert!(graph.try_is_tight(&duals, 0, 1).unwrap());
}

#[test]
fn detects_non_tight_edge() {
    let graph = EdmondsGraph::from_graph(&Graph {
        nodes: vec![
            Node {
                id: 0,
                x: 0.0,
                y: 0.0,
                z: None,
            },
            Node {
                id: 1,
                x: 0.0,
                y: 0.0,
                z: None,
            },
        ],
        edges: vec![Edge {
            u: 0,
            v: 1,
            weight: 10,
        }],
    });

    let mut duals = DualState::new(2);

    duals.try_set(0, 6).unwrap();
    duals.try_set(1, 4).unwrap();

    assert!(!graph.try_is_tight(&duals, 0, 1).unwrap());
}

#[test]
fn returns_only_tight_neighbors() {
    let graph = EdmondsGraph::from_graph(&Graph {
        nodes: vec![
            Node {
                id: 0,
                x: 0.0,
                y: 0.0,
                z: None,
            },
            Node {
                id: 1,
                x: 0.0,
                y: 0.0,
                z: None,
            },
            Node {
                id: 2,
                x: 0.0,
                y: 0.0,
                z: None,
            },
        ],
        edges: vec![
            Edge {
                u: 0,
                v: 1,
                weight: 10,
            },
            Edge {
                u: 0,
                v: 2,
                weight: 7,
            },
        ],
    });

    let mut duals = DualState::new(3);

    duals.try_set(0, 8).unwrap();
    duals.try_set(1, 12).unwrap();
    duals.try_set(2, 1).unwrap();

    let tight_neighbors = graph
        .try_tight_neighbors(&duals, 0)
        .expect("tight neighbors should compute");

    assert_eq!(tight_neighbors, vec![1]);
}

#[test]
fn tight_search_ignores_non_tight_edges() {
    let graph = EdmondsGraph::from_graph(&Graph {
        nodes: vec![
            Node {
                id: 0,
                x: 0.0,
                y: 0.0,
                z: None,
            },
            Node {
                id: 1,
                x: 0.0,
                y: 0.0,
                z: None,
            },
            Node {
                id: 2,
                x: 0.0,
                y: 0.0,
                z: None,
            },
        ],
        edges: vec![
            Edge {
                u: 0,
                v: 1,
                weight: 10,
            },
            Edge {
                u: 0,
                v: 2,
                weight: 7,
            },
        ],
    });

    let mut duals = DualState::new(3);
    duals.try_set(0, 8).unwrap();
    duals.try_set(1, 12).unwrap();
    duals.try_set(2, 1).unwrap();

    let matching = MatchingState::new(3);

    let result =
        search_tight_alternating_tree(&graph, &duals, &matching, 0).expect("search should succeed");

    assert_eq!(result, SearchResult::AugmentingPath(vec![0, 1]));
}

#[test]
fn initializes_duals_from_minimum_incident_edge_weight() {
    let graph = EdmondsGraph::from_graph(&Graph {
        nodes: vec![
            Node {
                id: 0,
                x: 0.0,
                y: 0.0,
                z: None,
            },
            Node {
                id: 1,
                x: 0.0,
                y: 0.0,
                z: None,
            },
            Node {
                id: 2,
                x: 0.0,
                y: 0.0,
                z: None,
            },
        ],
        edges: vec![
            Edge {
                u: 0,
                v: 1,
                weight: 10,
            },
            Edge {
                u: 0,
                v: 2,
                weight: 6,
            },
        ],
    });

    let duals = try_initialize_duals(&graph).expect("dual initialization should succeed");

    assert_eq!(duals.try_get(0).unwrap(), 6);
    assert_eq!(duals.try_get(1).unwrap(), 10);
    assert_eq!(duals.try_get(2).unwrap(), 6);
}

#[test]
fn initialized_duals_produce_non_negative_slack() {
    let graph = EdmondsGraph::from_graph(&Graph {
        nodes: vec![
            Node {
                id: 0,
                x: 0.0,
                y: 0.0,
                z: None,
            },
            Node {
                id: 1,
                x: 0.0,
                y: 0.0,
                z: None,
            },
            Node {
                id: 2,
                x: 0.0,
                y: 0.0,
                z: None,
            },
        ],
        edges: vec![
            Edge {
                u: 0,
                v: 1,
                weight: 10,
            },
            Edge {
                u: 0,
                v: 2,
                weight: 6,
            },
            Edge {
                u: 1,
                v: 2,
                weight: 8,
            },
        ],
    });

    let duals = try_initialize_duals(&graph).expect("dual initialization should succeed");

    for u in 0..graph.adjacency.len() {
        for v in graph.neighbors(u) {
            let slack = duals.try_slack(&graph, u, v).expect("slack should compute");

            assert!(slack >= 0, "negative slack on edge ({u}, {v}): {slack}");
        }
    }
}
