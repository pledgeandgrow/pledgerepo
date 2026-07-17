# Dev Server & HMR

## `pledge dev` — Development Server

### Overview
The dev server serves source files from `src/` with on-demand Oxc transforms. Unlike `pledge build` which pre-builds everything to `dist/`, the dev server transforms each file when requested by the browser.

### Routes

| Route | Handler | Description |
|-------|---------|-------------|
| `GET /` | `index_handler` | Serves HTML shell with React shim + HMR client + error overlay |
| `GET /__pledge_hmr` | `hmr_websocket_handler` | WebSocket endpoint for HMR updates |
| `GET /__pledge_error` | `error_overlay_handler` | Error overlay endpoint |
| `GET /*path` | `module_handler` | Transforms and serves any source file |
| `GET /api/*` | `proxy_handler` | Proxied API requests (if proxy configured) |

### On-Demand Transform Flow

```
Browser requests /src/index.tsx
        │
        ▼
   module_handler
   ├── Resolve file on disk (with extension fallback)
   │   └── /src/utils.js → tries .tsx, .ts, .jsx, .mjs, .json
   ├── Read file content (Zig read_file)
   ├── Determine ModuleKind from extension
   ├── Oxc Transform:
   │   ├── Parse (SourceType from extension)
   │   ├── Semantic analysis (symbols + scopes)
   │   ├── Transform (JSX → React.createElement, TS → strip types)
   │   └── Codegen (JavaScript output)
   ├── Rewrite imports: "./utils" → "./utils.js"
   ├── Inject HMR boundary: import.meta.hot.accept()
   └── Return with Content-Type: application/javascript
```

### Import Rewriting

The browser's ES module loader requires file extensions in import specifiers. Pledge uses Oxc AST-based rewriting with string fallback:

| Source import | Rewritten to |
|---------------|-------------|
| `import { foo } from "./utils"` | `import { foo } from "./utils.js"` |
| `import { bar } from "../helpers"` | `import { bar } from "../helpers.js"` |
| `import { x } from "@/components"` | `import { x } from "/src/components"` (alias rewriting) |
| `import React from "react"` | Resolved via import map (dep pre-bundling) |
| `import('./lazy')` | `import('./lazy.js')` (dynamic import rewriting) |

### Extension Fallback

When the browser requests `/src/utils.js` (because import rewriting added `.js`), the dev server:
1. Checks if `utils.js` exists on disk → no
2. Tries `utils.tsx` → no
3. Tries `utils.ts` → yes! Serves transformed `utils.ts`

Order: `.tsx` → `.ts` → `.jsx` → `.mjs` → `.json`

Also supports: `.vue` → `.svelte` → `.astro` → `.css` → `.worker.js` → `.module.css`

### Web Worker Support

The dev server transforms `new Worker(new URL('./worker.ts', import.meta.url))` patterns:
- Rewrites to `new Worker('/src/worker.js')` for browser compatibility
- `new SharedWorker(new URL(...))` patterns also supported
- `.worker.js` / `.worker.ts` extensions detected as worker modules
- Worker scripts served with same on-demand transform pipeline

### Dynamic Import Support

`import('./lazy')` specifiers are detected via Oxc AST `ImportExpression` visitor:
- Relative specifiers get `.js` extension: `import('./lazy')` → `import('./lazy.js')`
- String-based fallback if parsing fails
- Dynamic imports collected in `TransformOutput.dynamic_imports` for future code splitting
- Async chunks can be loaded on-demand by the browser

### HTML Shell

The dev server generates HTML with:
- `<div id="root">` mount point
- Inline React shim (`React.createElement` minimal implementation)
- `<script type="module" src="/src/index.tsx">` entry point
- WebSocket HMR client script
- **Error overlay**: Full-screen overlay with error message, file path, source context, and color-coded line numbers
- **CSS HMR**: `updatePledgeCSS()` and `fetchPledgeCSS()` functions for injecting/updating `<style>` tags
- **Auto-reconnect**: WebSocket reconnects with exponential backoff on disconnect

### React Shim

Since React is not installed, a minimal `React.createElement` is injected:
- Creates DOM elements for string types
- Handles `children`, `className`, `style`, `onClick` props
- Supports `React.Fragment` (renders as `<div>`)
- No virtual DOM diffing (direct DOM manipulation)

## HMR (Hot Module Replacement)

### Architecture

```
File saved on disk
        │
        ▼
   notify crate watcher (recursive, project root)
        │
        ▼
   Debounce 150ms (batch rapid changes)
        │
        ▼
   Broadcast WebSocket message to all clients
   {
     "type": "update",
     "path": "/src/index.tsx"
   }
        │
        ▼
   Client-side: reloads the changed script tag
   with ?t=timestamp cache buster
```

### File Watcher
- **Crate**: `notify` (cross-platform file system notifications)
- **Scope**: Recursive watch on project root
- **Debounce**: 150ms to batch rapid saves (e.g., format-on-save + content change)
- **Filter**: Only triggers on `.ts`, `.tsx`, `.js`, `.jsx`, `.css`, `.json` files

### WebSocket Protocol
- **Endpoint**: `ws://localhost:3000/__pledge_hmr`
- **Connection**: Client connects on page load
- **Messages**:
  - `{ "type": "connected", "message": "Pledge HMR connected" }` — on connect
  - `{ "type": "update", "path": "/src/index.tsx" }` — on JS/TS file change
  - `{ "type": "update", "path": "/src/style.css", "css": "..." }` — on CSS file change (with content)
  - `{ "type": "error", "message": "...", "file": "...", "source": "..." }` — on transform error
  - `{ "type": "server-reload", "path": "...", "message": "Server code changed — reloading..." }` — on server-only file change
  - `{ "type": "server-reload-complete", "path": "...", "message": "Server code reloaded successfully" }` — after server reload completes

### Client-Side HMR
```javascript
const ws = new WebSocket('ws://' + location.host + '/__pledge_hmr');
ws.onmessage = (event) => {
    const data = JSON.parse(event.data);
    if (data.type === 'update') {
        if (data.css) {
            // CSS HMR: update <style> tag in-place
            updatePledgeCSS(data.path, data.css);
        } else {
            // JS HMR: reload the changed script
            const links = document.querySelectorAll('script[src="' + data.path + '"]');
            links.forEach(link => {
                const newLink = document.createElement('script');
                newLink.type = 'module';
                newLink.src = data.path + '?t=' + Date.now();
                link.replaceWith(newLink);
            });
        }
    } else if (data.type === 'error') {
        // Show error overlay with source context
        showPledgeError(data.message, data.file, data.source);
    }
};
```

### HMR Boundary Injection

For `.ts`, `.tsx`, `.js`, `.jsx` files, the dev server appends:
```javascript
// HMR boundary
if (import.meta.hot) {
    import.meta.hot.accept();
}
```

This allows modules to accept updates without full page reloads.

### React Fast Refresh

For React components in development mode, the dev server injects Fast Refresh code:

1. **Component detection**: Scans for function declarations and arrow functions with capitalized names
2. **Registration**: Components registered in `window.__pledge_fast_refresh` registry
3. **State preservation**: On HMR update, component state is preserved via React's `useState` hooks
4. **Boundary injection**: `import.meta.hot.accept()` with component list for targeted updates

```javascript
// Injected by Pledge for React Fast Refresh
if (import.meta.hot) {
    import.meta.hot.accept();
    window.__pledge_fast_refresh = window.__pledge_fast_refresh || {};
    window.__pledge_fast_refresh[import.meta.url] = ['App', 'Header'];
}
```

### Framework-Specific Dev Support

| Framework | Dev Features |
|-----------|-------------|
| **React** | Fast Refresh, AST-based component detection, classic JSX shim |
| **Solid** | Automatic JSX runtime, `development: true` mode, dedicated adapter crate |
| **Vue** | SFC parsing, scoped CSS, render function compilation |
| **Svelte** | SFC parsing, scoped CSS, DOM render functions |
| **Astro** | Frontmatter parsing, async render, style extraction |

## Error Overlay

The dev server includes an in-browser error overlay for transform errors and runtime errors:

### Transform Errors
- **Source context**: Shows the error line with surrounding code (5 lines before/after)
- **Color-coded**: Line numbers in gray, error line highlighted in red
- **File path**: Full file path displayed at top of overlay
- **Auto-clear**: Overlay disappears when the next successful HMR update arrives
- **WebSocket delivery**: Errors pushed to all connected clients in real-time

### Runtime Errors
- **window.error events**: Catches uncaught JavaScript errors via `window.addEventListener('error')`
- **Unhandled promise rejections**: Catches via `window.addEventListener('unhandledrejection')`
- **Stack traces**: Runtime error stack traces displayed in the overlay
- **Auto-clear**: Overlay dismisses on next successful HMR update

### Error Message Format
```json
{
  "type": "error",
  "message": "Unexpected token in expression: '...'",
  "file": "/src/index.tsx",
  "source": "line 1\nline 2\nline 3 (error)\nline 4\nline 5"
}
```

## Auto-Open Browser

The dev server can automatically open the default browser when it starts:

### Configuration
In `pledge.config.ts`:
```typescript
export default defineConfig({
  dev_server: {
    open: true, // Auto-open browser on dev server start
  },
});
```

Or via CLI flag:
```bash
pledge dev --open
```

### Implementation
- Uses the `opener` crate for cross-platform browser opening
- Handles WSL, sandboxed macOS, and Linux variants automatically
- No platform-specific code needed — single `opener::open(url)` call

## CSS HMR

CSS file changes are handled without full page reloads:

### How It Works
1. File watcher detects CSS file change
2. CSS content is read and included in the HMR WebSocket message
3. Client-side `updatePledgeCSS(path, css)` function:
   - Finds existing `<style data-pledge-path="...">` tag
   - If found: replaces its `textContent` with new CSS
   - If not found: creates a new `<style>` tag and appends to `<head>`
4. No page reload needed — styles update instantly

## Server-Only Hot Reload

When `server_entry` is configured in `pledge.config.ts`, the dev server detects changes to server-only files and triggers a graceful reload while preserving WebSocket connections to connected clients.

### Configuration

In `pledge.config.ts`:
```typescript
export default defineConfig({
  server_entry: 'server/index.ts', // Path to your server entry point
  dev_server: {
    hmr: true,
  },
});
```

### How It Works

1. **Server directory detection**: `compute_server_dirs()` derives server-only directories from the `server_entry` path (e.g., `server/index.ts` → `server/`). Common SSR/API directories (`api/`, `server/`, `src/api/`, `src/server/`, `app/api/`) are also checked.

2. **File classification**: `is_server_file()` checks if a changed file is the server entry file itself or resides in a server-only directory.

3. **HMR update sequence**:
   - `server-reload` message sent to all connected clients with a "Server code changed — reloading..." message
   - Brief 100ms delay to let clients process the notification
   - `server-reload-complete` message sent to signal the server is back

4. **Client-side UI**: A banner appears at the top of the page showing "⟳ Server reloading..." and disappears when the reload completes. WebSocket connections are preserved throughout the reload.

### Client-Side Handler
```javascript
if (data.type === 'server-reload') {
    showPledgeServerReload(data.message); // Shows banner
} else if (data.type === 'server-reload-complete') {
    clearPledgeServerReload(); // Removes banner
}
```

## HTTPS Support

The dev server supports HTTPS via rustls + tokio-rustls:

### Configuration
In `pledge.config.ts`:
```typescript
export default defineConfig({
  https: {
    cert: './cert.pem',
    key: './key.pem',
  },
  dev_server: {
    port: 3000,
  },
});
```

When HTTPS is configured, the dev server serves over TLS, enabling testing of Secure Context APIs (Service Workers, Web Crypto, etc.).

## Import Map Injection

Bare specifiers (e.g., `import React from 'react'`) are resolved via import maps:
- Dep pre-bundler scans source files for bare imports
- Resolves from `node_modules` via `package.json` `exports`/`module`/`main` fields
- Generates import map injected into HTML `<script type="importmap">`
- Pre-bundled deps written to `node_modules/.pledge-deps/`

## Dev Server Proxy

The dev server can proxy API requests to a backend server:

### Configuration
In `pledge.config.ts`:
```typescript
export default defineConfig({
  dev_server: {
    proxy: [
      {
        path: '/api',
        target: 'http://localhost:8080',
        rewrite: true, // Remove /api prefix when forwarding
        ws: true, // Enable WebSocket proxy
        headers: { 'X-Forwarded-Host': 'localhost:3000' }
      }
    ]
  }
});
```

### How It Works
- Requests matching a proxy `path` prefix are forwarded to the `target` URL
- All HTTP methods supported: GET, POST, PUT, DELETE, PATCH
- If `rewrite` is true, the path prefix is stripped (e.g., `/api/users` → `http://localhost:8080/users`)
- If `rewrite` is false, the full path is preserved (e.g., `/api/users` → `http://localhost:8080/api/users`)
- Uses `reqwest` for HTTP forwarding
- Hop-by-hop headers are stripped from the proxy response

### WebSocket Proxy
- Set `ws: true` on proxy config to enable bidirectional WebSocket proxying
- Uses `tokio-tungstenite` for WS bridge between client and target
- Useful for HMR or live-reload backends

## Source Maps in Dev

The dev server appends `sourceMappingURL` comments to transformed modules:
```
//# sourceMappingURL=data:application/json;charset=utf-8;base64,...
```
This enables browser DevTools to show original source code instead of transformed output.

## `pledge serve` — Production Server

Simple static file server for `dist/`:
- **Crate**: `axum` + `tower-http::ServeDir`
- **Port**: 4000 (configurable)
- **Purpose**: Preview production build locally
- **No transforms**: Serves pre-built files as-is

```bash
pledge build   # Build to dist/
pledge serve   # Serve dist/ on :4000
```

## Dev Server Optimizations (Features 9-15)

### Native File Watcher (`crates/core/src/watcher.rs`)
- Platform-specific native watchers for lower latency:
  - **Linux**: `inotify` via `notify` crate
  - **macOS**: `FSEvents` via `notify` crate
  - **Windows**: `ReadDirectoryChangesW` via `notify` crate
- Fallback to polling watcher if native APIs unavailable
- 200ms debounce to batch rapid file changes
- Filters out `node_modules`, `.pledge`, `target`, `.git` directories

### HMR Partial Updates (`crates/dev-server/src/hmr_diff.rs`)
- **Line-level diff**: Uses `similar` crate (Myers algorithm) to compute minimal diff between old and new module content
- **No line limit**: Previous 200-line LCS cap removed — `similar` handles any file size efficiently
- **`is_small()` heuristic**: Only sends diff for small changes, falls back to full replacement for large changes
- **WebSocket transport**: Diff sent via WebSocket as JSON `{ type: "diff", path, additions, deletions }`
- **Reduced bandwidth**: Only changed lines transmitted instead of full module

### Cold Boot Optimization (`crates/core/src/lazy_pipeline.rs`)
- **Deferred initialization**: Oxc parser and Lightning CSS only initialized on first request
- **Dirty dependency tracking**: Only re-transforms modules whose dependencies changed
- **Lazy pipeline**: Transform pipeline components loaded on-demand

### WebSocket Compression
- `tower-http` `CompressionLayer` with gzip and `Fastest` quality level
- Per-message deflate for HMR WebSocket to reduce bandwidth on large module updates

### Multi-Entry Dev Server
- `detect_entries()` auto-detects HTML files in project root
- Each HTML entry gets independent HMR context
- Per-entry routes registered dynamically

### Middleware Chain (`crates/core/src/middleware.rs`)
- Configurable middleware pipeline for request processing
- `MiddlewareFn` parsed from config (auth, logging, headers, CORS, rewrites)
- Middleware executed before module serving
- CORS and rewrite helpers built-in

### On-Demand Dependency Optimization
- Import patterns tracked per-module in `DevServerState`
- Re-optimizes dependencies only when import patterns change
- Not on every server start — faster cold boots

## Network URL Display

The dev server displays the local network URL alongside localhost, so you can test on other devices:

```
  → Local:    http://localhost:3000
  → Network:  http://192.168.1.42:3000
```

- Uses `local-ip-address` crate to detect the machine's network IP
- Shown for both HTTP and HTTPS dev servers
- Useful for testing on mobile devices, other machines, or VMs on the same network

## `pledge dashboard` — Build Telemetry (#101)

The dashboard command serves an interactive web UI for build observability:

```
pledge dashboard [--port 4300]
```

- Serves at `localhost:4300` (configurable via `--port`)
- Reads build history from `.pledge/history.json` (populated during `pledge build`)
- Displays SVG chart with build duration trends and cache hit rates
- Shows recent build summary table with status, duration, module counts
- No build history required to run — shows empty state if no builds recorded

### Build History Records
Each `pledge build` records telemetry data:
```json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "duration_ms": 1234,
  "success": true,
  "modules_built": 42,
  "modules_cached": 18,
  "cache_hit_rate": 0.3,
  "bundle_size": 245678
}
```
