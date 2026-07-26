# PledgePack Limitations

Known limitations, trade-offs, and areas for improvement.

---

## Platform Support

### Status: ✅ Resolved
CI (GitHub Actions) cross-compiles and publishes prebuilt binaries for 5 platform targets (Windows x64, Linux x64/arm64, macOS x64/arm64) on each release. The release workflow passes `-Dtarget` to `zig build` for correct cross-compilation of the Zig native library.

---

## Parallel Transform Pipeline

### Status: ✅ Resolved
The transform pipeline now uses rayon for parallel module transformation. The build loop is split into a BFS resolution phase followed by parallel transformation, with cache population and graph wiring.

---

## Source Maps in Production

### Status: ✅ Resolved
`pledge build` emits source maps with `none`, `inline`, and `external` options. Oxc's native source map generation is used.

---

## CSS Bundling

### Status: ✅ Resolved
Lightning CSS is integrated for CSS minification, autoprefixing, dead code elimination, and CSS code splitting aligned with JS chunk boundaries.

---

## Code Splitting for Dynamic Imports

### Status: ✅ Resolved
The optimizer splits dynamic `import()` calls into separate lazy-loaded chunks. The import map includes chunk mappings for dynamic imports.

---

## JS Plugin System — Full API

### Status: ✅ Resolved
The JS plugin host (powered by Boa engine) supports module graph access (`get_module_info`), custom resolvers (`resolve_id`), build lifecycle hooks (`on_build_start`, `on_build_end`), HMR interception (`on_hmr_update`), and a `PluginContext` for passing graph data. The Vite-compatible API provides `resolveId`, `load`, `transform`, `transformIndexHtml`, `configureServer`, `buildStart`, `buildEnd`, and `generateBundle` hooks.

---

## Tree Shaking for CSS-in-JS

### Status: ✅ Resolved
Static analysis for styled-components and emotion removes unused style definitions during tree shaking.

---

## Hot Reloading for Server-Only Code

### Status: ✅ Resolved
Server-only file changes are detected via `compute_server_dirs()` and `is_server_file()`. The dev server sends `server-reload` → `server-reload-complete` HMR updates, preserving WebSocket connections. A client-side banner UI shows reload status.

---

## Import Map — Version Deduplication

### Status: ✅ Resolved
The auto-generated import map now includes `scopes` entries for packages with multiple versions in nested `node_modules`. Monorepo setups with conflicting dependency versions resolve correctly per-scope.

---

## Built-in Test Runner UI

### Status: ✅ Resolved
`pledge test --watch` provides an interactive terminal UI with coverage reporting and browser-based test runner support for component tests.

---

## HTTPS Dev Server

### Status: ✅ Resolved
`pledge dev --https` enables HTTPS with automatic self-signed certificate generation via `rcgen`. Custom certificates are supported via `https.cert` and `https.key` config.

---

## Incremental Build Watch Mode

### Status: ✅ Resolved
`pledge build --watch` uses the function-level incremental cache. On file change, only affected modules are re-transformed and changed chunks are re-emitted.

---

## Binary Size

### Status: ✅ Resolved
- Release profile uses `strip = true`, `lto = "fat"`, `opt-level = 3`, `codegen-units = 1`, and `panic = "abort"` for maximum optimization.
- WASM plugin host crate removed — wasmtime dependency (~10MB) eliminated entirely.
- Release binary ~23.6MB (includes Oxc, Lightning CSS, Boa JS runtime, notify, tokio, axum).

---

## PledgeStack Full-Stack Runtime

### Status: ⚠️ Partial — Route Discovery Only

The PledgeStack adapter (`crates/adapter-pledgestack/`) provides comprehensive **route discovery and manifest generation** but does not yet implement runtime execution:

- ✅ **Frontend route discovery** — `app/` directory scanning with dynamic routes, layouts, loading/error boundaries
- ✅ **API route discovery** — `app/api/*/route.ts` with HTTP method detection
- ✅ **Rust backend route discovery** — `server/api/*.rs` and `.psx` files with `#[route(...)]` macro parsing
- ✅ **Middleware discovery** — Root and server middleware files detected
- ✅ **`.psx` → `.rs` copy** — Files copied for `cargo build` compatibility
- ✅ **Route manifest generation** — JSON manifest with all frontend + backend + middleware routes
- ✅ **Project scaffolding** — `pledge create pledgestack` generates full app structure
- ❌ **Rust backend compilation** — PledgePack does not compile or run the `server/` Rust backend
- ❌ **API route execution** — API routes are discovered but not executed in dev mode (requires manual proxy)
- ❌ **SSR rendering** — SSR/SSG is detected but no server-side React rendering pipeline exists
- ❌ **Middleware execution** — Middleware files are discovered but not loaded or executed
- ❌ **`.psx` transpilation** — `.psx` files are copied to `.rs` but no JSX→Rust transpilation occurs
- ❌ **Production server** — `pledge serve` only serves static files; no integrated API/SSR server
