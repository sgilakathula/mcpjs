# Contributing to mcpify

Thank you for your interest in contributing! This guide will help you get started.

## Prerequisites

- **Rust** ≥ 1.75 — [rustup.rs](https://rustup.rs)
- **Node.js** ≥ 18 — [nodejs.org](https://nodejs.org)
- **yarn** — `npm install -g yarn`
- **@napi-rs/cli** — `npm install -g @napi-rs/cli`

## Development Setup

```bash
# Clone the repo
git clone https://github.com/your-org/mcpify
cd mcpify

# Install Node.js dependencies
yarn install

# Build the native module (debug mode for faster iteration)
yarn build:debug

# Run tests
yarn test
```

## Project Structure

```
mcpify/
├── src/
│   └── lib.rs          # Rust core: MCP protocol + napi bindings
├── __test__/
│   └── mcpify.test.js  # Node.js tests
├── examples/
│   ├── weather-server.js  # stdio transport example
│   └── http-server.js     # HTTP transport example
├── .github/
│   └── workflows/
│       └── ci.yml         # Build + test + publish pipeline
├── index.js               # JavaScript wrapper + plugin system
├── index.d.ts             # TypeScript definitions
├── Cargo.toml             # Rust manifest
├── package.json           # npm manifest
└── build.rs               # napi-rs build script
```

## Workflow

### Adding a new MCP capability

1. **Define the type** in `src/lib.rs` (with `#[napi(object)]`)
2. **Implement the handler** in `McpServer` (with `#[napi]`)
3. **Expose it** through the JS wrapper in `index.js`
4. **Add TypeScript types** in `index.d.ts`
5. **Write tests** in `__test__/`

### Making a release

```bash
# Bump version in package.json and Cargo.toml
npm version patch   # or minor, major

# Push tag — CI will build all platforms and publish to npm
git push origin main --tags
```

## Code Style

- **Rust**: `cargo fmt` + `cargo clippy`
- **JavaScript**: Standard formatting (no semicolons, 2-space indent)

## Commit Convention

We use [Conventional Commits](https://www.conventionalcommits.org/):

- `feat: add SSE transport`
- `fix: handle empty params in tools/call`
- `docs: update README plugin section`
- `chore: update napi-rs to 2.18`

## Reporting Issues

Please include:
- OS and architecture (`node -p "process.platform + '-' + process.arch"`)
- Node.js version (`node --version`)
- Rust version (`rustc --version`)
- Minimal reproduction case
