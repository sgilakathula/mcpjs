/**
 * examples/http-server.js
 *
 * MCP server running over HTTP — useful for web-based MCP clients.
 * Exposes a POST /mcp endpoint and GET /health.
 *
 * Run: node http-server.js
 */

const { McpServer, textContent } = require('./index.js')

const server = new McpServer({
  name: 'http-mcp-server',
  version: '1.0.0',
  description: 'MCP server over HTTP',
})

server.tool(
  'echo',
  {
    description: 'Echo a message back',
    inputSchema: {
      type: 'object',
      properties: { message: { type: 'string' } },
      required: ['message'],
    },
  },
  async ({ message }) => ({
    content: [textContent(`Echo: ${message}`)],
  })
)

server.tool(
  'add_numbers',
  {
    description: 'Add two numbers',
    inputSchema: {
      type: 'object',
      properties: {
        a: { type: 'number' },
        b: { type: 'number' },
      },
      required: ['a', 'b'],
    },
  },
  async ({ a, b }) => ({
    content: [textContent(`${a} + ${b} = ${a + b}`)],
  })
)

server.listen({
  transport: 'http',
  host: '0.0.0.0',
  port: 3000,
  path: '/mcp',
})
  .then(() => {
    server.log.info('Server ready. Test with:')
    server.log.info('  curl -X POST http://localhost:3000/mcp \\')
    server.log.info('    -H "Content-Type: application/json" \\')
    server.log.info('    -d \'{"jsonrpc":"2.0","id":"1","method":"tools/list"}\'')
  })
  .catch((err) => {
    console.error('Failed to start server:', err)
    process.exit(1)
  })
