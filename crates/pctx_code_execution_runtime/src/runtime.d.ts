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
// JS Local Tool API (JavaScript Callbacks)
// ============================================================================

/**
 * JS local tool metadata and configuration
 */
export interface JsLocalToolConfig {
    /** Unique name for the tool */
    name: string;
    /** Tool description */
    description?: string;
    /** JSON Schema for tool input parameters */
    inputSchema?: {
        type: string;
        properties?: Record<string, unknown>;
        required?: string[];
        [key: string]: unknown;
    };
}

/**
 * JS local tool callback function type
 */
export type JsLocalToolCallback<TArgs = unknown, TReturn = unknown> = (
    args?: TArgs
) => TReturn | Promise<TReturn>;

/**
 * Register a JS local tool with a JavaScript callback
 *
 * @example
 * ```typescript
 * registerJsLocalTool({
 *   name: "my-tool",
 *   description: "Does something useful",
 *   inputSchema: {
 *     type: "object",
 *     properties: {
 *       message: { type: "string" }
 *     }
 *   }
 * }, async (args) => {
 *   console.log("Tool called with:", args);
 *   return { success: true };
 * });
 * ```
 */
export function registerJsLocalTool<TArgs = unknown, TReturn = unknown>(
    config: JsLocalToolConfig,
    callback: JsLocalToolCallback<TArgs, TReturn>
): void;

/**
 * Call a JS local tool (invokes the registered JavaScript callback)
 *
 * @example
 * ```typescript
 * const result = await callJsLocalTool("my-tool", { message: "Hello!" });
 * ```
 */
export function callJsLocalTool<TReturn = unknown, TArgs = unknown>(
    name: string,
    args?: TArgs
): Promise<TReturn>;

/**
 * JS local tool metadata (returned from registry)
 */
export interface JsLocalToolMetadata {
    name: string;
    description?: string;
    input_schema?: {
        type: string;
        properties?: Record<string, unknown>;
        required?: string[];
        [key: string]: unknown;
    };
}

/**
 * JS Local Tool Registry - provides access to registered JS local tools
 */
export const JS_LOCAL_TOOLS: {
    /** Check if a JS local tool is registered */
    has(name: string): boolean;
    /** Get JS local tool metadata */
    get(name: string): JsLocalToolMetadata | undefined;
    /** List all registered JS local tools */
    list(): JsLocalToolMetadata[];
    /** Delete a JS local tool */
    delete(name: string): boolean;
    /** Clear all JS local tools */
    clear(): void;
};

// ============================================================================
// Python Callback API
// ============================================================================

/**
 * Python callback metadata (returned from registry)
 */
export interface PythonCallbackMetadata {
    name: string;
    description?: string;
    input_schema?: {
        type: string;
        properties?: Record<string, unknown>;
        required?: string[];
        [key: string]: unknown;
    };
}

/**
 * Call a Python callback (invokes registered Python function via pyo3)
 *
 * @example
 * ```typescript
 * const result = await callPythonCallback("my-callback", { value: 42 });
 * ```
 */
export function callPythonCallback<TReturn = unknown, TArgs = unknown>(
    name: string,
    args?: TArgs
): Promise<TReturn>;

/**
 * Python Callback Registry - provides access to registered Python callbacks
 */
export const PYTHON_CALLBACKS: {
    /** Check if a Python callback is registered */
    has(name: string): boolean;
    /** List all registered Python callbacks */
    list(): PythonCallbackMetadata[];
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

// ============================================================================
// Console Output Capturing
// ============================================================================

declare global {
    var __stdout: string[];
    var __stderr: string[];
    var registerMCP: typeof import('./runtime').registerMCP;
    var callMCPTool: typeof import('./runtime').callMCPTool;
    var REGISTRY: typeof import('./runtime').REGISTRY;
    var registerJsLocalTool: typeof import('./runtime').registerJsLocalTool;
    var callJsLocalTool: typeof import('./runtime').callJsLocalTool;
    var JS_LOCAL_TOOLS: typeof import('./runtime').JS_LOCAL_TOOLS;
    var callPythonCallback: typeof import('./runtime').callPythonCallback;
    var PYTHON_CALLBACKS: typeof import('./runtime').PYTHON_CALLBACKS;
    var fetch: typeof import('./runtime').fetch;
}
