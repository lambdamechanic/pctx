use anyhow::Result;
use clap::Args;
use pctx_agent_server::{AppState, start_server};
use pctx_code_mode::CodeMode;

#[derive(Debug, Args)]
pub struct DevCmd {
    /// Port to run the server on
    #[arg(long, short = 'p', default_value = "8080")]
    pub port: u16,

    /// Host to bind to
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,
}

impl DevCmd {
    pub async fn handle(&self) -> Result<()> {
        // Create default CodeMode (tools can be registered dynamically via API)
        let code_mode = CodeMode::default();

        // Create app state
        let state = AppState::new(code_mode);

        // Start the server
        start_server(&self.host, self.port, state).await?;

        Ok(())
    }
}
