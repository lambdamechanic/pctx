pub mod case;
pub mod format;
pub mod schema_type;
pub mod typegen;
mod utils;

use indexmap::IndexMap;
use schemars::schema::Schema;
use thiserror::Error;

pub type SchemaDefinitions = IndexMap<String, Schema>;

pub type CodegenResult<T> = Result<T, CodegenError>;

#[derive(Debug, Error)]
pub enum CodegenError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Type generation error: {0}")]
    TypeGen(String),
}
