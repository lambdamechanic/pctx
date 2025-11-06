use schemars::schema::RootSchema;

use crate::CodegenResult;

pub struct TypegenResult {
    pub type_signature: String,
    pub types: Vec<String>,
}
pub fn generate_typescript_types(
    json_schema: serde_json::Value,
    type_name: &str,
) -> CodegenResult<TypegenResult> {
    let schema: RootSchema = serde_json::from_value(json_schema)?;
    println!("{schema:#?}");
    todo!()
}

/// Iterates through the provided schema, assigning unique type names recursively
fn assign_type_names() {}
