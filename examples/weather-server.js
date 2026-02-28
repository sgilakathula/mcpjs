/**
 * examples/weather-server.js
 *
 * A minimal MCP weather server — shows the Fastify-inspired API.
 * Run with: node weather-server.js
 * Then connect via Claude Desktop or any MCP client using stdio transport.
 */

const { McpServer, textContent, imageContent, definePlugin } = require('./index.js')

// ─── Create server ────────────────────────────────────────────────────────────
const server = new McpServer({
  name: 'weather-server',
  version: '1.0.0',
  description: 'Demo MCP weather server built with mcpify',
  logLevel: 'info',
})

// ─── Plugin: Auth (example) ───────────────────────────────────────────────────
const authPlugin = definePlugin('auth', async (server, opts) => {
  server.decorate('apiKey', opts.apiKey)

  server.addHook('pre', async (ctx) => {
    server.log.debug(`[auth] Checking request ${ctx.requestId}`)
  })
})

// ─── Register plugins ─────────────────────────────────────────────────────────
server.register(authPlugin, { apiKey: process.env.API_KEY || 'demo-key' })

// ─── Register tools ───────────────────────────────────────────────────────────
server.tool(
  'get_current_weather',
  {
    description: 'Get the current weather for a city',
    inputSchema: {
      type: 'object',
      properties: {
        city: {
          type: 'string',
          description: 'City name, e.g. "London" or "Tokyo"',
        },
        units: {
          type: 'string',
          enum: ['celsius', 'fahrenheit'],
          description: 'Temperature unit (default: celsius)',
        },
      },
      required: ['city'],
    },
  },
  async ({ city, units = 'celsius' }) => {
    // In a real server, you'd call a weather API here
    const temp = units === 'fahrenheit' ? '72°F' : '22°C'
    return {
      content: [
        textContent(`Current weather in ${city}: ☀️ Sunny, ${temp}, humidity 65%`),
      ],
    }
  }
)

server.tool(
  'get_forecast',
  {
    description: 'Get a 5-day weather forecast',
    inputSchema: {
      type: 'object',
      properties: {
        city:    { type: 'string' },
        days:    { type: 'number', minimum: 1, maximum: 7 },
      },
      required: ['city'],
    },
  },
  async ({ city, days = 5 }) => {
    const forecast = Array.from({ length: days }, (_, i) => {
      const date = new Date()
      date.setDate(date.getDate() + i + 1)
      return `${date.toDateString()}: 🌤 Partly cloudy, 19°C`
    })

    return {
      content: [
        textContent(`${days}-day forecast for ${city}:\n${forecast.join('\n')}`),
      ],
    }
  }
)

// ─── Register resources ───────────────────────────────────────────────────────
server.resource(
  'weather-report',
  'weather://reports/**',
  { description: 'Historical weather reports', mimeType: 'text/plain' },
  async (uri) => {
    const city = uri.replace('weather://reports/', '')
    return {
      contents: [{
        uri,
        mimeType: 'text/plain',
        text: `Historical weather data for ${city}\n---\nDate: 2025-01-01, Temp: 18°C, Sunny`,
      }],
    }
  }
)

// ─── Register prompts ─────────────────────────────────────────────────────────
server.prompt(
  'weather-summary',
  {
    description: 'Generate a weather summary for a city',
    arguments: [
      { name: 'city', description: 'City to summarize', required: true },
      { name: 'style', description: 'Summary style: brief | detailed', required: false },
    ],
  },
  async ({ city, style = 'brief' }) => ({
    messages: [{
      role: 'user',
      content: {
        type: 'text',
        text: style === 'detailed'
          ? `Please provide a detailed weather summary for ${city}, including current conditions, forecast, and any weather alerts.`
          : `Give me a brief weather update for ${city} in 2 sentences.`,
      },
    }],
  })
)

// ─── Start server ─────────────────────────────────────────────────────────────
server.listen({ transport: 'stdio' })
  .then(() => {
    server.log.info(`Registered tools: ${server.listTools().join(', ')}`)
    server.log.info(`Registered resources: ${server.listResources().join(', ')}`)
    server.log.info(`Registered prompts: ${server.listPrompts().join(', ')}`)
  })
  .catch((err) => {
    console.error('Failed to start server:', err)
    process.exit(1)
  })
