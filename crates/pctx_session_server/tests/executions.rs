mod utils;

use crate::utils::{callback_tools, connect_websocket, create_test_server_with_session};
use pctx_code_mode::model::CallbackConfig;
use pctx_session_server::{CODE_MODE_SESSION_HEADER, model::WsJsonRpcMessage};
use serde_json::json;
use serial_test::serial;
use similar_asserts::assert_serde_eq;

#[tokio::test]
#[serial]
async fn test_exec_code_only() {
    let (session_id, server, _) = create_test_server_with_session().await;
    let mut ws = connect_websocket(&server, session_id)
        .await
        .into_websocket()
        .await;

    // Send execute_code request via WebSocket
    ws.send_json(&json!({
        "jsonrpc": "2.0",
        "id": "test-1",
        "method": "execute_code",
        "params": {
            "code": "async function run() { return 1 + 1; }"
        }
    }))
    .await;

    // Receive response
    let response: serde_json::Value = ws.receive_json().await;

    assert_serde_eq!(
        response,
        json!({
            "jsonrpc": "2.0",
            "id": "test-1",
            "result": {
                "success": true,
                "stdout": "",
                "stderr": "",
                "output": 2
            }
        })
    );
}

#[tokio::test]
#[serial]
async fn test_exec_code_console_output() {
    let (session_id, server, _) = create_test_server_with_session().await;
    let mut ws = connect_websocket(&server, session_id)
        .await
        .into_websocket()
        .await;

    let code = r#"
        async function run() {
            console.log("Test log");
            console.error("Test error");
            return "done";
        }
    "#;

    // Send execute_code request via WebSocket
    ws.send_json(&json!({
        "jsonrpc": "2.0",
        "id": "test-2",
        "method": "execute_code",
        "params": {
            "code": code
        }
    }))
    .await;

    // Receive response
    let response: serde_json::Value = ws.receive_json().await;

    assert_serde_eq!(
        response,
        json!({
            "jsonrpc": "2.0",
            "id": "test-2",
            "result": {
                "success": true,
                "stdout": "Test log",
                "stderr": "Test error",
                "output": "done"
            }
        })
    );
}

#[tokio::test]
#[serial]
async fn test_exec_code_syntax_err() {
    let (session_id, server, _) = create_test_server_with_session().await;
    let mut ws = connect_websocket(&server, session_id)
        .await
        .into_websocket()
        .await;

    let invalid_code = "
        async function run() {
            bloop x = 12;
            return x;
        }
    ";

    // Send execute_code request via WebSocket
    ws.send_json(&json!({
        "jsonrpc": "2.0",
        "id": "test-3",
        "method": "execute_code",
        "params": {
            "code": invalid_code
        }
    }))
    .await;

    // Receive response
    let response: serde_json::Value = ws.receive_json().await;

    assert_serde_eq!(
        response,
        json!({
            "jsonrpc": "2.0",
            "id": "test-3",
            "result": {
                "success": false,
                "stdout": "",
                "stderr": "ReferenceError: bloop is not defined\n    at run (file:///execute.js:2:3)\n    at file:///execute.js:6:22",
                "output": null
            }
        })
    );
}

#[test_log::test(tokio::test)]
#[serial]
async fn test_exec_callbacks() {
    let (session_id, server, _) = create_test_server_with_session().await;

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
    let mut ws = connect_websocket(&server, session_id)
        .await
        .into_websocket()
        .await;
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

    // Send execute_code request via WebSocket
    ws.send_json(&json!({
        "jsonrpc": "2.0",
        "id": "test-4",
        "method": "execute_code",
        "params": {
            "code": code
        }
    }))
    .await;

    // Confirm websocket handler sequence
    let msg: WsJsonRpcMessage = ws.receive_json().await;
    let (add_msg, req_id) = msg.into_request().unwrap();
    assert_serde_eq!(
        json!(add_msg),
        json!({
            "method": "execute_tool",
            "params": {
                "namespace": "test_math",
                "name": "add",
                "args": {
                    "a": 8,
                    "b": 2,
                }
            }
        })
    );
    let add_output = callbacks[0].1(Some(json!({
        "a": 8,
        "b": 2,
    })))
    .await
    .unwrap();
    ws.send_json(&json!({
        "jsonrpc": "2.0",
        "id": req_id,
        "result": {
            "output": add_output
        }
    }))
    .await;

    let msg: WsJsonRpcMessage = ws.receive_json().await;
    let (sub_msg, req_id) = msg.into_request().unwrap();
    assert_serde_eq!(
        json!(sub_msg),
        json!({
            "method": "execute_tool",
            "params": {
                "namespace": "test_math",
                "name": "subtract",
                "args": {
                    "a": 10,
                    "b": 5,
                }
            }
        })
    );
    let sub_output = callbacks[1].1(Some(json!({
        "a": 10,
        "b": 5})))
    .await
    .unwrap();
    ws.send_json(&json!({
        "jsonrpc": "2.0",
        "id": req_id,
        "result": {
            "output": sub_output
        }
    }))
    .await;

    let msg: WsJsonRpcMessage = ws.receive_json().await;
    let (mult_msg, req_id) = msg.into_request().unwrap();
    assert_serde_eq!(
        json!(mult_msg),
        json!({
            "method": "execute_tool",
            "params": {
                "namespace": "test_math",
                "name": "multiply",
                "args": {
                    "a": 5,
                    "b": 10,
                }
            }
        })
    );
    let mult_output = callbacks[2].1(Some(json!({
        "a": 5,
        "b": 10,
    })))
    .await
    .unwrap();
    ws.send_json(&json!({
        "jsonrpc": "2.0",
        "id": req_id,
        "result": {
            "output": mult_output
        }
    }))
    .await;

    let msg: WsJsonRpcMessage = ws.receive_json().await;
    let (div_msg, req_id) = msg.into_request().unwrap();
    assert_serde_eq!(
        json!(div_msg),
        json!({
            "method": "execute_tool",
            "params": {
                "namespace": "test_math",
                "name": "divide",
                "args": {
                    "a": 50,
                    "b": 2,
                }
            }
        })
    );
    let div_output = callbacks[3].1(Some(json!({
        "a": 50,
        "b": 2,
    })))
    .await
    .unwrap();
    ws.send_json(&json!({
        "jsonrpc": "2.0",
        "id": req_id,
        "result": {
            "output": div_output
        }
    }))
    .await;

    // Receive the execute_code response
    let response: serde_json::Value = ws.receive_json().await;
    assert_serde_eq!(
        response,
        json!({
            "jsonrpc": "2.0",
            "id": "test-4",
            "result": {
                "success": true,
                "stdout": "after add: 10\nafter subtract: 5\nafter multiply: 50\nafter divide: 25",
                "stderr": "",
                "output": 25
            }
        })
    );
}
