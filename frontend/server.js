const http = require("node:http");
const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");
const { spawn } = require("node:child_process");

const HOST = "127.0.0.1";
const PORT = Number(process.env.PORT || 3001);
const ROOT = __dirname;
const BACKEND_URL = (process.env.TSPLIB_BACKEND_URL || "http://127.0.0.1:3000").replace(/\/$/, "");
const BACKEND_DIR = path.resolve(ROOT, "..");
const DATA_DIR = path.join(BACKEND_DIR, "data");
const AUTO_START_BACKEND = process.env.TSPLIB_AUTO_START_BACKEND !== "0";

let backendProcess = null;
let latestJob = createIdleJob();

if (AUTO_START_BACKEND) {
  ensureBackendStarted();
}

const server = http.createServer(async (req, res) => {
  try {
    const url = new URL(req.url, `http://${req.headers.host}`);

    if (url.pathname === "/" && req.method === "GET") {
      return serveFile(res, path.join(ROOT, "index.html"), "text/html; charset=utf-8");
    }

    if (url.pathname === "/api.yaml" && req.method === "GET") {
      return serveFile(res, path.join(ROOT, "api.yaml"), "application/yaml; charset=utf-8");
    }

    if (!url.pathname.startsWith("/api/v1/")) {
      return error(res, 404, "NOT_FOUND", "Endpoint not found.");
    }

    if (url.pathname === "/api/v1/info" && req.method === "GET") {
      const backend = await backendHealth();
      return json(res, 200, {
        name: "TSPLIB Frontend Adapter",
        version: "1.0.0",
        apiVersion: "1.0.0",
        backendUrl: BACKEND_URL,
        backendStatus: backend.ok ? "reachable" : "unreachable"
      });
    }

    if (url.pathname === "/api/v1/algorithms" && req.method === "GET") {
      return json(res, 200, await listAlgorithms());
    }

    if ((url.pathname === "/api/v1/problems" || url.pathname === "/api/v1/problems/tsplib") && req.method === "GET") {
      return json(res, 200, await listProblemSummaries());
    }

    if (url.pathname.startsWith("/api/v1/problems/") && req.method === "GET") {
      const id = decodeURIComponent(url.pathname.slice("/api/v1/problems/".length));
      return json(res, 200, await getProblem(id));
    }

    if (url.pathname === "/api/v1/solver/start" && req.method === "POST") {
      const body = await readJson(req);
      return json(res, 200, startSolver(body));
    }

    if (url.pathname === "/api/v1/solver/status" && req.method === "GET") {
      return json(res, 200, currentSolverStatus());
    }

    if (url.pathname === "/api/v1/solver/result" && req.method === "GET") {
      if (!latestJob.result) {
        return error(res, 404, "RESULT_NOT_FOUND", "No solver result is available.");
      }

      return json(res, 200, latestJob.result);
    }

    if (url.pathname === "/api/v1/solver/cancel" && req.method === "POST") {
      await cancelSolver();
      return json(res, 200, { status: latestJob.status, message: "Solver cancelled." });
    }

    if (url.pathname === "/api/v1/processing/cancel" && req.method === "POST") {
      await cancelBackendProcessing();
      return json(res, 200, { status: "cancelled", message: "Processing cancellation requested." });
    }

    return error(res, 404, "NOT_FOUND", "Endpoint not found.");
  } catch (err) {
    if (err && err.code === "INVALID_JSON") {
      return error(res, 400, "INVALID_JSON", err.message);
    }

    const status = err.status || 500;
    const code = err.code || "INTERNAL_ERROR";
    const message = err.message || "Unexpected server error.";
    return error(res, status, code, message, err.details || {});
  }
});

server.listen(PORT, HOST, () => {
  console.log(`TSP graph viewer running at http://${HOST}:${PORT}`);
  console.log(`Using TSPLIB backend at ${BACKEND_URL}`);
});

function ensureBackendStarted() {
  if (!fs.existsSync(path.join(BACKEND_DIR, "Cargo.toml"))) {
    console.warn("Backend workspace not found. Start tsplib-server manually and set TSPLIB_BACKEND_URL if needed.");
    return;
  }

  backendHealth().then(status => {
    if (status.ok) return;

    backendProcess = spawn("cargo", ["run", "-p", "tsplib-server"], {
      cwd: BACKEND_DIR,
      shell: process.platform === "win32",
      stdio: ["ignore", "pipe", "pipe"]
    });

    backendProcess.stdout.on("data", chunk => process.stdout.write(`[backend] ${chunk}`));
    backendProcess.stderr.on("data", chunk => process.stderr.write(`[backend] ${chunk}`));
    backendProcess.on("error", err => {
      backendProcess = null;
      console.warn(`Could not start backend automatically: ${err.message}`);
    });
    backendProcess.on("exit", code => {
      backendProcess = null;
      if (code !== 0) {
        console.warn(`Backend process exited with code ${code}.`);
      }
    });
  }).catch(err => {
    console.warn(`Could not check backend status: ${err.message}`);
  });
}

async function backendHealth() {
  try {
    const response = await fetchWithTimeout(`${BACKEND_URL}/algorithms`, { method: "GET" }, 1000);
    const contentType = response.headers.get("content-type") || "";
    return { ok: response.ok && contentType.includes("application/json") };
  } catch {
    return { ok: false };
  }
}

async function listAlgorithms() {
  const raw = await backendJson("/algorithms");
  const algorithms = Array.isArray(raw) ? raw : [];

  return algorithms.map(id => ({
    id,
    name: algorithmName(id),
    description: algorithmDescription(id),
    parameters: [
      { name: "startNode", type: "integer", required: false, default: 0 }
    ]
  })).sort((left, right) => algorithmRank(left.id) - algorithmRank(right.id));
}

async function listProblemSummaries() {
  const ids = await backendJson("/problems");
  if (!Array.isArray(ids)) {
    throw httpError(502, "BACKEND_RESPONSE_INVALID", "Backend returned an invalid problem list.");
  }

  return ids.map(id => {
    const metadata = readProblemMetadata(id);
    return {
      id,
      name: metadata.name || id,
      nodeCount: metadata.dimension || null,
      edgeWeightType: metadata.edgeWeightType || null
    };
  });
}

async function getProblem(id) {
  const raw = await backendJson(`/problems/${encodeURIComponent(id)}`);
  const metadata = readProblemMetadata(id);
  const nodes = Array.isArray(raw.nodes)
    ? raw.nodes.map((node, index) => ({
      id: index,
      x: node.x,
      y: node.y,
      name: String(node.id ?? index + 1)
    }))
    : [];

  return {
    id,
    name: raw.name || metadata.name || id,
    problemType: normalizeProblemType(raw.problem_type),
    source: "tsplib-rs",
    nodeCount: nodes.length,
    edgeWeightType: metadata.edgeWeightType || "TSPLIB",
    nodes,
    adjacencyMatrix: raw.adjacency_matrix || [],
    metadata: {
      originalProblemType: raw.problem_type,
      fixedEdges: raw.fixed_edges || null,
      backendUrl: BACKEND_URL
    }
  };
}

function startSolver(body) {
  if (latestJob.status === "running") {
    throw httpError(409, "SOLVER_RUNNING", "A solver job is already running.");
  }

  if (!body || typeof body.problemId !== "string" || typeof body.algorithmId !== "string") {
    throw httpError(400, "INVALID_SOLVE_REQUEST", "problemId and algorithmId are required.");
  }

  const jobId = `job-${Date.now().toString(36)}-${crypto.randomBytes(3).toString("hex")}`;
  const algorithm = toBackendAlgorithm(body.algorithmId);
  const startNode = Number.isInteger(body.parameters?.startNode)
    ? body.parameters.startNode + 1
    : 1;

  latestJob = {
    jobId,
    problemId: body.problemId,
    algorithmId: body.algorithmId,
    status: "running",
    progress: 0,
    currentIteration: 0,
    maxIterations: null,
    bestDistance: null,
    elapsedMs: 0,
    solutionId: null,
    result: null,
    startedAt: Date.now()
  };

  runSolverJob({ jobId, problemId: body.problemId, algorithmId: body.algorithmId, algorithm, startNode });

  return {
    jobId,
    status: "running",
    message: "Solver started."
  };
}

async function runSolverJob({ jobId, problemId, algorithmId, algorithm, startNode }) {
  try {
    const raw = await backendJson("/solver/start", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        problem_id: problemId,
        algorithm,
        start_node: startNode
      })
    }, 120000);

    if (latestJob.jobId !== jobId) return;

    const runtimeMs = Date.now() - latestJob.startedAt;
    const totalDistance = typeof raw.cost === "number" ? raw.cost : raw.totalDistance;

    latestJob.status = "finished";
    latestJob.progress = 1;
    latestJob.currentIteration = null;
    latestJob.bestDistance = totalDistance;
    latestJob.elapsedMs = runtimeMs;
    latestJob.solutionId = `solution-${jobId}`;
    latestJob.result = {
      problemId,
      algorithmId,
      tour: normalizeTour(raw.tour),
      totalDistance,
      runtimeMs,
      status: "finished"
    };
  } catch (err) {
    if (latestJob.jobId !== jobId) return;

    latestJob.status = "failed";
    latestJob.elapsedMs = Date.now() - latestJob.startedAt;
    latestJob.result = {
      problemId,
      algorithmId,
      tour: [],
      totalDistance: null,
      runtimeMs: latestJob.elapsedMs,
      status: "failed",
      error: err.message
    };
  }
}

function currentSolverStatus() {
  return {
    jobId: latestJob.jobId,
    status: latestJob.status,
    progress: latestJob.status === "running" ? 0.5 : latestJob.progress,
    currentIteration: latestJob.currentIteration,
    maxIterations: latestJob.maxIterations,
    bestDistance: latestJob.bestDistance,
    elapsedMs: elapsed(),
    solutionId: latestJob.solutionId
  };
}

async function cancelSolver() {
  if (latestJob.status !== "running") {
    return;
  }

  latestJob.status = "cancelled";
  latestJob.progress = 0;
  latestJob.elapsedMs = elapsed();

  await cancelBackendProcessing();
}

async function cancelBackendProcessing() {
  try {
    await backendJson("/cancel", { method: "POST" }, 5000);
  } catch {
    // Cancellation is best-effort; the backend may already be idle or unreachable.
  }
}

async function backendJson(pathname, options = {}, timeoutMs = 30000) {
  const response = await fetchWithTimeout(`${BACKEND_URL}${pathname}`, options, timeoutMs);
  const contentType = response.headers.get("content-type") || "";
  const payload = contentType.includes("application/json")
    ? await response.json().catch(() => null)
    : await response.text().catch(() => "");

  if (!response.ok) {
    const message = typeof payload === "string"
      ? payload
      : payload?.error?.message || `Backend HTTP ${response.status}`;
    throw httpError(response.status, "BACKEND_ERROR", message || `Backend HTTP ${response.status}`);
  }

  return payload;
}

async function fetchWithTimeout(url, options, timeoutMs) {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), timeoutMs);

  try {
    return await fetch(url, { ...options, signal: controller.signal });
  } catch (err) {
    if (err.name === "AbortError") {
      throw httpError(504, "BACKEND_TIMEOUT", "Backend did not answer in time.");
    }
    throw httpError(502, "BACKEND_UNREACHABLE", `Backend is not reachable at ${BACKEND_URL}.`);
  } finally {
    clearTimeout(timeout);
  }
}

function serveFile(res, filePath, contentType) {
  fs.readFile(filePath, (err, buffer) => {
    if (err) {
      return error(res, 404, "FILE_NOT_FOUND", "File not found.");
    }

    res.writeHead(200, {
      "Content-Type": contentType,
      "Cache-Control": "no-store"
    });
    res.end(buffer);
  });
}

function json(res, status, body) {
  const payload = JSON.stringify(body, null, 2);
  res.writeHead(status, {
    "Content-Type": "application/json; charset=utf-8",
    "Cache-Control": "no-store",
    "Content-Length": Buffer.byteLength(payload)
  });
  res.end(payload);
}

function error(res, status, code, message, details = {}) {
  return json(res, status, {
    error: {
      code,
      message,
      details
    }
  });
}

function readJson(req) {
  return new Promise((resolve, reject) => {
    let body = "";

    req.on("data", chunk => {
      body += chunk;

      if (body.length > 1_000_000) {
        req.destroy();
        const err = new Error("Request body is too large.");
        err.code = "INVALID_JSON";
        reject(err);
      }
    });

    req.on("end", () => {
      try {
        resolve(body ? JSON.parse(body) : null);
      } catch {
        const err = new Error("Request body must be valid JSON.");
        err.code = "INVALID_JSON";
        reject(err);
      }
    });

    req.on("error", reject);
  });
}

function readProblemMetadata(id) {
  const filePath = path.join(DATA_DIR, `${id}.tsp`);

  try {
    const text = fs.readFileSync(filePath, "utf8");
    const metadata = {};

    for (const line of text.split(/\r?\n/)) {
      const match = line.match(/^\s*([A-Z_]+)\s*:\s*(.*?)\s*$/);
      if (!match) continue;

      const [, key, value] = match;
      if (key === "NAME") metadata.name = value;
      if (key === "DIMENSION") metadata.dimension = Number(value);
      if (key === "EDGE_WEIGHT_TYPE") metadata.edgeWeightType = value;
      if (metadata.name && metadata.dimension && metadata.edgeWeightType) break;
    }

    return metadata;
  } catch {
    return {};
  }
}

function normalizeProblemType(value) {
  if (value === "ATSP" || value === "SOP") return "asymmetric";
  return "symmetric";
}

function normalizeTour(tour) {
  if (!Array.isArray(tour)) return [];
  return tour.map(node => Number(node) - 1);
}

function toBackendAlgorithm(id) {
  if (id === "nearest-neighbor") return "greedy";
  if (id === "exact-bruteforce") return "held_karp";
  if (id === "greedy" || id === "held_karp") return id;
  throw httpError(400, "UNKNOWN_ALGORITHM", `Algorithm '${id}' is not available.`);
}

function algorithmName(id) {
  if (id === "held_karp") return "Held-Karp";
  if (id === "greedy") return "Greedy";
  return id;
}

function algorithmDescription(id) {
  if (id === "held_karp") return "Exact dynamic-programming solver for small and medium TSP instances.";
  if (id === "greedy") return "Greedy baseline solver using the nearest reachable node.";
  return "TSPLIB backend algorithm.";
}

function algorithmRank(id) {
  if (id === "greedy") return 0;
  if (id === "held_karp") return 1;
  return 2;
}

function elapsed() {
  if (latestJob.status === "running" && latestJob.startedAt) {
    return Date.now() - latestJob.startedAt;
  }

  return latestJob.elapsedMs || 0;
}

function createIdleJob() {
  return {
    jobId: null,
    problemId: null,
    algorithmId: null,
    status: "idle",
    progress: 0,
    currentIteration: null,
    maxIterations: null,
    bestDistance: null,
    elapsedMs: 0,
    solutionId: null,
    result: null,
    startedAt: null
  };
}

function httpError(status, code, message, details = {}) {
  const err = new Error(message);
  err.status = status;
  err.code = code;
  err.details = details;
  return err;
}

process.on("exit", () => {
  if (backendProcess) {
    backendProcess.kill();
  }
});
