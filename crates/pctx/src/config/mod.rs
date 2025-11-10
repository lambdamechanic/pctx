use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;

use crate::config::server::ServerConfig;

pub(crate) mod auth;
pub(crate) mod server;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct Config {
    #[serde(default)]
    pub servers: Vec<ServerConfig>,
}

impl Config {
    /// Loads config from json file, falling back on default path
    /// if none is provided
    pub(crate) fn load(path: Option<Utf8PathBuf>) -> Result<Self> {
        let config_path = path.unwrap_or(Self::default_path());

        if !config_path.exists() {
            anyhow::bail!("Config file does not exist: {config_path}");
        }

        let contents = fs::read_to_string(&config_path)
            .context(format!("Failed reading config: {config_path}"))?;

        serde_json::from_str(&contents).context(format!("Failed loading config: {config_path}"))
    }

    /// Saves config to json file, falling back on default path if non is provided
    pub(crate) fn save(&self, dest: Option<Utf8PathBuf>) -> Result<()> {
        let config_path = dest.unwrap_or(Self::default_path());
        let contents = serde_json::to_string_pretty(self).unwrap_or(json!(self).to_string());

        fs::write(&config_path, contents)
            .context(format!("Failed writing config: {config_path}"))?;

        Ok(())
    }

    /// Default config path is ./pctx.json
    pub(crate) fn default_path() -> Utf8PathBuf {
        Utf8PathBuf::new().join("pctx.json")
    }

    pub(crate) fn add_server(&mut self, server: ServerConfig) -> Result<()> {
        if self.servers.iter().any(|s| s.name == server.name) {
            anyhow::bail!("Server '{}' already exists", server.name);
        }

        self.servers.push(server);
        Ok(())
    }

    pub(crate) fn remove_server(&mut self, name: &str) -> Result<()> {
        let index = self
            .servers
            .iter()
            .position(|s| s.name == name)
            .context(format!("Server '{name}' not found"))?;

        self.servers.remove(index);
        Ok(())
    }

    pub(crate) fn get_server(&self, name: &str) -> Option<&ServerConfig> {
        self.servers.iter().find(|s| s.name == name)
    }

    pub(crate) fn get_server_mut(&mut self, name: &str) -> Option<&mut ServerConfig> {
        self.servers.iter_mut().find(|s| s.name == name)
    }
}
