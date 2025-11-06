use deno_ast::{
    EmitOptions, MediaType, ModuleSpecifier, ParseParams, TranspileModuleOptions, TranspileOptions,
};

#[derive(Debug, thiserror::Error)]
pub enum TranspileError {
    #[error("Failed to parse TypeScript: {0}")]
    ParseError(String),

    #[error("Failed to transpile: {0}")]
    TranspileError(String),

    #[error("Invalid module specifier: {0}")]
    InvalidSpecifier(String),
}

pub type Result<T> = std::result::Result<T, TranspileError>;

/// Transpile TypeScript code to JavaScript
///
/// This function takes TypeScript code and converts it to JavaScript using ``deno_ast``.
/// Type annotations are stripped and modern JavaScript is emitted.
///
/// # Arguments
/// * `code` - The TypeScript/JavaScript code to transpile
/// * `specifier` - Optional module specifier (defaults to "<file:///execute.ts>")
///
/// # Returns
/// * `Ok(String)` - The transpiled JavaScript code
/// * `Err(TranspileError)` - If parsing or transpilation fails
///
/// # Errors
/// Returns an error in the following cases:
/// * `TranspileError::InvalidSpecifier` - If the provided module specifier is invalid
/// * `TranspileError::ParseError` - If the TypeScript code cannot be parsed
/// * `TranspileError::TranspileError` - If the parsed code cannot be transpiled to JavaScript
///
/// # Examples
/// ```
/// use deno_transpiler::transpile;
///
/// let ts_code = r#"
///     const add = (a: number, b: number): number => a + b;
///     export default add(1, 2);
/// "#;
/// let js_code = transpile(ts_code, None).expect("transpilation should succeed");
/// assert!(js_code.contains("const add"));
/// assert!(!js_code.contains(": number")); // Type annotations removed
/// ```
pub fn transpile(code: &str, specifier: Option<&str>) -> Result<String> {
    let specifier = ModuleSpecifier::parse(specifier.unwrap_or("file:///execute.ts"))
        .map_err(|e| TranspileError::InvalidSpecifier(e.to_string()))?;

    let parsed = deno_ast::parse_module(ParseParams {
        specifier: specifier.clone(),
        text: code.into(),
        media_type: MediaType::TypeScript,
        capture_tokens: false,
        scope_analysis: false,
        maybe_syntax: None,
    })
    .map_err(|e| TranspileError::ParseError(e.to_string()))?;

    let transpiled = parsed
        .transpile(
            &TranspileOptions::default(),
            &TranspileModuleOptions::default(),
            &EmitOptions {
                source_map: deno_ast::SourceMapOption::None,
                ..Default::default()
            },
        )
        .map_err(|e| TranspileError::TranspileError(e.to_string()))?;

    Ok(transpiled.into_source().text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transpile_simple_typescript() {
        let code = r"const x: number = 42;";
        let result = transpile(code, None).unwrap();
        assert!(result.contains("const x"));
        assert!(!result.contains(": number"));
    }

    #[test]
    fn test_transpile_function_with_types() {
        let code = r"
            function add(a: number, b: number): number {
                return a + b;
            }
        ";
        let result = transpile(code, None).unwrap();
        assert!(result.contains("function add"));
        assert!(!result.contains(": number"));
    }

    #[test]
    fn test_transpile_arrow_function() {
        let code = r"const multiply = (x: number, y: number): number => x * y;";
        let result = transpile(code, None).unwrap();
        assert!(result.contains("const multiply"));
        assert!(!result.contains(": number"));
    }

    #[test]
    fn test_transpile_interface() {
        let code = r#"
            interface Person {
                name: string;
                age: number;
            }
            const person: Person = { name: "Alice", age: 30 };
        "#;
        let result = transpile(code, None).unwrap();
        assert!(!result.contains("interface Person"));
        assert!(result.contains("const person"));
        assert!(!result.contains(": Person"));
    }

    #[test]
    fn test_transpile_plain_javascript() {
        let code = r"const x = 42; console.log(x);";
        let result = transpile(code, None).unwrap();
        assert!(result.contains("const x = 42"));
        assert!(result.contains("console.log"));
    }

    #[test]
    fn test_transpile_with_imports() {
        let code = r#"
            import { z } from "zod";
            const schema: z.ZodType = z.string();
        "#;
        let result = transpile(code, None).unwrap();
        assert!(result.contains(r#"from "zod""#));
        assert!(!result.contains(": z.ZodType"));
    }
}
