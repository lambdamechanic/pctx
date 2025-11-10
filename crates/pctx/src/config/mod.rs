use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;

use crate::config::server::ServerConfig;

pub(crate) mod auth;
pub(crate) mod server;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct Config {
    #[serde(skip_serializing)]
    path: Option<Utf8PathBuf>,

    #[serde(default)]
    pub servers: Vec<ServerConfig>,
}

impl Config {
    pub(crate) fn with_path(mut self, path: Utf8PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    pub(crate) fn path(&self) -> Utf8PathBuf {
        self.path.clone().unwrap_or(Self::default_path())
    }

    /// Loads config from json file, falling back on default path
    /// if none is provided
    pub(crate) fn load(path: &Utf8PathBuf) -> Result<Self> {
        debug!("Loading config from {path}");

        if !path.exists() {
            anyhow::bail!("Config file does not exist: {path}");
        }

        let contents =
            fs::read_to_string(path).context(format!("Failed reading config: {path}"))?;

        let mut cfg: Self =
            serde_json::from_str(&contents).context(format!("Failed loading config: {path}"))?;
        cfg.path = Some(path.clone());

        Ok(cfg)
    }

    /// Saves config to json file, falling back on default path if non is provided
    pub(crate) fn save(&self) -> Result<()> {
        let dest = self.path();
        debug!("Saving config to {dest}");
        let contents = serde_json::to_string_pretty(self).unwrap_or(json!(self).to_string());

        fs::write(&dest, contents).context(format!("Failed writing config: {dest}"))?;

        Ok(())
    }

    /// Default config path is ./pctx.json
    pub(crate) fn default_path() -> Utf8PathBuf {
        Utf8PathBuf::new().join("pctx.json")
    }

    pub(crate) fn add_server(&mut self, server: ServerConfig, force: bool) -> Result<()> {
        if !force && self.servers.iter().any(|s| s.name == server.name) {
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
