use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallbackConfig {
    pub name: String,
    pub namespace: String,
    pub description: Option<String>,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
}
impl CallbackConfig {
    pub fn id(&self) -> String {
        format!("{}.{}", &self.namespace, &self.name)
    }
}
