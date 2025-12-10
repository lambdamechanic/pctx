Get detailed information about specific functions you want to use.

WHEN TO USE: After calling list_functions(), use this to learn about parameter types, return values, and usage for specific functions.

REQUIRED FORMAT: Functions must be specified as 'namespace.functionName' (e.g., 'Namespace.apiPostSearch')

This tool is lightweight and only returns details for the functions you request, avoiding unnecessary token usage.
Only request details for functions you actually plan to use in your code.

NOTE ON RETURN TYPES:
- If a function returns Promise<any>, the MCP server didn't provide an output schema
- The actual value is a parsed object (not a string) - access properties directly
- Don't use JSON.parse() on the results - they're already JavaScript objects
