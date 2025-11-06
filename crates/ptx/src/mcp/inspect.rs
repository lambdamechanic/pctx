use rmcp::{
    ServiceExt,
    model::{
        ClientCapabilities, ClientInfo, Implementation, PaginatedRequestParam, ProtocolVersion,
        Tool,
    },
    transport::StreamableHttpClientTransport,
};

// TODO: implement errors
pub(crate) async fn inspect_mcp_server(url: &str) -> (Implementation, Vec<Tool>) {
    let transport = StreamableHttpClientTransport::from_uri(url);
    let client_info = ClientInfo {
        protocol_version: ProtocolVersion::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "test sse client".to_string(),
            title: None,
            version: "0.0.1".to_string(),
            website_url: None,
            icons: None,
        },
    };
    let client = client_info
        .serve(transport)
        .await
        .inspect_err(|e| {
            println!("client error: {e:?}");
        })
        .unwrap();

    // Get server info (required)
    let server_info = client
        .peer_info()
        .expect("MCP must support initialization for automatic ptx indexing")
        .server_info
        .clone();

    // Collect all tools
    let list_res = client.list_tools(None).await.unwrap();
    let mut all_tools = list_res.tools;
    let mut cursor = list_res.next_cursor;
    while cursor.is_some() {
        let list_res_page = client
            .list_tools(Some(PaginatedRequestParam {
                cursor: cursor.clone(),
            }))
            .await
            .unwrap();
        all_tools.extend(list_res_page.tools);
        cursor = list_res_page.next_cursor;
    }

    client.cancel().await.unwrap();

    (server_info, all_tools)
}
