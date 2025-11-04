mod ts_go_check;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[derive(Debug, Error)]
pub enum SdkRunnerError {
    #[error("Internal check error: {0}")]
    InternalError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, SdkRunnerError>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Diagnostic {
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub severity: String,
    pub code: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckResult {
    pub success: bool,
    pub diagnostics: Vec<Diagnostic>,
}

/// Check TypeScript code and return structured diagnostics if there are problems
///
/// This function performs TypeScript type checking with typescript-go:
/// - Syntax validation
/// - TypeScript parsing
/// - Type inference and checking
/// - Detects type mismatches (e.g., `const x: number = "string"`)
///
/// The typescript-go binary is automatically downloaded during build and bundled with the crate.
///
/// # Arguments
/// * `code` - The TypeScript code snippet to check
///
/// # Returns
/// * `Ok(CheckResult)` - Contains type diagnostics and success status
///
/// # Errors
/// * `ParseError` - If the code cannot be parsed
/// * `InternalError` - If typescript-go execution fails
/// * `IoError` - If file I/O fails
///
/// # Examples
/// ```
/// use sdk_runner::check;
///
/// // This will pass - types match
/// let code = r#"const greeting: string = "hello";"#;
/// let result = check(code).expect("check should not fail");
/// assert!(result.success);
/// ```
pub fn check(code: &str) -> Result<CheckResult> {
    let binary_path = ts_go_check::get_tsgo_binary_path()
        .ok_or_else(|| SdkRunnerError::InternalError(
            "typescript-go binary not found. This should not happen - please report this build issue.".to_string()
        ))?;

    ts_go_check::check_with_tsgo(code, &binary_path)
}

#[cfg(test)]
mod tests;
