mod utils;

use crate::utils::{callback_tools, connect_websocket, create_test_server_with_session};
use pctx_agent_server::{
    CODE_MODE_SESSION_HEADER,
    model::{WsExecuteTool, WsExecuteToolResult},
};
use pctx_code_mode::model::CallbackConfig;
use rmcp::model::{JsonRpcRequest, JsonRpcResponse, Request as JsonRpcRequestData};
use serde_json::json;
use serial_test::serial;
use similar_asserts::assert_serde_eq;

#[tokio::test]
#[serial]
async fn test_exec_code_only() {
    let (session_id, server, _) = create_test_server_with_session().await;

    // Execute simple code without any registered tools
    let exec_res = server
        .post("/code-mode/execute")
        .add_header(CODE_MODE_SESSION_HEADER, session_id.to_string())
        .json(&json!({
            "code": "async function run() { return 1 + 1; }",
        }))
        .await;

    exec_res.assert_status_ok();
    exec_res.assert_json(&json!({
        "success": true,
        "stdout": "",
        "stderr": "",
        "output": 2
    }));
}

#[tokio::test]
#[serial]
async fn test_exec_code_console_output() {
    let (session_id, server, _) = create_test_server_with_session().await;

    let code = r#"
        async function run() {
            console.log("Test log");
            console.error("Test error");
            return "done";
        }
    "#;

    // Execute simple code without any registered tools
    let exec_res = server
        .post("/code-mode/execute")
        .add_header(CODE_MODE_SESSION_HEADER, session_id.to_string())
        .json(&json!({"code": code}))
        .await;

    exec_res.assert_status_ok();
    exec_res.assert_json(&json!({
        "success": true,
        "stdout": "Test log",
        "stderr": "Test error",
        "output": "done"
    }));
}

#[tokio::test]
#[serial]
async fn test_exec_code_syntax_err() {
    let (session_id, server, _) = create_test_server_with_session().await;

    let invalid_code = "
        async function run() {
            bloop x = 12;
            return x;
        }
    ";

    // Execute simple code without any registered tools
    let exec_res = server
        .post("/code-mode/execute")
        .add_header(CODE_MODE_SESSION_HEADER, session_id.to_string())
        .json(&json!({"code": invalid_code}))
        .await;

    exec_res.assert_status_ok();
    exec_res.assert_json(&json!({
        "success": false,
        "stdout": "",
        "stderr": "ReferenceError: bloop is not defined
    at run (file:///execute.js:2:3)
    at file:///execute.js:6:22",
        "output": null
    }));
}

#[tokio::test]
#[serial]
async fn test_exec_callbacks() {
    let (session_id, server, _) = create_test_server_with_session().await;
    let mut ws = connect_websocket(&server, session_id)
        .await
        .into_websocket()
        .await;

    // register tools
    let callbacks = callback_tools();
    let test_tools: Vec<CallbackConfig> = callback_tools().into_iter().map(|(c, _)| c).collect();
    let register_res = server
        .post("/register/tools")
        .add_header(CODE_MODE_SESSION_HEADER, session_id.to_string())
        .json(&json!({
            "tools": test_tools,
        }))
        .await;
    register_res.assert_status_ok();

    // kick off execution script that uses all of the tools
    let code = "
        async function run() {
            let value = await TestMath.add({a: 8, b: 2});
            console.log(`after add: ${value}`);
            value = await TestMath.subtract({a: value, b: 5});
            console.log(`after subtract: ${value}`);
            value = await TestMath.multiply({a: value, b: 10});
            console.log(`after multiply: ${value}`);
            value = await TestMath.divide({a: value, b: 2});
            console.log(`after divide: ${value}`);
            return value;
        }";

    // Spawn the execution request as a separate task so we can test websocket messages concurrently
    let exec_handle = tokio::spawn(async move {
        server
            .post("/code-mode/execute")
            .add_header(CODE_MODE_SESSION_HEADER, session_id.to_string())
            .json(&json!({"code": code}))
            .await
    });

    // Confirm websocket handler sequence
    let add_msg: JsonRpcRequest<JsonRpcRequestData<String, WsExecuteTool>> =
        ws.receive_json().await;
    assert_serde_eq!(
        json!(add_msg.request),
        json!({
            "method": "execute_tool",
            "params": {
                "id": add_msg.id,
                "namespace": "test_math",
                "name": "add",
                "args": {
                    "a": 8,
                    "b": 2,
                }
            }
        })
    );
    let add_output = callbacks[0].1(add_msg.request.params.args).await.unwrap();
    ws.send_json(&JsonRpcResponse {
        jsonrpc: rmcp::model::JsonRpcVersion2_0,
        id: add_msg.id,
        result: WsExecuteToolResult {
            output: Some(add_output.clone()),
        },
    })
    .await;

    let sub_msg: JsonRpcRequest<JsonRpcRequestData<String, WsExecuteTool>> =
        ws.receive_json().await;
    assert_serde_eq!(
        json!(sub_msg.request),
        json!({
            "method": "execute_tool",
            "params": {
                "id": sub_msg.id,
                "namespace": "test_math",
                "name": "subtract",
                "args": {
                    "a": 10,
                    "b": 5,
                }
            }
        })
    );
    let sub_output = callbacks[1].1(sub_msg.request.params.args).await.unwrap();
    ws.send_json(&JsonRpcResponse {
        jsonrpc: rmcp::model::JsonRpcVersion2_0,
        id: sub_msg.id,
        result: WsExecuteToolResult {
            output: Some(sub_output.clone()),
        },
    })
    .await;

    let mult_msg: JsonRpcRequest<JsonRpcRequestData<String, WsExecuteTool>> =
        ws.receive_json().await;
    assert_serde_eq!(
        json!(mult_msg.request),
        json!({
            "method": "execute_tool",
            "params": {
                "id": mult_msg.id,
                "namespace": "test_math",
                "name": "multiply",
                "args": {
                    "a": 5,
                    "b": 10,
                }
            }
        })
    );
    let mult_output = callbacks[2].1(mult_msg.request.params.args).await.unwrap();
    ws.send_json(&JsonRpcResponse {
        jsonrpc: rmcp::model::JsonRpcVersion2_0,
        id: mult_msg.id,
        result: WsExecuteToolResult {
            output: Some(mult_output.clone()),
        },
    })
    .await;

    let div_msg: JsonRpcRequest<JsonRpcRequestData<String, WsExecuteTool>> =
        ws.receive_json().await;
    assert_serde_eq!(
        json!(div_msg.request),
        json!({
            "method": "execute_tool",
            "params": {
                "id": div_msg.id,
                "namespace": "test_math",
                "name": "divide",
                "args": {
                    "a": 50,
                    "b": 2,
                }
            }
        })
    );
    let div_output = callbacks[3].1(div_msg.request.params.args).await.unwrap();
    ws.send_json(&JsonRpcResponse {
        jsonrpc: rmcp::model::JsonRpcVersion2_0,
        id: div_msg.id,
        result: WsExecuteToolResult {
            output: Some(div_output.clone()),
        },
    })
    .await;

    let exec_res = exec_handle.await.unwrap();
    exec_res.assert_status_ok();
    exec_res.assert_json(&json!({
        "success": true,
        "stdout": "after add: 10\nafter subtract: 5\nafter multiply: 50\nafter divide: 25",
        "stderr": "",
        "output": 25
    }));
}
