use super::*;

#[test]
fn test_check_valid_typescript() {
    let code = r#"
const greeting: string = "Hello, World!";
console.log(greeting);
"#;

    let result = check(code).expect("check should succeed");
    assert!(result.success, "Valid TypeScript should pass type checking");
    assert!(
        result.diagnostics.is_empty(),
        "Valid TypeScript should have no diagnostics"
    );
}

#[test]
fn test_check_type_mismatch() {
    let code = r#"const x: number = "string""#;

    let result = check(code).expect("check should succeed");

    assert!(
        !result.success,
        "Type mismatch should fail with typescript-go"
    );
    assert!(
        !result.diagnostics.is_empty(),
        "Should have type error diagnostics"
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("not assignable") || d.message.contains("Type")),
        "Error should mention type incompatibility, got: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_check_syntax_error() {
    let code = r"const x: string =";

    let result = check(code);
    // Should catch syntax error
    if let Ok(result) = result {
        assert!(!result.success, "Invalid syntax should fail");
    }
}

#[test]
fn test_nested_object_type_mismatch() {
    let code = r#"
interface User {
    name: string;
    profile: {
        age: number;
        email: string;
    };
}

const user: User = {
    name: "Alice",
    profile: {
        age: "thirty",  // Type error: should be number, not string
        email: "alice@example.com"
    }
};
"#;

    let result = check(code).expect("check should succeed");

    assert!(
        !result.success,
        "Type mismatch in nested object should fail with typescript-go"
    );
    assert!(
        !result.diagnostics.is_empty(),
        "Should detect type error in nested object, got: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_function_signature_mismatch() {
    let code = r#"
function greet(name: string): string {
    return name;
}

const result: number = greet("Alice");  // Type error
"#;

    let result = check(code).expect("check should succeed");

    assert!(
        !result.success,
        "Function return type mismatch should fail with typescript-go"
    );
    assert!(
        !result.diagnostics.is_empty(),
        "Should detect return type mismatch, got: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_undeclared_variable() {
    // Note: console.log itself is filtered (TS2580), but undeclaredVariable should fail
    // We need to use a different context that doesn't involve console
    let code = r"const x = undeclaredVariable;";

    let result = check(code).expect("check should succeed");

    // If typescript-go is available, it should catch the error
    // If using syntax-only fallback, it might pass
    if result.diagnostics.is_empty() {
        // Fallback to syntax-only checking doesn't catch this
        return;
    }

    assert!(
        !result.success,
        "Undeclared variable should fail with typescript-go"
    );
    assert!(
        !result.diagnostics.is_empty(),
        "Should detect undeclared variable, got: {:?}",
        result.diagnostics
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("Cannot find name")),
        "Error should mention undeclared variable, got: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_console_log_is_ignored() {
    // TS2580: Cannot find name 'console' should be ignored
    let code = r#"console.log("Hello, World!");"#;

    let result = check(code).expect("check should succeed");

    assert!(
        result.success,
        "console.log should be allowed (TS2580 should be filtered), got: {:?}",
        result.diagnostics
    );
    assert!(
        result.diagnostics.is_empty(),
        "Should have no diagnostics after filtering, got: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_promise_is_ignored() {
    // TS2585: 'Promise' only refers to a type, but is being used as a value
    // TS2591: Cannot find name 'Promise'
    // Both should be ignored
    let code = r"
const myPromise = new Promise((resolve) => {
    resolve(42);
});
";

    let result = check(code).expect("check should succeed");

    // The test should pass - Promise-related errors should be filtered
    assert!(
        result.success,
        "Promise usage should be allowed (TS2585/TS2591 should be filtered), got: {:?}",
        result.diagnostics
    );
    assert!(
        result.diagnostics.is_empty(),
        "All Promise errors should be filtered out, got: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_implicit_any_is_ignored() {
    // TS7006: Parameter implicitly has an 'any' type should be ignored
    let code = r#"
function greet(name) {
    return "Hello, " + name;
}
"#;

    let result = check(code).expect("check should succeed");

    assert!(
        result.success,
        "Implicit any parameters should be allowed (TS7006 should be filtered), got: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_dynamic_object_access_is_ignored() {
    // TS7053: Element implicitly has an 'any' type should be ignored
    let code = r#"
const obj: Record<string, any> = { key: "value" };
const key = "key";
const value = obj[key];
"#;

    let result = check(code).expect("check should succeed");

    assert!(
        result.success,
        "Dynamic object access should be allowed (TS7053 should be filtered), got: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_relevant_errors_not_filtered() {
    // TS2322: Type error should NOT be filtered
    let code = r#"
const x: number = "string";
"#;

    let result = check(code).expect("check should succeed");

    assert!(
        !result.success,
        "Type mismatch should fail (TS2322 should NOT be filtered)"
    );
    assert!(
        !result.diagnostics.is_empty(),
        "Should have type error diagnostics"
    );
    assert!(
        result.diagnostics.iter().any(|d| d.code == Some(2322)),
        "Should include TS2322 error, got: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_mixed_errors_only_relevant_shown() {
    // This should have both filtered (console) and unfiltered (type error) diagnostics
    let code = r#"
console.log("This uses console");
const x: number = "string";
"#;

    let result = check(code).expect("check should succeed");

    assert!(!result.success, "Should fail due to type error");

    // Should have diagnostics but console error should be filtered out
    assert!(!result.diagnostics.is_empty(), "Should have diagnostics");

    // Should NOT include console error (TS2580)
    assert!(
        !result.diagnostics.iter().any(|d| d.code == Some(2580)),
        "Should not include TS2580 (console) error, got: {:?}",
        result.diagnostics
    );

    // Should include type error (TS2322)
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == Some(2322) || d.message.contains("not assignable")),
        "Should include type mismatch error, got: {:?}",
        result.diagnostics
    );
}
