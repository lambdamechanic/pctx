// TypeScript definitions for PCTX Runtime

// ============================================================================
// MCP Client API
// ============================================================================

/**
 * MCP server configuration
 */
export interface MCPServerConfig {
    /** Unique name for the MCP server */
    name: string;
    /** URL of the MCP server */
    url: string;
    /** Optional environment variables */
    env?: Record<string, string>;
    /** Optional command to execute */
    command?: string;
    /** Optional command arguments */
    args?: string[];
}

/**
 * MCP tool call parameters
 */
export interface MCPToolCall {
    /** Name of the registered MCP server */
    name: string;
    /** Name of the tool to call */
    tool: string;
    /** Arguments to pass to the tool */
    arguments?: Record<string, unknown>;
}

/**
 * Register an MCP server
 */
export function registerMCP(config: MCPServerConfig): void;

/**
 * Call an MCP tool
 */
export function callMCPTool<T = unknown>(call: MCPToolCall): Promise<T>;

/**
 * MCP Registry singleton - provides access to registered servers
 */
export const REGISTRY: {
    /** Check if an MCP server is registered */
    has(name: string): boolean;
    /** Get an MCP server configuration */
    get(name: string): MCPServerConfig | undefined;
    /** Delete an MCP server configuration */
    delete(name: string): boolean;
    /** Clear all MCP server configurations */
    clear(): void;
};









// ============================================================================
// Fetch API
// ============================================================================

/**
 * Fetch options
 */
export interface FetchOptions {
    method?: string;
    headers?: Record<string, string>;
    body?: string;
}

/**
 * Fetch response
 */
export interface FetchResponse {
    status: number;
    headers: Record<string, string>;
    body: string;
}

/**
 * Fetch with host-based permissions
 */
export function fetch(url: string, options?: FetchOptions): Promise<FetchResponse>;

/**
 * Call a local tool - UNIFIED API (works for Python, JS, any language!)
 *
 * @template TReturn - The return type of the tool
 * @template TArgs - The argument type for the tool
 * @param name - Name of the tool to call
 * @param args - Arguments to pass to the tool
 * @returns Promise resolving to the tool's result
 *
 * @example
 * // Works for Python tools
 * const result1 = await callLocalTool("python_tool", { value: 42 });
 *
 * // Works for JavaScript tools
 * const result2 = await callLocalTool("js_tool", { value: 42 });
 *
 * // You don't need to know which language the tool is written in!
 */
export function callLocalTool<TReturn = unknown, TArgs = unknown>(
    name: string,
    args?: TArgs
): Promise<TReturn>;

// ============================================================================
// Console Output Capturing
// ============================================================================

declare global {
    var __stdout: string[];
    var __stderr: string[];
    var registerMCP: typeof import('./runtime').registerMCP;
    var callMCPTool: typeof import('./runtime').callMCPTool;
    var REGISTRY: typeof import('./runtime').REGISTRY;
    var callLocalTool: typeof import('./runtime').callLocalTool;
    var fetch: typeof import('./runtime').fetch;
}
