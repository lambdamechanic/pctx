mod tools;
use codegen::CodegenError;
pub use tools::PctxTools;

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("MCP Connection error: {0}")]
    McpConnection(#[from] pctx_config::server::McpConnectionError),
    #[error("MCP Service error: {0}")]
    McpService(#[from] pctx_config::server::ServiceError),
    #[error("Codegen error: {0}")]
    Codegen(#[from] CodegenError),
    #[error("Error: {0}")]
    Message(String),
}
