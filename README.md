# tsplib-rs

Rust backend for parsing, viewing, and solving symmetric TSPLIB instances of
the Travelling Salesman Problem. Exposes a small REST API (axum) over the
problem instances in `./data` and implements MST, minimum-weight perfect
matching, and full TSP solvers (Greedy, Held–Karp, Christofides).

## Workspace layout

| Crate                | Purpose                                                            |
| -------------------- | ------------------------------------------------------------------ |
| `tsplib-core`        | Core models, TSPLIB distance functions, MST, graph types          |
| `tsplib-parser`      | Parser for the TSPLIB `.tsp` file format                          |
| `tsplib-solver`      | Solvers (Greedy, Held–Karp, Christofides) and matchers (MWPM)      |
| `tsplib-server`      | REST API over the instances in `./data`                            |
| `tsplib-dev-runner`  | Local harness for benchmarking/validating solvers and matchers     |
| `blossom-v` / `-sys` | Optional FFI bindings to Blossom V (excluded from the workspace)   |

## Requirements

- Rust / Cargo, installed from <https://rustup.rs/>

## Build and run

From the repository root:

```sh
cargo run -p tsplib-server --release
```

The server listens on:

```text
http://0.0.0.0:3000/
```

It reads problem instances and the known-solution list from the `./data`
directory, so run it from the repository root (the paths are relative).

### Optional: Blossom V matching

To build the server with the Blossom V minimum-weight perfect matcher enabled:

```sh
cargo run -p tsplib-server --features blossom-v --release
```

**NOTE**\
**Blossom V must be present on the machine and the environment variable
`BLOSSOM_V_PATH` must point to its root directory.**\
**Tested with Blossom V version 2.05, available from
<https://pub.ist.ac.at/~vnk/software/blossom5-v2.05.src.tar.gz>.**

## REST API

| Method | Path                                     | Description                                             |
| ------ | ---------------------------------------- | ------------------------------------------------------- |
| GET    | `/health`                                | Health check (returns `200 OK`)                         |
| GET    | `/problems`                              | List available instances with metadata                  |
| GET    | `/problems/{problemId}`                  | Parse and return a single instance                      |
| GET    | `/problems/{problemId}/adjacency_matrix` | Adjacency matrix for a specific instance                |
| GET    | `/problems/{problemId}/no_matrix`        | Specific instance without adjacency matrix              |
| GET    | `/problems/{problemId}/edges`            | Edge weight between two nodes                           |
| GET    | `/problems/{problemId}/edges/{nodeId}`   | All edges from a node for a specific instance           |
| POST   | `/problems`                              | Create a new problem instance and save it on the server |
| GET    | `/solutions`                             | Known solution costs for all instances                  |
| GET    | `/solutions/{problemId}`                 | Known solution cost for a single instance               |
| GET    | `/solver/algorithms`                     | Available TSP solver algorithms                         |
| POST   | `/solver/start`                          | Run a solver on an instance                             |
| POST   | `/cancel`                                | Cancel the running solver/processing task               |
| GET    | `/mst/algorithms`                        | Available minimum-spanning-tree algorithms              |
| GET    | `/mwpm/algorithms`                       | Available minimum-weight perfect-matching algos         |

`GET //problems/{problemId}/edges` expects a JSON body:

```json
{
    "from": 1,
    "to": 3
}
```

`POST /solver/start` expects a JSON body:

```json
{
  "algorithm": "Christofides",
  "problem_id": "eil51",
  "start_node": 1
}
```
Additionally it is possible to provide solver options:
```json
"solver_options": {
    "mst_algorithm" : "boruvka",
    "matcher_algorithm" : "edmonds_blossom"
}
```

`POST /problems` expects a JSON body:

```json
{
    "problem_id": "test123",
    "definition": "..."
}
```
IMPORTANT: The `definition` has to be a valid JSON string (e.g. created by JSON.stringify when using JavaScript)!

## Development

Run the test suite:

```sh
cargo test
```

The `tsplib-dev-runner` crate is a local harness for validating and comparing
solvers and matchers (e.g. the weighted Edmonds matcher against Blossom V):

```sh
cargo run -p tsplib-dev-runner --features blossom-v
```