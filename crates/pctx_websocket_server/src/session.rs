/// WebSocket session management for PCTX local tools

// Re-export all types from pctx_session_types
pub use pctx_session_types::{
    CodeExecutorFn, ExecuteCodeError, ExecuteCodeResult, ExecuteToolError, OutgoingMessage,
    PendingExecution, RegisterToolError, SendError, Session, SessionId, SessionManager,
};

use crate::protocol::*;

/// Extension trait for SessionManager that adds WebSocket-specific functionality
pub trait SessionManagerExt {
    /// Execute a tool and wait for response
    fn execute_tool(
        &self,
        tool_name: &str,
        arguments: Option<serde_json::Value>,
        request_id: serde_json::Value,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, ExecuteToolError>> + Send;

    /// Execute TypeScript/JavaScript code using the registered executor
    fn execute_code(
        &self,
        code: &str,
    ) -> impl std::future::Future<Output = Result<ExecuteCodeResult, ExecuteCodeError>> + Send;

    /// Wrap user code with namespace-based tool proxies for direct function calls
    fn wrap_code_with_callable_tools(&self, user_code: &str, tools: &[String]) -> String;
}

impl SessionManagerExt for SessionManager {
    async fn execute_tool(
        &self,
        tool_name: &str,
        arguments: Option<serde_json::Value>,
        request_id: serde_json::Value,
    ) -> Result<serde_json::Value, ExecuteToolError> {
        // Find session for tool
        let session_id = self
            .get_tool_session(tool_name)
            .await
            .ok_or(ExecuteToolError::ToolNotFound)?;

        // Create std::sync::mpsc channel for response
        let (response_tx, response_rx) = std::sync::mpsc::channel();

        // Store pending execution
        let pending = PendingExecution {
            tool_name: tool_name.to_string(),
            response_tx,
        };
        eprintln!("[SessionManager] Adding pending execution for request_id: {:?}", request_id);
        self.pending_executions()
            .write()
            .await
            .insert(request_id.clone(), pending);
        eprintln!("[SessionManager] Pending execution added, count: {}", self.pending_executions().read().await.len());

        // Send execution request to client
        let request = JsonRpcRequest::new(
            "execute_tool",
            Some(
                serde_json::to_value(ExecuteToolParams {
                    name: tool_name.to_string(),
                    arguments,
                })
                .unwrap(),
            ),
            request_id.clone(),
        );

        self.send_to_session(
            &session_id,
            OutgoingMessage::Response(serde_json::to_value(request).unwrap()),
        )
        .await
        .map_err(|_| ExecuteToolError::SendFailed)?;

        // Wait for response with timeout
        eprintln!("[SessionManager] Waiting for tool execution response (30s timeout)...");
        let result = tokio::time::timeout(
            tokio::time::Duration::from_secs(30),
            tokio::task::spawn_blocking(move || response_rx.recv())
        ).await;
        eprintln!("[SessionManager] Tool execution wait completed: {:?}", result.as_ref().map(|_| "received").map_err(|_| "timeout"));

        // Clean up pending execution
        eprintln!("[SessionManager] Cleaning up pending execution for request_id: {:?}", request_id);
        self.pending_executions().write().await.remove(&request_id);

        match result {
            // spawn_blocking succeeded, recv succeeded, tool execution succeeded
            Ok(Ok(Ok(Ok(value)))) => Ok(value),
            // spawn_blocking succeeded, recv succeeded, tool execution failed
            Ok(Ok(Ok(Err(error)))) => Err(ExecuteToolError::ExecutionFailed(error)),
            // spawn_blocking succeeded, recv failed (channel closed)
            Ok(Ok(Err(_))) => Err(ExecuteToolError::ChannelClosed),
            // spawn_blocking failed (task panicked)
            Ok(Err(_)) => Err(ExecuteToolError::ChannelClosed),
            // Timeout waiting for response
            Err(_) => Err(ExecuteToolError::Timeout),
        }
    }

    async fn execute_code(&self, code: &str) -> Result<ExecuteCodeResult, ExecuteCodeError> {
        let executor_opt = self.code_executor().read().unwrap().clone();
        match executor_opt {
            Some(executor) => {
                // Get list of registered tools for this execution context
                let tools = self.list_tools().await;

                // Wrap the code with CALLABLE_TOOLS implementation
                let wrapped_code = self.wrap_code_with_callable_tools(code, &tools);

                executor(wrapped_code).await
            }
            None => Err(ExecuteCodeError::NoExecutor),
        }
    }

    fn wrap_code_with_callable_tools(&self, user_code: &str, tools: &[String]) -> String {
        // Parse tools into namespaces and create proxy objects
        // Tools are formatted as "namespace.toolName"

        // Build a map of namespaces to tool names
        let mut namespaces: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        for tool in tools {
            if let Some((namespace, tool_name)) = tool.split_once('.') {
                namespaces.entry(namespace.to_string())
                    .or_default()
                    .push(tool_name.to_string());
            }
        }

        // JavaScript built-in globals that we shouldn't shadow
        let js_builtins = [
            "Math", "Date", "JSON", "Array", "Object", "String", "Number",
            "Boolean", "Function", "RegExp", "Error", "Promise", "Map", "Set",
            "WeakMap", "WeakSet", "Symbol", "Proxy", "Reflect", "console"
        ];

        // Generate JavaScript proxy objects for each namespace
        let mut namespace_setup = String::new();
        for (namespace, tool_names) in namespaces {
            // Check if this namespace conflicts with a JavaScript built-in
            if js_builtins.contains(&namespace.as_str()) {
                // For built-in conflicts, create a proxy that intercepts property access
                namespace_setup.push_str(&format!(
                    r#"
// Namespace: {} (proxied to avoid shadowing built-in)
const _{}_original = (globalThis as any).{};
(globalThis as any).{} = new Proxy(_{}_original, {{
    get(target: any, prop: any) {{
        const toolMap: any = {{{}}};
        if (toolMap[prop]) {{
            return toolMap[prop];
        }}
        return target[prop];
    }}
}});
"#,
                    namespace, namespace, namespace, namespace, namespace,
                    tool_names.iter()
                        .map(|tool_name| {
                            let full_name = format!("{}.{}", namespace, tool_name);
                            format!(
                                r#"{}: async function(params: any) {{ return await callLocallyCallableTool('{}', params); }}"#,
                                tool_name, full_name
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            } else {
                // For non-conflicting namespaces, create a simple object
                namespace_setup.push_str(&format!(
                    r#"
// Namespace: {}
const {} = {{}};
"#,
                    namespace, namespace
                ));

                for tool_name in tool_names {
                    let full_name = format!("{}.{}", namespace, tool_name);
                    namespace_setup.push_str(&format!(
                        r#"
{}.{} = async function(params) {{
    return await callLocallyCallableTool('{}', params);
}};
"#,
                        namespace, tool_name, full_name
                    ));
                }
            }
        }

        // Also keep CALLABLE_TOOLS for backwards compatibility
        let wrapped = format!(
            r#"
// PCTX Tool Injection - Direct function calls
{}

// Backwards compatibility: CALLABLE_TOOLS
const CALLABLE_TOOLS = {{
    execute: async function(toolName, params) {{
        return await callLocallyCallableTool(toolName, params);
    }},
    list: async function() {{
        return [];
    }}
}};

// User code
{}
"#,
            namespace_setup,
            user_code
        );

        wrapped
    }
}
