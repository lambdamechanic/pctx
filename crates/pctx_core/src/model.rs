use schemars::{JsonSchema, json_schema};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::json;

// -------------- List Functions --------------
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListFunctionsOutput {
    /// Available functions
    pub functions: Vec<ListedFunction>,

    #[serde(skip)]
    pub code: String,
}
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListedFunction {
    /// Namespace the function belongs in
    pub namespace: String,
    /// Function name
    pub name: String,
    /// Function description
    pub description: Option<String>,
}

// -------------- Get Function Details --------------

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetFunctionDetailsInput {
    /// List of functions to get details of.
    pub functions: Vec<FunctionId>,
}

#[derive(Debug, Clone, Default)]
pub struct FunctionId {
    pub mod_name: String,
    pub fn_name: String,
}

impl JsonSchema for FunctionId {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "FunctionId".into()
    }

    fn json_schema(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
        json_schema!({
            "type": "string",
            "description": "Function representation in the form should be in the form '<namespace>.<function name>'. e.g. If there is a function `getData` within the `DataApi` namespace the value provided in this field is DataApi.getData"
        })
    }
}

impl Serialize for FunctionId {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}.{}", self.mod_name, self.fn_name);
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for FunctionId {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let parts: Vec<&str> = s.splitn(2, '.').collect();

        if parts.len() != 2 {
            return Err(serde::de::Error::custom(format!(
                "Expected format '<mod_name>.<fn_name>', got '{}'",
                s
            )));
        }

        Ok(FunctionId {
            mod_name: parts[0].to_string(),
            fn_name: parts[1].to_string(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetFunctionDetailsOutput {
    pub functions: Vec<FunctionDetails>,

    #[serde(skip)]
    pub code: String,
}
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FunctionDetails {
    #[serde(flatten)]
    pub listed: ListedFunction,

    /// typescript input type for the function
    pub input_type: String,
    /// typescript output type for the function
    pub output_type: String,
    /// full typescript type definitions for input/output types
    pub types: String,
}

// -------------- Execute --------------

#[allow(clippy::doc_markdown)]
#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ExecuteInput {
    /// Typescript code to execute.
    ///
    /// REQUIRED FORMAT:
    /// async function ``run()`` {
    ///   // YOUR CODE GOES HERE e.g. const result = await ``Namespace.method();``
    ///   // ALWAYS RETURN THE RESULT e.g. return result;
    /// }
    ///
    /// IMPORTANT: Your code should ONLY contain the function definition.
    /// The sandbox automatically calls run() and exports the result.
    ///
    pub code: String,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ExecuteOutput {
    /// Success of executed code
    pub success: bool,
    /// Standard output of executed code
    pub stdout: String,
    /// Standard error of executed code
    pub stderr: String,
    /// Value returned by executed function
    pub output: Option<serde_json::Value>,
}
impl ExecuteOutput {
    pub fn markdown(&self) -> String {
        format!(
            "Code Executed Successfully: {success}

# Return Value
```json
{return_val}
```

# STDOUT
{stdout}

# STDERR
{stderr}
",
            success = self.success,
            return_val = serde_json::to_string_pretty(&self.output)
                .unwrap_or(json!(&self.output).to_string()),
            stdout = &self.stdout,
            stderr = &self.stderr,
        )
    }
}
