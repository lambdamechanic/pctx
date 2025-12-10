#[cfg(test)]
mod tests {
    use std::fs;

    fn extract_tool_description(source: &str, tool_name: &str) -> Option<String> {
        // Find the #[tool( section for the given tool
        let tool_marker = format!("async fn {}(", tool_name);
        let tool_start = source.find(&tool_marker)?;

        // Find the #[tool( attribute before the function
        let before_fn = &source[..tool_start];
        let attr_start = before_fn.rfind("#[tool(")?;
        let attr_section = &source[attr_start..tool_start];

        // Extract description value
        let desc_start = attr_section.find(r#"description = ""#)?;
        let after_desc = &attr_section[desc_start + r#"description = ""#.len()..];

        // Find the closing quote (accounting for multi-line strings)
        let mut depth = 0;
        let mut end_pos = 0;
        let chars: Vec<char> = after_desc.chars().collect();

        for (i, &ch) in chars.iter().enumerate() {
            if ch == '"' && (i == 0 || chars[i - 1] != '\\') {
                if depth == 0 {
                    end_pos = i;
                    break;
                }
            }
        }

        Some(after_desc[..end_pos].to_string())
    }

    fn normalize_whitespace(s: &str) -> String {
        s.lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn test_tool_descriptions_match_markdown_files() {
        // Get the workspace root
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let workspace_root = std::path::PathBuf::from(&manifest_dir)
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let tool_descriptions_dir = workspace_root.join("tool_descriptions");

        // Read service.rs
        let service_rs_path = std::path::PathBuf::from(&manifest_dir).join("src/service.rs");
        let service_rs = fs::read_to_string(service_rs_path)
            .expect("Failed to read service.rs");

        // Read the markdown files
        let list_functions_md =
            fs::read_to_string(tool_descriptions_dir.join("list_functions.md"))
                .expect("Failed to read list_functions.md");
        let get_function_details_md =
            fs::read_to_string(tool_descriptions_dir.join("get_function_details.md"))
                .expect("Failed to read get_function_details.md");
        let execute_md = fs::read_to_string(tool_descriptions_dir.join("execute.md"))
            .expect("Failed to read execute.md");

        // Extract descriptions from service.rs
        let service_list_functions = extract_tool_description(&service_rs, "list_functions")
            .expect("Failed to extract list_functions description from service.rs");
        let service_get_function_details = extract_tool_description(&service_rs, "get_function_details")
            .expect("Failed to extract get_function_details description from service.rs");
        let service_execute = extract_tool_description(&service_rs, "execute")
            .expect("Failed to extract execute description from service.rs");

        // Normalize whitespace for comparison
        let list_functions_md_normalized = normalize_whitespace(&list_functions_md);
        let service_list_functions_normalized = normalize_whitespace(&service_list_functions);

        let get_function_details_md_normalized = normalize_whitespace(&get_function_details_md);
        let service_get_function_details_normalized = normalize_whitespace(&service_get_function_details);

        let execute_md_normalized = normalize_whitespace(&execute_md);
        let service_execute_normalized = normalize_whitespace(&service_execute);

        // Assert that the descriptions match
        assert_eq!(
            list_functions_md_normalized, service_list_functions_normalized,
            "list_functions description in service.rs does not match tool_descriptions/list_functions.md"
        );

        assert_eq!(
            get_function_details_md_normalized, service_get_function_details_normalized,
            "get_function_details description in service.rs does not match tool_descriptions/get_function_details.md"
        );

        assert_eq!(
            execute_md_normalized, service_execute_normalized,
            "execute description in service.rs does not match tool_descriptions/execute.md"
        );
    }
}
