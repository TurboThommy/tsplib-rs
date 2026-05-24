# tsplib-rs

Rust backend and local frontend for viewing and solving TSPLIB TSP instances.

## Requirements

- Rust/Cargo, installed from <https://rustup.rs/>
- Node.js 18 or newer
- npm

## Install After Cloning

From the repository root:

```powershell
cd tsplib-rs
cd frontend
npm install
```

The frontend currently has no external runtime dependencies, but `npm install` verifies the Node project and creates/uses the lockfile.

## Start The App

From the `frontend` folder:

```powershell
npm start
```

This starts the frontend adapter at:

```text
http://127.0.0.1:3001/
```

The adapter also tries to start the Rust backend automatically at:

```text
http://127.0.0.1:3000/
```

Open `http://127.0.0.1:3001/` in your browser.

## Start Backend Manually

If the backend does not start automatically, open a second terminal from the repository root:

```powershell
cargo run -p tsplib-server
```

This will start the backend without Blossom V minimum weight perfect matching (MWPM). To start the backend with support for Blossom V use the following command instead:
```powershell
cargo run -p tsplib-server --features blossom-v
```

Then reload:

```text
http://127.0.0.1:3001/
```

**NOTE**\
**The Blossom V MWPM requires that Blossom V exists on the machine and that the environment variable `BLOSSOM_V_PATH` is set to it's root directory.**\
**The operability was tested using version 2.05 of Blossom V which can be obtained from https://pub.ist.ac.at/~vnk/software/blossom5-v2.05.src.tar.gz**


## Useful Commands

Run only the frontend adapter without auto-starting the backend:

```powershell
cd frontend
npm run start:frontend
```

Run the backend from the frontend folder:

```powershell
cd frontend
npm run start:backend
```
