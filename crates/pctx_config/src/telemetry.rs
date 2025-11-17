use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct TelemetryConfig {
    pub traces: TracesConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct TracesConfig {
    // pub sampling // TODO
    pub enabled: bool,
    #[serde(default)]
    pub exporters: Vec<ExporterConfig>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExporterConfig {
    pub name: String,
    pub url: Url,
    pub protocol: Protocol,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Protocol {
    #[serde(rename = "http")]
    Http,
    #[serde(rename = "grpc")]
    Grpc,
}
