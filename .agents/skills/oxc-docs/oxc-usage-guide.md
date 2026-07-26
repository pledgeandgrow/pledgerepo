# Oxc Tool Usage Guide

Practical guide for using each Oxc tool — parser, linter, formatter, transformer,
minifier, and resolver. Covers both Node.js and Rust installation.

---

## Parser

### Features
- 3x faster than swc parser
- Parses `.js(x)` and `.ts(x)`
- Passes all Test262 parser tests, 99%+ Babel and TypeScript
- Returns ESM information directly (no need for `es-module-lexer`)

### Installation

#### Node.js
```bash
npm install oxc-parser
```

```js
import { parseSync } from "oxc-parser";

const { program } = parseSync("test.js", 'alert("hello");');
```

#### Rust
```toml
[dependencies]
oxc = "latest"
# or individual crates:
oxc_parser = "latest"
oxc_ast = "latest"
```

```rust
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;

let allocator = Allocator::default();
let source_text = "const x = 1;";
let source_type = SourceType::default();
let result = Parser::new(&allocator, source_text, source_type).parse();
```

Rust example: https://github.com/oxc-project/oxc/blob/main/crates/oxc_parser/examples/parser.rs

### Printing (Parse in Reverse)

Use `esrap` to print code from an AST:

```js
import { print } from "esrap";
import ts from "esrap/languages/ts";
import { parseSync } from "oxc-parser";

const { program } = parseSync("test.js", 'alert("hello oxc & esrap");');
const { code } = print(program, ts());
console.log(code); // alert("hello oxc & esrap");
```

---

## Linter (Oxlint)

### Overview
- 50-100x faster than ESLint
- 840+ rules with ESLint, TypeScript, React, Jest, Vitest, Import, Unicorn, jsx-a11y coverage
- Type-aware linting via tsgo (TypeScript 7 Go port)
- Multi-file analysis with project-wide module graph
- Correctness-focused defaults (high-signal, low-noise)
- Custom JS plugins compatible with ESLint plugin ecosystem

### Getting Started

```bash
pnpm add -D oxlint
```

Add to `package.json`:
```json
{
  "scripts": {
    "lint": "oxlint",
    "lint:fix": "oxlint --fix"
  }
}
```

### Supported File Types
- JavaScript/TypeScript: `.js`, `.mjs`, `.cjs`, `.ts`, `.mts`, `.cts`
- JSX/TSX: `.jsx`, `.tsx`
- Framework: `.vue`, `.svelte`, `.astro` (lints `<script>` blocks only)

### Migration from ESLint

**Option 1: Replace ESLint (recommended)**
Use `@oxlint/migrate` to convert your ESLint config:
```bash
npx @oxlint/migrate
```

**Option 2: Incremental migration**
Run Oxlint first, then ESLint with overlapping rules disabled:
```bash
npm install eslint-plugin-oxlint
```
This plugin automatically disables ESLint rules that Oxlint already covers.

### Configuration

See: https://oxc.rs/docs/guide/usage/linter/config.html

### Type-Aware Linting

Oxlint leverages tsgo (native Go port of TypeScript compiler) for full TypeScript compatibility. Enables checks like floating promise detection.

See: https://oxc.rs/docs/guide/usage/linter/type-aware.html

### Multi-File Analysis

Builds a project-wide module graph, shares parsing and resolution across rules. Improves cross-file import checks (e.g., `import/no-cycle`).

See: https://oxc.rs/docs/guide/usage/linter/multi-file-analysis.html

### Reliability
- Crashes treated as top priority bugs
- Performance regressions treated as bugs
- Stability prioritized for CI and large monorepos

---

## Formatter (Oxfmt)

Prettier-compatible formatting. Targets the fastest formatter for JavaScript and TypeScript.

See: https://oxc.rs/docs/guide/usage/formatter.html

---

## Transformer

### Features (runs in fixed order)
1. **React Compiler** — runs first on original source
2. **TypeScript** — type stripping
3. **Decorators** — experimental decorators
4. **Plugins** — e.g., styled-components
5. **React Refresh** — component instrumentation
6. **JSX** — JSX to JavaScript
7. **Lowering** — ES2026 down to ES2015
8. **Inject** — global variable injection
9. **Define** — global variable replacement

Also supports **TypeScript Isolated Declarations** emit without the TypeScript compiler.

### Installation

#### Node.js
```bash
npm install oxc-transform
```

```js
import { transform } from "oxc-transform";

const result = await transform("lib.ts", sourceCode, {
  lang: "tsx",              // "js" | "jsx" | "ts" | "tsx" | "dts"
  sourceType: "module",     // "script" | "module" | "commonjs" | "unambiguous"
  cwd: "/path/to/project",
  sourcemap: true,
  helpers: {
    mode: "Runtime",        // "Runtime" (@oxc-project/runtime) or "External" (babelHelpers)
  },
  // typescript, jsx, target, assumptions, define, inject, decorator, plugins
});

// Synchronous variant also available:
import { transformSync } from "oxc-transform";
const result = transformSync("lib.ts", sourceCode, { /* options */ });
```

#### Rust
```toml
[dependencies]
oxc = { version = "latest", features = ["transformer"] }
```

Rust example: https://github.com/oxc-project/oxc/blob/main/crates/oxc_transformer/examples/transformer.rs

### Integrations
- `unplugin-oxc` — universal plugin for transform/minify
- `unplugin-isolated-decl` — isolated declarations plugin
- `oxc-webpack-loader` — webpack loader

---

## Minifier

### Features
- Dead code elimination
- Syntax normalization (shorter output)
- Variable name mangling
- Whitespace and comment removal

### Assumptions
Makes assumptions about code for better optimizations. See: [ASSUMPTIONS.md](https://github.com/oxc-project/oxc/blob/main/crates/oxc_minifier/docs/ASSUMPTIONS.md)

### Installation

#### With Rolldown
Oxc-minify is used by default for minification in Rolldown. No extra installation needed.

#### Node.js
```bash
npm install oxc-minify
```

#### Rust
```toml
[dependencies]
oxc = { version = "latest", features = ["minifier"] }
```

Rust example: https://github.com/oxc-project/oxc/blob/main/crates/oxc_minifier/examples/minifier.rs

---

## Resolver

### Features
- All configurations aligned with webpack/enhanced-resolve
- 28x faster than enhanced-resolve
- See: https://github.com/oxc-project/oxc-resolver

### Installation

#### Node.js
```bash
npm install oxc-resolver
```

#### Rust
```toml
[dependencies]
oxc_resolver = "latest"
```

Docs: https://docs.rs/oxc_resolver

---

## Who Oxc is For

- **App/library developers** — fastest lint and format loop locally and in CI
- **Toolchain/platform teams** — fast compiler-grade foundation at scale
- **Tool authors** — fast reusable Rust crates or npm packages for JS tooling
