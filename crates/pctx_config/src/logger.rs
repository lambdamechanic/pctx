use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggerConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub level: LogLevel,
    #[serde(default)]
    pub format: LoggerFormat,
    #[serde(default = "default_true")]
    pub colors: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum LoggerFormat {
    #[serde(rename = "compact")]
    #[default]
    Compact,
    #[serde(rename = "pretty")]
    Pretty,
    #[serde(rename = "json")]
    Json,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: LogLevel::Info,
            format: LoggerFormat::Compact,
            colors: true,
        }
    }
}

// Define an enumeration for log levels
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub enum LogLevel {
    /// The "trace" level.
    #[serde(rename = "trace")]
    Trace,
    /// The "debug" level.
    #[serde(rename = "debug")]
    Debug,
    /// The "info" level.
    #[serde(rename = "info")]
    #[default]
    Info,
    /// The "warn" level.
    #[serde(rename = "warn")]
    Warn,
    /// The "error" level.
    #[serde(rename = "error")]
    Error,
}
