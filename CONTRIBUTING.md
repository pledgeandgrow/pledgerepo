# Contributing to Pledge

Thank you for your interest in contributing to Pledge! This document outlines the process for contributing to the project.

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable, edition 2024)
- [Zig](https://ziglang.org/) (0.14.0+)
- [Node.js](https://nodejs.org/) (>=18)

### Setup

```bash
# Clone the repository
git clone https://github.com/pledgeandgrow/pledgerepo.git
cd pledgerepo

# Build the project
.\build.ps1        # Windows
# or
zig build -Doptimize=ReleaseFast && cargo build --release  # Manual

# Run tests
cargo test

# Run benchmarks
.\build.ps1 bench
```

## Development Workflow

### 1. Create a Branch

```bash
git checkout -b feat/your-feature-name
```

Use the following prefixes:
- `feat/` — New features
- `fix/` — Bug fixes
- `docs/` — Documentation changes
- `refactor/` — Code refactoring
- `test/` — Test additions or fixes
- `chore/` — Build, CI, or tooling changes

### 2. Make Your Changes

- Follow the existing code style and patterns
- Add tests for new functionality
- Update documentation as needed
- Ensure `cargo test` passes
- Ensure `cargo clippy` passes without warnings

### 3. Commit Your Changes

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add support for Preact adapter
fix: resolve import path edge case with trailing slash
docs: update ARCHITECTURE.md with polyfills module
refactor: simplify transform pipeline error handling
test: add unit tests for CSS module hash generation
chore: update oxc dependency to latest version
```

### 4. Push and Create a Pull Request

```bash
git push origin feat/your-feature-name
```

Then create a pull request on GitHub with:
- A clear title following conventional commit format
- A description of what changed and why
- Any breaking changes noted
- Links to related issues

## Code Style

### Rust

- Follow `rustfmt` defaults (run `cargo fmt`)
- Follow `clippy` recommendations (run `cargo clippy`)
- Use `anyhow::Result` for error handling in application code
- Use `thiserror` for library error types
- Prefer `tracing` over `log` for structured logging
- Document public APIs with `///` doc comments

### Zig

- Follow the [Zig Style Guide](https://ziglang.org/documentation/master/#Style-Guide)
- Use snake_case for functions and variables
- Use TitleCase for types
- Document public functions with `///` comments

### TypeScript/JavaScript

- Use TypeScript for all new code
- Follow the existing ESLint configuration
- Use ESM (`import`/`export`) syntax

## Project Structure

```
crates/
├── cli/              # CLI entry point (pledgepack-cli)
├── core/             # Core engine, config, transform, pipeline
├── cache/            # Function-level incremental cache
├── resolver/         # Module resolution
├── dev-server/       # Dev server + HMR
├── optimizer/        # Tree shaking, code splitting
├── plugin-host/      # WASM plugin system
├── js-plugin-host/   # JS plugin system (boa_engine)
├── adapter-react/    # React adapter
├── adapter-solid/    # Solid.js adapter
├── adapter-next/     # Next.js adapter
└── adapter-tanstack/ # TanStack Router adapter
native-sys/           # Zig FFI bindings
docs/                 # Documentation
```

## Testing

```bash
# Run all Rust tests
cargo test

# Run tests for a specific crate
cargo test -p pledgepack-core

# Run tests with output
cargo test -- --nocapture

# Run benchmarks
cargo bench
```

## Reporting Issues

When reporting issues, please include:
- Pledge version (`pledge --version`)
- Operating system
- Rust version (`rustc --version`)
- Zig version (`zig version`)
- Minimal reproduction case
- Expected vs actual behavior

## License

By contributing, you agree that your contributions will be licensed under the Mozilla Public License, Version 2.0.

## Questions?

Feel free to open a discussion on GitHub or reach out to the maintainers.
