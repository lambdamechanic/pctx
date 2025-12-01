use pctx_agent_server::server::ApiDoc;
use std::fs;
use std::path::PathBuf;
use utoipa::OpenApi;

fn main() {
    let openapi = ApiDoc::openapi();
    let json = openapi
        .to_pretty_json()
        .expect("Failed to serialize OpenAPI spec");

    // Save to pctx_agent_server directory
    let output_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("openapi.json");
    fs::write(&output_path, json).expect("Failed to write OpenAPI spec to file");
    println!("OpenAPI spec saved to {}", output_path.display());
}
