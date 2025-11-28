# @pctx/sdk

Node.js/TypeScript SDK for PCTX - A complete toolkit for working with MCP servers and local tools.

## Installation

```bash
npm install @pctx/sdk
```

## Features

- **MCP Server Integration**: Connect to any Model Context Protocol server
- **Local Tools**: Register JavaScript/TypeScript functions as tools
- **Function Discovery**: List and inspect all available functions
- **Code Execution**: Execute TypeScript code with full tool access
- **Type-Safe**: Full TypeScript definitions included
- **High Performance**: Native Rust implementation via N-API

## Quick Start

```typescript
import { PctxTools } from '@pctx/sdk';

const tools = new PctxTools();

// Register an MCP server
await tools.addMcpServer({
  name: 'github',
  command: 'npx',
  args: ['-y', '@modelcontextprotocol/server-github'],
  env: { GITHUB_TOKEN: process.env.GITHUB_TOKEN }
});

// Register a local tool
tools.registerLocalTool({
  name: 'getCurrentTime',
  description: 'Gets the current ISO timestamp',
  namespace: 'utils'
}, () => new Date().toISOString());

// List all available functions
const { functions } = await tools.listFunctions();
console.log(functions.map(f => `${f.namespace}.${f.name}`));

// Execute TypeScript code with tool access
const result = await tools.execute({
  code: `
    async function run() {
      const time = await utils.getCurrentTime();
      return { message: 'Current time is ' + time };
    }
  `
});
console.log(result.output);
```

## API Reference

### `PctxTools`

Main class for interacting with PCTX.

#### Methods

##### `addMcpServer(config: McpServerConfig): Promise<void>`

Register an MCP server.

```typescript
await tools.addMcpServer({
  name: 'github',
  command: 'npx',
  args: ['-y', '@modelcontextprotocol/server-github'],
  env: { GITHUB_TOKEN: process.env.GITHUB_TOKEN }
});
```

##### `registerLocalTool(options: LocalToolOptions, handler: Function): void`

Register a local JavaScript/TypeScript function as a tool.

```typescript
tools.registerLocalTool({
  name: 'add',
  description: 'Adds two numbers',
  namespace: 'math',
  inputSchema: {
    type: 'object',
    properties: {
      a: { type: 'number' },
      b: { type: 'number' }
    }
  }
}, ({ a, b }) => a + b);
```

##### `listFunctions(): Promise<ListFunctionsOutput>`

List all available functions from MCP servers and local tools.

```typescript
const { functions, code } = await tools.listFunctions();
// functions: Array of { namespace, name, description }
// code: TypeScript import code
```

##### `getFunctionDetails(input: { functions: string[] }): Promise<GetFunctionDetailsOutput>`

Get detailed type information about specific functions.

```typescript
const details = await tools.getFunctionDetails({
  functions: ['github.createIssue', 'math.add']
});
// Returns full TypeScript type definitions
```

##### `execute(input: { code: string }): Promise<ExecuteOutput>`

Execute TypeScript code with access to all registered tools.

```typescript
const result = await tools.execute({
  code: `
    async function run() {
      const sum = await math.add({ a: 5, b: 3 });
      return { result: sum };
    }
  `
});
console.log(result.output); // { result: 8 }
```

## License

MIT
