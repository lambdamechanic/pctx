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

/**
 * Register a JS local tool with a JavaScript callback
 * @param {Object} config - JS local tool configuration
 * @param {string} config.name - Unique name for the tool
 * @param {string} [config.description] - Tool description
 * @param {Object} [config.inputSchema] - JSON Schema for tool input
 * @param {Function} callback - JavaScript function to invoke when tool is called
 */
export function registerJsLocalTool(config, callback) {
    if (typeof callback !== 'function') {
        throw new TypeError('callback must be a function');
    }

    // Store the callback in JavaScript
    localToolCallbacks.set(config.name, callback);

    // Register metadata in Rust (using generic local tool ops)
    return ops.op_register_local_tool_metadata({
        name: config.name,
        description: config.description,
        input_schema: config.inputSchema
    });
}

/**
 * Call a JS local tool (invokes the registered JavaScript callback)
 * @template T
 * @param {string} name - Name of the registered JS local tool
 * @param {Object} [args] - Arguments to pass to the callback
 * @returns {Promise<T>} The callback's return value
 */
export async function callJsLocalTool(name, args) {
    // Ensure pre-registered tools are loaded (lazy initialization)
    ensurePreRegisteredToolsLoaded();

    // Get the callback from our JavaScript Map
    const callback = localToolCallbacks.get(name);

    if (!callback) {
        throw new Error(`JS local tool "${name}" not found`);
    }

    // Invoke the callback with the arguments
    // The callback can be sync or async, so we await it
    return await callback(args);
}

/**
 * JS Local Tool Registry - provides access to registered JS local tools
 */
export const JS_LOCAL_TOOLS = {
    /**
     * Check if a JS local tool is registered
     * @param {string} name - Name of the JS local tool
     * @returns {boolean} True if registered
     */
    has(name) {
        return ops.op_local_tool_has(name);
    },

    /**
     * Get JS local tool metadata
     * @param {string} name - Name of the JS local tool
     * @returns {Object|undefined} Tool metadata or undefined
     */
    get(name) {
        return ops.op_local_tool_get(name);
    },

    /**
     * List all registered JS local tools
     * @returns {Array<Object>} Array of tool metadata
     */
    list() {
        return ops.op_local_tool_list();
    },

    /**
     * Delete a JS local tool
     * @param {string} name - Name of the JS local tool
     * @returns {boolean} True if deleted, false if not found
     */
    delete(name) {
        localToolCallbacks.delete(name);
        return ops.op_local_tool_delete(name);
    },

    /**
     * Clear all JS local tools
     */
    clear() {
        localToolCallbacks.clear();
        ops.op_local_tool_clear();
    }
};

// ============================================================================
// PYTHON CALLBACK API
// ============================================================================

/**
 * Call a Python callback (invokes registered Python function via pyo3)
 * @template T
 * @param {string} name - Name of the registered Python callback
 * @param {Object} [args] - Arguments to pass to the callback
 * @returns {Promise<T>} The callback's return value
 */
export async function callPythonCallback(name, args) {
    if (typeof ops.op_python_callback_execute !== 'function') {
        throw new Error('Python callback runtime not available');
    }
    return await ops.op_python_callback_execute(name, args);
}

/**
 * Python Callback Registry - provides access to registered Python callbacks
 */
export const PYTHON_CALLBACKS = {
    /**
     * Check if a Python callback is registered
     * @param {string} name - Name of the Python callback
     * @returns {boolean} True if registered
     */
    has(name) {
        if (typeof ops.op_python_callback_has !== 'function') {
            return false;
        }
        return ops.op_python_callback_has(name);
    },

    /**
     * List all registered Python callbacks
     * @returns {Array<Object>} Array of callback metadata
     */
    list() {
        if (typeof ops.op_python_callback_list !== 'function') {
            return [];
        }
        return ops.op_python_callback_list();
    },
};

// Make APIs available globally for convenience (matching original behavior)
globalThis.registerMCP = registerMCP;
globalThis.callMCPTool = callMCPTool;
globalThis.REGISTRY = REGISTRY;
globalThis.registerJsLocalTool = registerJsLocalTool;
globalThis.callJsLocalTool = callJsLocalTool;
globalThis.JS_LOCAL_TOOLS = JS_LOCAL_TOOLS;
globalThis.callPythonCallback = callPythonCallback;
globalThis.PYTHON_CALLBACKS = PYTHON_CALLBACKS;
globalThis.fetch = fetch;
