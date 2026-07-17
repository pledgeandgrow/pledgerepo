# PledgePack — Dependencies

## Project Overview

PledgePack is a Rust-native bundler and dev server, similar in scope to Turbopack.
It supports React, Solid, Svelte, Vue, and TypeScript frameworks.
The project also includes a Next.js-like full-stack framework where the backend
is implemented in Rust instead of Node.js or TypeScript — enabling faster builds,
lower memory usage, and native performance for SSR/routing/API routes.

## Workspace Dependencies (shared across crates)

| # | Crate | Version | Category | Used By |
|---|-------|---------|----------|---------|
| 1 | `serde` | 1 (derive) | Serialization | core, cli, dev-server, cache, resolver, js-plugin-host, optimizer, adapter-react |
| 2 | `serde_json` | 1 | Serialization | core, cli, dev-server, cache, resolver, js-plugin-host, optimizer, adapter-react |
| 3 | `bincode` | 1 | Binary serialization | core, cache |
| 4 | `tokio` | 1 (full) | Async runtime | core, cli, dev-server, cache |
| 5 | `axum` | 0.8 | HTTP server | cli, dev-server |
| 6 | `tower-http` | 0.6 (fs, cors) | HTTP middleware | cli, dev-server |
| 7 | `tokio-tungstenite` | 0.26 | WebSocket (HMR) | dev-server |
| 8 | `oxc` | 0.36 (full) | JS/TS/JSX compiler | core, dev-server, adapter-react |
| 9 | `lightningcss` | 1.0.0-alpha.71 | CSS engine | core |
| 10 | `blake3` | 1 | Hashing (cache keys) | core, cache |
| 11 | `base64` | 0.22 | Base64 encoding | core |
| 12 | `image` | 0.25 (jpeg, png, webp, gif) | Image processing | core |
| 13 | `tracing` | 0.1 | Logging | all crates |
| 14 | `tracing-subscriber` | 0.3 (env-filter) | Logging setup | cli |
| 15 | `anyhow` | 1 | Error handling | core, cli, dev-server, cache, resolver, js-plugin-host, optimizer, adapter-react |
| 16 | `thiserror` | 2 | Typed errors | core |
| 17 | `clap` | 4 (derive) | CLI parsing | cli |
| 18 | `clap_complete` | 4 | Shell completions | cli |
| 19 | `indicatif` | 0.17 | Progress bars | cli |
| 20 | `inquire` | 0.7 | Interactive prompts | cli |
| 21 | `notify` | 8 | File watching | core, cli, dev-server |
| 22 | `notify-debouncer-full` | 0.7 | Debounced file watching | core, dev-server |
| 23 | `libc` | 0.2 | C library bindings (Linux) | dev-server |
| 24 | `rayon` | 1 | Parallelism | core, cli, optimizer |
| 25 | `dashmap` | 6 | Concurrent HashMap | core, cache, resolver |
| 26 | `mimalloc` | 0.1 | Global allocator | cli |
| 27 | `tikv-jemallocator` | 0.6 (profiling) | Alt allocator (jemalloc) | cli (optional) |
| 28 | `camino` | 1 | Typed UTF-8 paths | cli |
| 29 | `globset` | 0.4 | Glob pattern matching | core, cli, optimizer |
| 30 | `regex` | 1 | Regex engine | core |
| 31 | `memmap2` | 0.9 | Memory-mapped I/O | core, cache |
| 32 | `comfy-table` | 7 | CLI tables | core, cli |
| 33 | `serde_yaml` | 0.9 | YAML parsing | core |
| 34 | `miette` | 7 (fancy) | Error diagnostics | core, cli |
| 35 | `clap_mangen` | 0.2 | Man page generation | cli |
| 36 | `humansize` | 2 | File size formatting | core |
| 37 | `similar` | 2 (text) | Diff algorithm (HMR patches) | dev-server |
| 38 | `opener` | 0.7 | Cross-platform browser opening | dev-server |
| 39 | `local-ip-address` | 0.6 | Network IP detection | dev-server |
| 40 | `schemars` | 1 | JSON Schema generation (config) | core, cli |

## Sub-crate Local Dependencies (not in workspace)

| # | Crate | Version | Used By | Purpose |
|---|-------|---------|---------|---------|
| 41 | `reqwest` | 0.12 | core, dev-server | HTTP client (webhooks, proxy) |
| 42 | `rustls` | 0.23 | dev-server | TLS |
| 43 | `rustls-pemfile` | 2 | dev-server | TLS cert parsing |
| 44 | `tokio-rustls` | 0.26 | dev-server | Async TLS |
| 45 | `futures-util` | 0.3 | dev-server | Async utilities |
| 46 | `flate2` | 1 | core | Gzip compression |
| 47 | `brotli` | 7 | core | Brotli compression |
| 48 | `chrono` | 0.4 | core | Date/time formatting |
| 49 | `dialoguer` | 0.11 | cli | Interactive dialogs |
| 50 | `console` | 0.15 | cli | Terminal styling |
| 51 | `atty` | 0.2 | cli | TTY detection |
| 52 | `boa_engine` | 0.20 | js-plugin-host | JS engine for plugins |
| 53 | `windows-sys` | 0.59 | dev-server (Windows only) | Win32 API |
| 54 | `bytemuck` | 1.21 | dev-server (Windows only) | Byte casting |

## Internal Crates (path dependencies)

| # | Crate | Path | Purpose |
|---|-------|------|---------|
| 55 | `pledgepack-core` | `crates/core` | Build engine, transforms, config, module graph |
| 56 | `pledgepack-cache` | `crates/cache` | Function-level incremental cache with disk persistence |
| 57 | `pledgepack-resolver` | `crates/resolver` | Module resolution |
| 58 | `pledgepack-dev-server` | `crates/dev-server` | Dev server with HMR, WebSocket, proxy |
| 59 | `pledgepack-optimizer` | `crates/optimizer` | Tree shaking, minification, chunk splitting |
| 60 | `pledgepack-js-plugin-host` | `crates/js-plugin-host` | JS plugin runtime (Boa engine) |
| 61 | `pledgepack-adapter-react` | `crates/adapter-react` | React Fast Refresh adapter |
| 62 | `pledgepack-adapter-solid` | `crates/adapter-solid` | Solid HMR adapter |
| 63 | `pledgepack-adapter-next` | `crates/adapter-next` | Next.js compatibility adapter |
| 64 | `pledgepack-adapter-tanstack` | `crates/adapter-tanstack` | TanStack router adapter |
| 65 | `pledgepack-adapter-pledgestack` | `crates/adapter-pledgestack` | PledgeStack framework adapter (React frontend + Rust backend, .rs/.psx) |
| 66 | `pledgepack-native-sys` | `native-sys` | Zig FFI (native graph operations) |

## Crates Added During Integration Sessions

### Session 1 — Core Infrastructure

| Crate | Replaced | Impact |
|-------|----------|--------|
| `globset` | Manual glob matching in `asset_pipeline.rs` | Faster pattern compilation, safer matching |
| `regex` | Manual string manipulation in `config.rs` | Correct regex via `OnceLock` cached patterns |
| `notify-debouncer-full` | Manual debounce in `lib.rs`, `plugin_system.rs`, `watcher.rs` | Coalesced events, fewer false triggers |
| `memmap2` | `std::fs::read` for large files | Zero-copy reads for files >64KB |
| `comfy-table` | Manual `println!` table formatting | Auto-sized columns, cleaner CLI output |

### Session 2 — Quality of Life

| Crate | Replaced | Impact |
|-------|----------|--------|
| `serde_yaml` | Hand-rolled line-based YAML parser | Robust nested YAML, handles edge cases |
| `miette` | Plain `anyhow` error messages | Graphical error diagnostics with source spans |
| `clap_mangen` | No man pages existed | Auto-generates roff man pages for package managers |
| `humansize` | 4 duplicate `format_bytes` functions | Unified `format_size()`, consistent units |

### Session 3 — HMR, Dev Server UX, Config Validation

| Crate | Replaced | Impact |
|-------|----------|--------|
| `similar` | Hand-rolled LCS diff (200-line cap) | Myers algorithm, no line limit, faster for small edits |
| `opener` | Platform-specific `open_browser` (20 lines) | Single call, handles WSL/sandboxed macOS |
| `local-ip-address` | No network URL display | Shows `→ Network: http://192.168.x.x:3000` |
| `schemars` | Manual config field validation | Auto-generates JSON Schema, `pledge schema` command |

## Build Profile

```toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true

[profile.dev]
opt-level = 1
```

## Summary

- **54 external crates** + **12 internal crates** = **66 total packages**
- **13 crates** added during integration sessions
- All additions are pure replacements of manual code or new capabilities
- No dependency conflicts or version mismatches
- Workspace uses resolver v2 for feature unification
