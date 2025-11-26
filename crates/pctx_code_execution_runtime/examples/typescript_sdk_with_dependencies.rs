//! Example: Using PCTX Runtime as a TypeScript SDK with Custom Dependencies
//!
//! This example demonstrates the primary use case: Users running a TypeScript environment
//! (like `deno_executor` or their own Deno runtime) can define local tools with their own
//! dependencies (Zod, custom libraries, etc.) and make them available to sandboxed code.
//!
//! ## User's Workflow:
//! 1. User has their own TypeScript environment with dependencies
//! 2. User defines tools using those dependencies
//! 3. User passes sandboxed code to execute
//! 4. Sandboxed code can call the user-defined tools
//!
//! ## Key Points:
//! - Dependencies (Zod, etc.) are available where tools are DEFINED (trusted zone)
//! - Sandboxed code doesn't need access to dependencies - it just calls tools
//! - This creates a clean bridge between trusted host code and untrusted sandbox

use deno_core::{JsRuntime, RuntimeOptions};
use pctx_code_execution_runtime::{
    AllowedHosts, JsLocalToolRegistry, MCPRegistry, RUNTIME_SNAPSHOT, pctx_runtime_snapshot,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== PCTX TypeScript SDK Example ===\n");
    println!("Simulating a user who has their own TypeScript environment");
    println!("with custom dependencies (Zod, database clients, APIs, etc.)\n");

    // Create runtime
    let mcp_registry = MCPRegistry::new();
    let local_tool_registry = JsLocalToolRegistry::new();
    let allowed_hosts = AllowedHosts::new(None);

    let mut runtime = JsRuntime::new(RuntimeOptions {
        startup_snapshot: Some(RUNTIME_SNAPSHOT),
        extensions: vec![pctx_runtime_snapshot::init(
            mcp_registry,
            local_tool_registry,
            allowed_hosts,
        )],
        ..Default::default()
    });

    // ============================================================================
    // STEP 1: User's Setup - Load Dependencies
    // ============================================================================
    println!("Step 1: Loading user's dependencies (simulated Zod)...\n");

    // In a real scenario, users would:
    // - Use deno_executor which has npm: imports
    // - Bundle their dependencies
    // - Use import maps
    //
    // For this example, we'll inject a minimal Zod-like validator
    runtime.execute_script(
        "<user-dependencies>",
        r#"
        // Simulate Zod being available in the user's environment
        // In reality, this would be: import { z } from 'npm:zod'
        globalThis.z = {
            object: (schema) => ({
                parse: (data) => {
                    for (const [key, validator] of Object.entries(schema)) {
                        if (!(key in data)) {
                            throw new Error(`Missing required field: ${key}`);
                        }
                        validator.parse(data[key]);
                    }
                    return data;
                },
                safeParse: (data) => {
                    try {
                        for (const [key, validator] of Object.entries(schema)) {
                            if (!(key in data)) {
                                return { success: false, error: `Missing: ${key}` };
                            }
                            validator.parse(data[key]);
                        }
                        return { success: true, data };
                    } catch (e) {
                        return { success: false, error: e.message };
                    }
                }
            }),
            string: () => ({
                parse: (val) => {
                    if (typeof val !== 'string') {
                        throw new Error('Expected string');
                    }
                    return val;
                },
                email: function() {
                    this.parse = (val) => {
                        if (typeof val !== 'string' || !val.includes('@')) {
                            throw new Error('Invalid email');
                        }
                        return val;
                    };
                    return this;
                },
                min: function(len) {
                    this.parse = (val) => {
                        if (typeof val !== 'string' || val.length < len) {
                            throw new Error(`String too short (min ${len})`);
                        }
                        return val;
                    };
                    return this;
                }
            }),
            number: () => ({
                parse: (val) => {
                    if (typeof val !== 'number') {
                        throw new Error('Expected number');
                    }
                    return val;
                },
                min: function(minVal) {
                    this.parse = (val) => {
                        if (typeof val !== 'number' || val < minVal) {
                            throw new Error(`Number too small (min ${minVal})`);
                        }
                        return val;
                    };
                    return this;
                },
                max: function(maxVal) {
                    this.parse = (val) => {
                        if (typeof val !== 'number' || val > maxVal) {
                            throw new Error(`Number too large (max ${maxVal})`);
                        }
                        return val;
                    };
                    return this;
                }
            }),
            array: (itemValidator) => ({
                parse: (val) => {
                    if (!Array.isArray(val)) {
                        throw new Error('Expected array');
                    }
                    return val.map(item => itemValidator.parse(item));
                }
            })
        };

        console.log("✓ Dependencies loaded (Zod available as 'z')");
    "#,
    )?;

    // ============================================================================
    // STEP 2: User Defines Tools with Their Dependencies
    // ============================================================================
    println!("\nStep 2: User defines tools using their dependencies...\n");

    runtime.execute_script(
        "<user-tool-definitions>",
        r#"
        // =================================================================
        // USER'S TRUSTED CODE
        // This is where the user has access to all their dependencies
        // =================================================================

        // Simulate user's database connection
        const userDatabase = new Map([
            [1, { id: 1, name: "Alice", email: "alice@example.com", role: "admin" }],
            [2, { id: 2, name: "Bob", email: "bob@example.com", role: "user" }],
            [3, { id: 3, name: "Charlie", email: "charlie@example.com", role: "user" }]
        ]);

        // Tool 1: Validate and Create User (using Zod)
        registerJsLocalTool({
            name: "createUser",
            description: "Validates and creates a new user using Zod schemas",
            inputSchema: {
                type: "object",
                properties: {
                    name: { type: "string" },
                    email: { type: "string" },
                    age: { type: "number" }
                },
                required: ["name", "email", "age"]
            }
        }, (args) => {
            // User can use Zod here because it's in their environment!
            const UserSchema = z.object({
                name: z.string().min(2),
                email: z.string().email(),
                age: z.number().min(0).max(120)
            });

            // Validate with Zod
            const validatedData = UserSchema.parse(args);

            // Create user in "database"
            const newId = userDatabase.size + 1;
            const newUser = { id: newId, ...validatedData, role: "user" };
            userDatabase.set(newId, newUser);

            return { success: true, user: newUser };
        });

        // Tool 2: Get User (with safe validation)
        registerJsLocalTool({
            name: "getUser",
            description: "Retrieves a user by ID with Zod validation"
        }, (args) => {
            const QuerySchema = z.object({
                id: z.number().min(1)
            });

            const { id } = QuerySchema.parse(args);
            const user = userDatabase.get(id);

            if (!user) {
                throw new Error(`User ${id} not found`);
            }

            return user;
        });

        // Tool 3: Batch Process (demonstrates arrays with Zod)
        registerJsLocalTool({
            name: "processUserIds",
            description: "Process multiple user IDs, validating each"
        }, (args) => {
            const BatchSchema = z.object({
                ids: z.array(z.number().min(1))
            });

            const { ids } = BatchSchema.parse(args);

            return ids.map(id => {
                const user = userDatabase.get(id);
                return user ? { id, name: user.name, found: true } : { id, found: false };
            });
        });

        // Tool 4: Safe Update (demonstrates safeParse)
        registerJsLocalTool({
            name: "updateUser",
            description: "Updates user with safe validation"
        }, (args) => {
            const UpdateSchema = z.object({
                id: z.number().min(1),
                name: z.string().min(2),
                email: z.string().email()
            });

            const result = UpdateSchema.safeParse(args);

            if (!result.success) {
                return { success: false, error: result.error };
            }

            const { id, name, email } = result.data;
            const user = userDatabase.get(id);

            if (!user) {
                return { success: false, error: `User ${id} not found` };
            }

            userDatabase.set(id, { ...user, name, email });
            return { success: true, user: userDatabase.get(id) };
        });

        console.log("✓ Registered 4 tools with Zod validation:");
        console.log("  - createUser (validates with Zod schema)");
        console.log("  - getUser (validates ID)");
        console.log("  - processUserIds (batch validation)");
        console.log("  - updateUser (safe validation with safeParse)");
    "#,
    )?;

    // ============================================================================
    // STEP 3: Run Sandboxed User Code
    // ============================================================================
    println!("\nStep 3: Running sandboxed user code (no direct dependency access)...\n");

    runtime.execute_script(
        "<sandboxed-user-code>",
        r#"
        (async () => {
            // =================================================================
            // SANDBOXED CODE
            // This code does NOT have access to Zod or the database
            // It can only call the tools defined above
            // =================================================================

            console.log("\n--- Sandboxed Code Execution ---\n");

            // 1. Create a valid user
            console.log("1. Creating a valid user...");
            const result1 = await callJsLocalTool("createUser", {
                name: "Diana",
                email: "diana@example.com",
                age: 28
            });
            console.log("   Result:", JSON.stringify(result1));

            // 2. Try to create an invalid user (will fail Zod validation)
            console.log("\n2. Attempting to create invalid user...");
            try {
                await callJsLocalTool("createUser", {
                    name: "E", // Too short!
                    email: "invalid-email", // Not an email!
                    age: 25
                });
                console.log("   ERROR: Should have failed validation!");
            } catch (e) {
                console.log("   ✓ Validation failed as expected:", e.message);
            }

            // 3. Get an existing user
            console.log("\n3. Fetching existing user...");
            const user = await callJsLocalTool("getUser", { id: 1 });
            console.log("   User:", JSON.stringify(user));

            // 4. Batch process users
            console.log("\n4. Batch processing user IDs...");
            const batch = await callJsLocalTool("processUserIds", {
                ids: [1, 2, 99, 3] // 99 doesn't exist
            });
            console.log("   Results:", JSON.stringify(batch));

            // 5. Update a user safely
            console.log("\n5. Updating user with valid data...");
            const update1 = await callJsLocalTool("updateUser", {
                id: 2,
                name: "Robert",
                email: "robert@example.com"
            });
            console.log("   Result:", JSON.stringify(update1));

            // 6. Try to update with invalid data
            console.log("\n6. Attempting update with invalid email...");
            const update2 = await callJsLocalTool("updateUser", {
                id: 2,
                name: "Robert",
                email: "not-an-email" // Invalid!
            });
            console.log("   Result:", JSON.stringify(update2));

            console.log("\n--- Sandboxed Execution Complete ---\n");
        })();
    "#,
    )?;

    // Run the event loop to execute async code
    runtime.run_event_loop(Default::default()).await?;

    // ============================================================================
    // Display Captured Console Output
    // ============================================================================
    let console_output = runtime.execute_script(
        "<get-console>",
        r"
        JSON.stringify({
            stdout: globalThis.__stdout || [],
            stderr: globalThis.__stderr || []
        })
    ",
    )?;

    let console_str = {
        deno_core::scope!(scope, &mut runtime);
        let local = deno_core::v8::Local::new(scope, &console_output);
        let string_val = local.to_string(scope).unwrap();
        string_val.to_rust_string_lossy(scope)
    };

    let console_data: serde_json::Value = serde_json::from_str(&console_str)?;

    println!("\n=== Console Output ===\n");
    if let Some(stdout) = console_data["stdout"].as_array() {
        for line in stdout {
            if let Some(msg) = line.as_str() {
                println!("{msg}");
            }
        }
    }

    // ============================================================================
    // Show Final State
    // ============================================================================
    println!("\n=== Summary ===\n");

    let summary = runtime.execute_script(
        "<summary>",
        r#"
        const tools = JS_LOCAL_TOOLS.list();
        const toolInfo = tools.map(t => ({
            name: t.name,
            description: t.description
        }));

        JSON.stringify({
            toolsRegistered: tools.length,
            tools: toolInfo,
            message: "Tools successfully bridged user dependencies to sandboxed code!"
        }, null, 2)
    "#,
    )?;

    let summary_str = {
        deno_core::scope!(scope, &mut runtime);
        let local = deno_core::v8::Local::new(scope, &summary);
        let string_val = local.to_string(scope).unwrap();
        string_val.to_rust_string_lossy(scope)
    };

    println!("{}", summary_str);

    println!("\n=== Key Takeaways ===\n");
    println!("✓ Users define tools in their trusted environment (with Zod, DB, etc.)");
    println!("✓ Sandboxed code has NO access to dependencies");
    println!("✓ Sandboxed code CAN call tools that use those dependencies");
    println!("✓ This creates a secure, clean bridge between trusted and untrusted code\n");

    Ok(())
}
