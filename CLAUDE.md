# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Aqueduct is a node-based dataflow engine inspired by vvvv. It provides real-time tick-driven graph evaluation with hot patching, a pluggable node system, and a React Flow-based web UI. Supports both WebSocket and Tauri desktop transport layers.

## Build & Development Commands

### Rust (workspace root)
- `cargo check` — type-check all crates
- `cargo test` — run all tests
- `cargo test -p aqueduct-core` — run tests for a single crate
- `cargo clippy` — lint (configured: all + pedantic warnings, `unsafe_code` forbidden)

### Frontend (`frontend/`)
- `npm run dev` — start Vite dev server (localhost:5173)
- `npm run build` — TypeScript check + Vite build
- `npm run lint` — ESLint
- `npx tsc -b` — TypeScript type-check only
- `npm run tauri:dev` — run Tauri desktop app in dev mode

## Architecture

### Crate Dependency Chain

```
aqueduct-protocol          (pure data types, no runtime deps)
    ↓
aqueduct-core              (LiveGraph, TickDriver, CompiledGraph, PinStore)
    ↓
aqueduct-nodes             (built-in nodes: math, string, logic, time, convert)
    ↓
aqueduct-server            (abstract TransportServer/TransportSession traits, message dispatch)
    ↓
├── aqueduct-server-ws     (axum WebSocket implementation, default port 9400)
└── aqueduct-server-tauri  (Tauri v2 plugin integration)
```

### Key Concepts

- **PinStore**: Central `HashMap<PinId, PinValue>` — all pin values live here, not in channels. Provides tick-wide snapshot consistency.
- **Hot Patching**: Mutation → validation → recompile → `ArcSwap::store()`. Current tick finishes with old graph; next tick picks up new one atomically.
- **TickDriver**: Evaluates nodes synchronously in topological order each tick. Async work is spawned and tracked via generation IDs.
- **Transport abstraction**: `TransportServer` trait with implementations for WebSocket and Tauri. Frontend uses a matching abstract `Transport` interface.

### Frontend Structure

- **State**: Zustand stores (`graphStore`, `pinStore`)
- **UI**: shadcn/ui + Tailwind CSS + React Flow (`@xyflow/react`)
- **Protocol layer**: `Transport` interface with `WebSocketTransport` and `TauriTransport` implementations in `src/protocol/`
- **Hooks**: `useTransport` (transport singleton + message routing), `useGraphSync` (graph state sync)

### Message Protocol

Client↔Server communication uses envelope format `{ request_id, body, graph_rev }` with typed message bodies. Messages defined in `aqueduct-protocol` (Rust) and mirrored in `frontend/src/types.ts`.

## Conventions

- Rust edition 2021, MSRV 1.77
- Clippy pedantic enabled across workspace
- `unsafe_code` is forbidden
- TypeScript strict mode
