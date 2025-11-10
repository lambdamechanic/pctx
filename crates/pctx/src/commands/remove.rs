use anyhow::Result;
use clap::Parser;
use log::info;
use pctx_config::Config;

use crate::utils::styles::{fmt_bold, fmt_dimmed, fmt_success};

#[derive(Debug, Clone, Parser)]
pub(crate) struct RemoveCmd {
    /// Name of the server to remove
    pub(crate) name: String,
}

impl RemoveCmd {
    pub(crate) fn handle(&self, mut cfg: Config) -> Result<Config> {
        cfg.remove_server(&self.name)?;

        cfg.save()?;

        info!(
            "{}",
            fmt_success(&format!(
                "{name} MCP Server removed from {path}",
                name = fmt_bold(&self.name),
                path = fmt_dimmed(cfg.path().as_str()),
            ))
        );

        Ok(cfg)
    }
}
