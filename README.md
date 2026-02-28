<div align="center">
  <h1>⚡ mcpify</h1>
  <p><strong>A MCP server framework — built in Rust, runs in Node.js</strong></p>

  <a href="https://www.npmjs.com/package/mcpify"><img src="https://img.shields.io/npm/v/mcpify?style=flat-square&color=cb3837" alt="npm"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue?style=flat-square" alt="MIT License"></a>
  <a href=".github/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/your-org/mcpify/ci.yml?style=flat-square" alt="CI"></a>
  <a href="https://github.com/your-org/mcpify"><img src="https://img.shields.io/badge/built%20with-Rust-orange?style=flat-square" alt="Rust"></a>
</div>

---

**mcpify** makes building [Model Context Protocol (MCP)](https://spec.modelcontextprotocol.io) servers as ergonomic as building HTTP servers with — with a Rust core for maximum throughput.

```js
const { McpServer, textContent } = require('mcpify')

const server = new McpServer({ name: 'my-server', version: '1.0.0' })

server
  .tool('get_weather', {
    description: 'Get current weather for a city',
    inputSchema: {
      type: 'object',
      properties: { city: { type: 'string' } },
      required: ['city'],
    },
  }, async ({ city }) => ({
    content: [textContent(`Weather in ${city}: ☀️ Sunny, 22°C`)]
  }))
  .resource('reports', 'weather://reports/**', {}, async (uri) => ({
    contents: [{ uri, text: 'Report data here' }]
  }))

server.listen({ transport: 'stdio' })
```

---

## Why mcpify?

| Feature | mcpify | Other MCP libs |
|---------|--------|----------------|
| 🦀 Rust core | ✅ Zero-cost protocol parsing | ❌ Pure JS |
| 🔌 Plugin system | ✅ Like `fastify-plugin` | ❌ Manual |
| 🎣 Lifecycle hooks | ✅ preRequest / postResponse / onError | ❌ |
| 🔒 Schema validation | ✅ jsonschema (Rust) | ⚠️ Optional |
| 🌐 Transports | ✅ stdio, HTTP, SSE | ⚠️ Partial |
| 📦 TypeScript | ✅ Full types | ⚠️ Partial |
| 🔗 Chainable API | ✅ Like Fastify | ❌ |

---

## Installation

```bash
npm install mcpify
# or
yarn add mcpify
```

Prebuilt native binaries are provided for:

| Platform | Architecture |
|----------|-------------|
| macOS | x64, ARM64 (Apple Silicon) |
| Linux | x64 (GNU/musl), ARM64 (GNU/musl) |
| Windows | x64, ARM64 |

No Rust compiler needed at install time!

---

## Core Concepts

mcpify maps MCP primitives directly to Fastify-like patterns:

| Fastify | mcpify | MCP Equivalent |
|---------|--------|----------------|
| `fastify.get('/path', handler)` | `server.tool('name', schema, handler)` | Tool |
| `fastify.get('/static/*', handler)` | `server.resource('name', 'uri://**', opts, handler)` | Resource |
| `fastify.register(plugin)` | `server.register(plugin, opts)` | — |
| `fastify.addHook('preHandler', fn)` | `server.addHook('preRequest', fn)` | — |
| `fastify.decorate('key', val)` | `server.decorate('key', val)` | — |
| `fastify.listen({ port })` | `server.listen({ transport })` | — |

---

## API Reference

### `new McpServer(options)`

```js
const server = new McpServer({
  name: 'my-server',          // Required: shown in MCP handshake
  version: '1.0.0',           // Required: semver
  description: 'My server',   // Optional
  logLevel: 'info',           // Optional: error | warn | info | debug
})
```

---

### `server.tool(name, schema, handler)`

Register a tool the AI can call.

```js
server.tool(
  'search_docs',
  {
    description: 'Search the documentation',
    inputSchema: {
      type: 'object',
      properties: {
        query: { type: 'string', description: 'Search query' },
        limit: { type: 'number', default: 10 },
      },
      required: ['query'],
    },
  },
  async ({ query, limit = 10 }) => {
    const results = await db.search(query, limit)
    return {
      content: [
        { type: 'text', text: `Found ${results.length} results:` },
        { type: 'text', text: results.map(r => r.title).join('\n') },
      ],
    }
  }
)
```

**Handler return type:**
```ts
{
  content: McpContent[]
  isError?: boolean
}
```

---

### `server.resource(name, uriPattern, schema, handler)`

Register a resource the AI can read. URI patterns support wildcards:

```js
// Match all files under /docs
server.resource('docs', 'file:///docs/**', { mimeType: 'text/markdown' }, async (uri) => {
  const content = await fs.readFile(uri.replace('file://', ''), 'utf8')
  return {
    contents: [{ uri, mimeType: 'text/markdown', text: content }],
  }
})

// Match single-segment patterns
server.resource('user', 'db://users/*', {}, async (uri) => {
  const id = uri.replace('db://users/', '')
  const user = await db.users.findById(id)
  return { contents: [{ uri, text: JSON.stringify(user) }] }
})
```

---

### `server.prompt(name, schema, handler)`

Register a reusable prompt template:

```js
server.prompt(
  'code_review',
  {
    description: 'Generate a code review prompt',
    arguments: [
      { name: 'language', description: 'Programming language', required: true },
      { name: 'focus',    description: 'Review focus area', required: false },
    ],
  },
  async ({ language, focus = 'general' }) => ({
    messages: [{
      role: 'user',
      content: {
        type: 'text',
        text: `Please review this ${language} code with a focus on ${focus}.`,
      },
    }],
  })
)
```

---

### `server.register(plugin, options?)` — Plugin System

```js
// Define a reusable plugin
const { definePlugin } = require('mcpify')

const cachePlugin = definePlugin('cache', async (server, opts) => {
  const cache = new Map()
  server.decorate('cache', cache)

  server.addHook('preRequest', async (ctx) => {
    server.log.debug(`Request ${ctx.requestId} starting`)
  })
})

// Register it
await server.register(cachePlugin, { ttl: 60 })

// Now server.cache is available
server.tool('cached_data', {}, async (params) => {
  if (server.cache.has(params.key)) {
    return { content: [textContent(server.cache.get(params.key))] }
  }
  // ...fetch and cache
})
```

---

### `server.addHook(event, handler)`

Lifecycle hooks run around every request:

```js
// Before request processing
server.addHook('preRequest', async (ctx) => {
  console.log(`→ ${ctx.method} [${ctx.requestId}]`)
})

// After response is sent
server.addHook('postResponse', async (ctx) => {
  console.log(`← ${ctx.method} done`)
})

// On any error
server.addHook('onError', async (ctx) => {
  console.error(`✗ Error in ${ctx.method}`)
})
```

---

### `server.decorate(key, value)`

Extend the server with custom properties (like decorators):

```js
server.decorate('db', new Database())
server.decorate('config', { apiKey: process.env.API_KEY })

// Access on the server instance
server.tool('query', {}, async (params) => {
  const data = await server.db.query(params.sql)
  // ...
})
```

---

### `server.listen(options)`

Start the server:

```js
// stdio (for Claude Desktop, most MCP clients)
await server.listen({ transport: 'stdio' })

// HTTP (for web-based clients)
await server.listen({
  transport: 'http',
  host: '0.0.0.0',
  port: 3000,
  path: '/mcp',
})
```

---

### Content Helpers

```js
const { textContent, imageContent, resourceContent } = require('mcpify')

// Text
textContent('Hello, world!')

// Image (base64)
imageContent(imageBase64, 'image/png')

// Resource reference
resourceContent('file:///path/to/file', 'text/plain')
```

---

### Schema Validation (Rust-powered)

```js
const { validateSchema } = require('mcpify')

const error = validateSchema(
  JSON.stringify({ type: 'object', properties: { name: { type: 'string' } } }),
  JSON.stringify({ name: 123 })  // wrong type!
)
// error => "123 is not of type 'string'"
```

---

## Claude Desktop Integration

Add to `~/.config/claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "my-server": {
      "command": "node",
      "args": ["/path/to/your-server.js"],
      "env": {
        "API_KEY": "your-key"
      }
    }
  }
}
```

---

## Building from Source

```bash
# Prerequisites: Rust + @napi-rs/cli
npm install -g @napi-rs/cli
cargo install cargo-watch

# Install deps
yarn install

# Build (release)
yarn build

# Build (debug, faster)
yarn build:debug

# Watch for Rust changes
cargo watch -s "yarn build:debug"
```

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

---

## License

[MIT](LICENSE) — © 2025 mcpify contributors
