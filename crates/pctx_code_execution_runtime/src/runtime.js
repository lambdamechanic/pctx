// PCTX Runtime - MCP Client and Console Capturing

const core = Deno.core;
const ops = core.ops;

// Debug: log available ops
const availableOps = Object.keys(ops).filter(k => k.startsWith('op_'));
if (availableOps.length === 0) {
    console.error("WARNING: No ops available! This indicates an extension loading issue.");
} else {
    // Only log MCP-related ops to reduce noise
    const mcpOps = availableOps.filter(k => k.includes('mcp'));
    if (mcpOps.length > 0) {
        console.log("Available MCP ops:", mcpOps);
    }
}

// ============================================================================
// CONSOLE OUTPUT CAPTURING
// ============================================================================

// Helper function to format console arguments
function formatConsoleArgs(...args) {
    return args.map(arg => {
        if (typeof arg === 'string') return arg;
        if (arg === null) return 'null';
        if (arg === undefined) return 'undefined';
        try {
            return JSON.stringify(arg);
        } catch {
            return String(arg);
        }
    }).join(' ');
}

// Set up console output capturing
globalThis.__stdout = [];
globalThis.__stderr = [];

// Override console.log to capture stdout
console.log = (...args) => {
    globalThis.__stdout.push(formatConsoleArgs(...args));
};

// Override console.error to capture stderr
console.error = (...args) => {
    globalThis.__stderr.push(formatConsoleArgs(...args));
};

// console.warn goes to stderr
console.warn = (...args) => {
    globalThis.__stderr.push(formatConsoleArgs(...args));
};

// console.info and console.debug go to stdout
console.info = (...args) => {
    globalThis.__stdout.push(formatConsoleArgs(...args));
};

console.debug = (...args) => {
    globalThis.__stdout.push(formatConsoleArgs(...args));
};

// ============================================================================
// MCP CLIENT API
// ============================================================================

/**
 * Register an MCP server
 * @param {Object} config - MCP server configuration
 * @param {string} config.name - Unique name for the MCP server
 * @param {string} config.url - URL of the MCP server
 */
export function registerMCP(config) {
    return ops.op_register_mcp(config);
}

/**
 * Call an MCP tool
 * @template T
 * @param {Object} call - Tool call configuration
 * @param {string} call.name - Name of the registered MCP server
 * @param {string} call.tool - Name of the tool to call
 * @param {Object} [call.arguments] - Arguments to pass to the tool
 * @returns {Promise<T>} The tool's response
 */
export async function callMCPTool(call) {
    return await ops.op_call_mcp_tool(call);
}

/**
 * MCP Registry singleton - provides access to registered servers
 */
export const REGISTRY = {
    /**
     * Check if an MCP server is registered
     * @param {string} name - Name of the MCP server
     * @returns {boolean} True if registered
     */
    has(name) {
        return ops.op_mcp_has(name);
    },

    /**
     * Get an MCP server configuration
     * @param {string} name - Name of the MCP server
     * @returns {Object|undefined} Server configuration or undefined
     */
    get(name) {
        return ops.op_mcp_get(name);
    },

    /**
     * Delete an MCP server configuration
     * @param {string} name - Name of the MCP server
     * @returns {boolean} True if deleted, false if not found
     */
    delete(name) {
        return ops.op_mcp_delete(name);
    },

    /**
     * Clear all MCP server configurations
     */
    clear() {
        ops.op_mcp_clear();
    }
};

/**
 * Fetch with host-based permissions
 * @param {string} url - URL to fetch
 * @param {Object} [options] - Fetch options (method, headers, body)
 * @returns {Promise<{status: number, headers: Object, body: string}>}
 */
async function fetch(url, options) {
    return await ops.op_fetch(url, options);
}

// ============================================================================
// LOCAL TOOL API (Runtime-agnostic Callbacks - JavaScript implementation)
// ============================================================================

// Store callbacks in a JavaScript Map (avoids V8 lifetime issues with Rust ops)
const localToolCallbacks = new Map();

// Flag to track if pre-registered tools have been loaded
let preRegisteredToolsLoaded = false;

// Load pre-registered tools (called lazily on first use)
function ensurePreRegisteredToolsLoaded() {
    if (preRegisteredToolsLoaded) return;
    preRegisteredToolsLoaded = true;

    // Load JS local tools
    if (typeof ops.op_get_pre_registered_tools === 'function') {
        const preRegistered = ops.op_get_pre_registered_tools();
        for (const tool of preRegistered) {
            try {
                // Evaluate the callback_data as JavaScript code to create the function
                const callback = eval(tool.callback_data);
                if (typeof callback !== 'function') {
                    console.error(`Pre-registered JS tool "${tool.metadata.name}" callback_data did not eval to a function`);
                    continue;
                }
                // Store the callback
                localToolCallbacks.set(tool.metadata.name, callback);
                console.log(`Auto-registered JS local tool: ${tool.metadata.name}`);
            } catch (e) {
                console.error(`Failed to register pre-registered JS tool "${tool.metadata.name}":`, e);
            }
        }
    }
}



// ============================================================================
// UNIFIED LOCAL TOOL API (Language-Agnostic - works for Python, JS, anything!)
// ============================================================================

/**
 * Call a local tool (UNIFIED API)
 *
 * This function works for ANY local tool regardless of its implementation language:
 * - Python callbacks (registered via wrap_python_callback)
 * - JavaScript callbacks (registered via registerJsLocalTool)
 * - Native Rust callbacks
 * - Future: Ruby, WASM, etc.
 *
 * From JavaScript's perspective, they're all just "tools" - the source language
 * is irrelevant!
 *
 * @template T
 * @param {string} name - Name of the tool to call
 * @param {Object} [args] - Arguments to pass to the tool
 * @returns {Promise<T>} The tool's return value
 */
export async function callLocalTool(name, args) {
    try {
        // Try the unified callback registry first (Python, native Rust, new JS callbacks)
        return await ops.op_execute_local_tool(name, args || null);
    } catch (err) {
        // Fall back to legacy JS tools if not found in unified registry
        if (err && err.message && err.message.includes("not found")) {
            ensurePreRegisteredToolsLoaded();
            const callback = localToolCallbacks.get(name);
            if (callback) {
                return await callback(args || {});
            }
        }
        // Re-throw error if tool truly doesn't exist
        throw err;
    }
}

// Make APIs available globally for convenience (matching original behavior)
globalThis.registerMCP = registerMCP;
globalThis.callMCPTool = callMCPTool;
globalThis.REGISTRY = REGISTRY;
globalThis.callLocalTool = callLocalTool;
globalThis.fetch = fetch;
