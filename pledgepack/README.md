# PledgePack

A Rust+Zig bundler with incremental computation, JS plugins, and Rollup-quality output. Like Vite, Webpack, or Turbopack — but faster.

> **npm package:** `pledgepack` · **CLI command:** `pledge`

## What It Is

PledgePack is **not** a framework. It's the build tool and dev server that frameworks and apps run on top of. It handles:

- **Dev server** — Rust-based with native HMR (Hot Module Replacement)
- **Bundling** — Powered by Oxc parser and Lightning CSS
- **JSX/TSX transformation** — Native Rust compilation, no Babel or SWC
- **File-based routing** — Automatic route detection from `app/` directory
- **Import maps** — Automatic bare specifier resolution for `node_modules`
- **CJS to ESM conversion** — Browser-compatible module serving
- **CSS pipeline** — Lightning CSS for optimization and preprocessing
- **Build output** — Optimized production bundles to `.pledge/` directory
- **Framework support** — React, Vue, Svelte, Solid, Astro, Next.js, TanStack
- **JS plugins** — Vite-compatible plugin system powered by Boa engine
- **Built-in test runner** — Vitest-compatible API with snapshot testing and coverage
- **Bundle analyzer** — Interactive HTML treemap visualization
- **Edge-ready output** — Cloudflare Workers, Vercel Edge, Deno Deploy
- **Library mode** — ESM, CJS, IIFE, UMD multi-format output

## Install

```bash
# Global install (CLI usage)
npm install -g pledgepack

# Or as a dev dependency in your project
npm install --save-dev pledgepack
```

The postinstall script automatically downloads the prebuilt native binary for your platform from GitHub Releases. Supported platforms:

- **Linux** x64/arm64
- **macOS** x64/arm64 (Intel + Apple Silicon)
- **Windows** x64/arm64

If no prebuilt binary is available, it falls back to building from source (requires Rust + Zig).

## Usage

### CLI

```bash
# Start dev server with HMR
pledge dev

# Build for production
pledge build

# Build with watch mode
pledge build --watch

# Build with profiling
pledge build --profile

# Preview production build
pledge serve

# Scaffold a new project
pledge create react my-app
pledge create vue my-app
pledge create svelte my-app
pledge create solid my-app
pledge create next my-app
pledge create tanstack my-app
pledge create vanilla my-app

# Add pledgepack to existing project (migrates from Vite/webpack/CRA)
pledge init

# Migrate config from Vite/webpack/Turbopack
pledge migrate

# Diagnose build issues
pledge doctor

# Analyze bundle size
pledge analyze

# Run tests (Vitest-compatible)
pledge test
pledge test --watch
pledge test --ui

# Run benchmarks
pledge bench

# Clear cache
pledge cache clear
pledge cache stats

# Generate env type declarations
pledge generate-env-types

# Generate shell completions
pledge completions --shell bash
```

### Programmatic API

```js
import { runPledgepack, getBinaryPath } from 'pledgepack';

// Run a build
await runPledgepack(['build']);

// Start dev server
await runPledgepack(['dev', '--port', '3000']);

// Get binary path
console.log(getBinaryPath());
```

### npm Scripts (in your project's package.json)

```json
{
  "scripts": {
    "dev": "pledge dev",
    "build": "pledge build",
    "preview": "pledge preview",
    "serve": "pledge serve",
    "test": "pledge test",
    "bench": "pledge bench",
    "analyze": "pledge analyze"
  }
}
```

## Configuration

### `pledge.config.ts`

```typescript
import { defineConfig } from 'pledge';

export default defineConfig({
  entry: ['src/index.tsx'],
  framework: 'react',
  source_maps: true,
  env_prefix: 'PLEDGE_',
  env_dts: true,
  html_entry: 'index.html',
  compress_gzip: true,
  compress_brotli: true,
  edge_target: 'cloudflare',
  plugins: ['./plugins/my-plugin.ts'],
  image: {
    quality: 80,
    webp: true,
    avif: false,
    max_width: 1920,
    max_height: 1080,
  },
  library: {
    entry: 'src/index.ts',
    formats: ['esm', 'cjs'],
    name: 'MyLib',
    external: ['react'],
    declarations: true,
  },
  https: {
    cert: './cert.pem',
    key: './key.pem',
  },
  node_polyfills: true,
  define: {
    'process.env.NODE_ID': '"production"',
    '__VERSION__': '"1.0.0"',
  },
  dev_server: {
    port: 3000,
    host: 'localhost',
    hmr: true,
    open: false,
    proxy: [
      { path: '/api', target: 'http://localhost:8080', rewrite: true, ws: true }
    ],
  },
  test: {
    include: ['**/*.{test,spec}.{ts,tsx,js,jsx}'],
    exclude: ['node_modules', '.pledge', 'dist'],
    environment: 'node',
    globals: false,
    coverage: false,
    snapshot: true,
  },
});
```

Config file resolution order: `pledge.config.ts` -> `pledge.config.js` -> `pledge.config.mjs` -> `pledge.json` -> defaults.

### `.env` Files

Pledge loads environment variables from `.env` files with the following precedence:

1. `.env.[mode].local`
2. `.env.[mode]`
3. `.env.local`
4. `.env`

Variables are injected via `import.meta.env.*`:

```typescript
const apiUrl = import.meta.env.PLEDGE_API_URL;
const isDev = import.meta.env.PLEDGE_DEV;
```

## Supported Frameworks

| Framework | Status | File Types |
|-----------|--------|------------|
| **React** | Full | `.tsx`, `.jsx`, Fast Refresh, classic JSX |
| **Solid** | Full | `.tsx`, `.jsx`, automatic JSX with `solid-js` |
| **Vue** | Full | `.vue` (SFC), scoped CSS, script setup |
| **Svelte** | Full | `.svelte` (SFC), scoped CSS, render functions |
| **Astro** | Full | `.astro`, frontmatter, islands-ready |
| **Next.js** | Adapter | App Router, Pages Router, API routes, SSR |
| **TanStack** | Adapter | File-based routing, route tree generation |
| **Vanilla TS/JS** | Full | `.ts`, `.js`, `.mjs` |

## Features

### Build and Transform
- Oxc-based JSX/TSX transformation (no Babel, no SWC)
- TypeScript type stripping
- Production minification (dead code elimination, variable mangling, constant folding)
- Source maps (v3) in dev and production
- CSS processing via Lightning CSS (minification, nesting, autoprefixing)
- CSS Modules with scoped class names
- PostCSS / Tailwind directive expansion
- Tree shaking with side-effects detection
- Cross-chunk variable hoisting
- CSS tree shaking (remove unused selectors by analyzing JS class names)
- Dead code elimination at expression level
- Constant folding with type info
- Optional chaining nullish short-circuit optimization
- Module-level memoization with content + config hash keys

### CSS and Styling
- Tailwind v4 Oxide engine integration (`@theme`, `@utility`, `@variant`)
- CSS-in-JS compile-time extraction (styled-components, emotion, vanilla-extract)
- CSS `@layer` cascade layer management and ordering
- Container queries polyfill for older browsers
- Critical CSS extraction and inlining for faster FCP
- CSS source maps pointing to original `.scss`/`.less`/`.css` files
- PostCSS plugin caching with blake3 content hashing

### Asset Pipeline
- MDX compilation (Markdown + JSX with frontmatter)
- GraphQL file loading with TypeScript type generation
- YAML/CSV/TSV imports with typed named exports
- Image format auto-selection (WebP/AVIF based on browser support)
- Audio/video asset handling with URL exports
- PDF asset handling with inline base64 support
- Asset manifest generation with hashed output paths

### Plugin System
- Plugin hot reload (reload JS plugins without restarting dev server)
- JS plugin sandboxing (memory limits, CPU time, filesystem access control)
- Plugin dependency resolution (npm packages via pre-bundled imports)
- Plugin lifecycle hooks (`watchStart`, `watchChange`, `watchEnd`, before/after transform/build)
- Plugin parallel execution via rayon

### Output and Distribution
- Service worker generation with precaching and runtime caching
- Web App Manifest generation
- Performance budget enforcement (per-entry, per-chunk size limits)
- Bundle size diff with regression detection
- Source map explorer with interactive HTML treemap
- Multi-format output (ESM, CJS, IIFE, UMD) for library mode
- Gzip + Brotli compression output
- Edge-ready output (Cloudflare Workers, Vercel Edge, Deno Deploy)

### DX and Tooling
- LSP server (import resolution, go-to-definition, diagnostics, hover, document symbols)
- Migration tooling (Vite/webpack/Turbopack to `pledge.config.ts`)
- Built-in test runner (Vitest-compatible API with mocks, snapshots, coverage)
- Bundle analyzer with interactive HTML report
- Build profiling (per-phase timing)
- Project scaffolding (`pledge create`)
- Config validation and diagnostics (`pledge doctor`)
- Shell completion generation (bash, zsh, fish, powershell)

### Dev Server
- Rust-based with native HMR
- On-demand transforms (only transforms requested modules)
- File watcher with platform-native APIs (inotify/FSEvents/ReadDirectoryChangesW)
- HMR partial updates (line-level diff via WebSocket)
- Cold boot optimization (lazy-load transform pipeline)
- WebSocket compression for HMR
- Multi-entry dev server with independent HMR contexts
- Configurable middleware chain
- Error overlay with source context and stack traces
- CSS HMR (style updates without page reload)
- HTTPS support
- Dev server proxy (HTTP + WebSocket)
- React Fast Refresh (component state preservation)

### Performance
- Incremental rebuild graph (only rebuild changed modules)
- Persistent module graph (serialized to disk between builds)
- Parallel dependency optimization (rayon)
- Lazy dependency scanning
- Build cache sharing (S3/GCS remote cache)
- Git-based cache invalidation
- Memory-mapped output writing
- Two-tier cache (memory + disk with bincode)

## Binary Resolution

The native binary is resolved in this order:

1. `target/release/pledge` — built from source (dev mode)
2. `target/debug/pledge` — built from source (dev mode)
3. `bin/{platform-arch}/pledge` — downloaded by postinstall
4. `bin/platform/{platform-arch}/pledge` — staged by CI
5. `bin/pledge` — direct binary (global install)

If no binary is found, the postinstall script downloads a prebuilt binary from GitHub Releases, or falls back to building from source if Rust is installed.

## How It Compares

| Feature | Vite | Webpack | Turbopack | PledgePack |
|---------|------|---------|-----------|------------|
| Language | JS/Go | JS | Rust | **Rust + Zig** |
| Parser | esbuild | acorn | SWC | **Oxc** |
| CSS | PostCSS | CSS loader | SWC | **Lightning CSS** |
| Dev server | Vite | webpack-dev-server | Turbopack | **PledgePack** |
| HMR | Partial | Partial | Partial | **Line-level diff** |
| Plugin system | JS (V8) | JS | None | **JS (Boa engine)** |
| Framework | Any | Any | Next.js only | **Any** |
| Bundle size | Smallest | Variable | +72% bloat | **Small** |
| Incremental | Module-level | Module-level | Function-level | **Function-level** |
| Test runner | Vitest | Jest | None | **Built-in** |
| Edge output | Plugin | Plugin | None | **Built-in** |

## License

MIT