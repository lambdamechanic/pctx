"""
Test suite for pctx_code_mode Python bindings.
"""

import pytest
import asyncio
import json


def test_import():
    """Test that the module can be imported."""
    import pctx_code_mode
    assert pctx_code_mode is not None


def test_code_mode_empty_init():
    """Test CodeMode can be initialized with no arguments."""
    from pctx_code_mode import CodeMode

    cm = CodeMode()
    assert cm is not None


def test_code_mode_init_with_local_tools():
    """Test CodeMode initialization with local tools."""
    from pctx_code_mode import CodeMode

    def add_callback(params):
        return {"result": params["a"] + params["b"]}

    cm = CodeMode(
        local_tools=[
            {
                "namespace": "math",
                "name": "add",
                "description": "Adds two numbers",
                "callback": add_callback,
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "a": {"type": "number"},
                        "b": {"type": "number"}
                    },
                    "required": ["a", "b"]
                }
            }
        ]
    )
    assert cm is not None


def test_register_local_tool():
    """Test registering a local tool after initialization."""
    from pctx_code_mode import CodeMode

    cm = CodeMode()

    def multiply_callback(params):
        return {"result": params["a"] * params["b"]}

    cm.register_local_tool(
        namespace="math",
        name="multiply",
        description="Multiplies two numbers",
        callback=multiply_callback,
        input_schema={
            "type": "object",
            "properties": {
                "a": {"type": "number"},
                "b": {"type": "number"}
            }
        }
    )


def test_register_local_tool_optional_description():
    """Test registering a local tool with optional description."""
    from pctx_code_mode import CodeMode

    cm = CodeMode()

    cm.register_local_tool(
        namespace="utils",
        name="helper",
        callback=lambda params: {"ok": True},
        input_schema={"type": "object"}
    )


def test_register_local_tool_optional_schema():
    """Test registering a local tool with optional input_schema."""
    from pctx_code_mode import CodeMode

    cm = CodeMode()

    cm.register_local_tool(
        namespace="utils",
        name="simple",
        description="A simple tool",
        callback=lambda params: {"ok": True}
    )


def test_list_functions_empty():
    """Test list_functions on empty CodeMode."""
    from pctx_code_mode import CodeMode

    cm = CodeMode()
    result = cm.list_functions()

    assert hasattr(result, 'functions')
    assert hasattr(result, 'code')
    assert isinstance(result.functions, list)
    assert len(result.functions) == 0


def test_list_functions_with_local_tools():
    """Test list_functions with registered local tools."""
    from pctx_code_mode import CodeMode

    cm = CodeMode()
    cm.register_local_tool(
        namespace="math",
        name="add",
        description="Adds two numbers",
        callback=lambda params: {"result": params["a"] + params["b"]},
        input_schema={"type": "object"}
    )

    result = cm.list_functions()

    assert len(result.functions) == 1
    func = result.functions[0]
    assert func.namespace == "Math"
    assert func.name == "add"
    assert func.description == "Adds two numbers"
    assert isinstance(result.code, str)
    assert len(result.code) > 0


def test_list_functions_multiple_namespaces():
    """Test list_functions with tools in multiple namespaces."""
    from pctx_code_mode import CodeMode

    cm = CodeMode()
    cm.register_local_tool(
        namespace="math",
        name="add",
        description="Adds",
        callback=lambda params: {"result": params["a"] + params["b"]},
    )
    cm.register_local_tool(
        namespace="string",
        name="concat",
        description="Concatenates",
        callback=lambda params: {"result": params["a"] + params["b"]},
    )

    result = cm.list_functions()

    assert len(result.functions) == 2
    namespaces = {f.namespace for f in result.functions}
    assert namespaces == {"Math", "String"}


def test_list_functions_same_namespace():
    """Test list_functions with multiple tools in same namespace."""
    from pctx_code_mode import CodeMode

    cm = CodeMode()
    cm.register_local_tool(
        namespace="math",
        name="add",
        callback=lambda params: {"result": params["a"] + params["b"]},
    )
    cm.register_local_tool(
        namespace="math",
        name="multiply",
        callback=lambda params: {"result": params["a"] * params["b"]},
    )

    result = cm.list_functions()

    assert len(result.functions) == 2
    math_funcs = [f for f in result.functions if f.namespace == "Math"]
    assert len(math_funcs) == 2
    names = {f.name for f in math_funcs}
    assert names == {"add", "multiply"}


def test_get_function_details_empty():
    """Test get_function_details with empty function list."""
    from pctx_code_mode import CodeMode

    cm = CodeMode()
    result = cm.get_function_details([])

    assert hasattr(result, 'functions')
    assert hasattr(result, 'code')
    assert len(result.functions) == 0


def test_get_function_details_single():
    """Test get_function_details for a single function."""
    from pctx_code_mode import CodeMode

    cm = CodeMode()
    cm.register_local_tool(
        namespace="math",
        name="add",
        description="Adds two numbers",
        callback=lambda params: {"result": params["a"] + params["b"]},
        input_schema={
            "type": "object",
            "properties": {
                "a": {"type": "number"},
                "b": {"type": "number"}
            }
        }
    )

    result = cm.get_function_details(["Math.add"])

    assert len(result.functions) == 1
    func = result.functions[0]
    assert func.namespace == "Math"
    assert func.name == "add"
    assert func.description == "Adds two numbers"
    assert isinstance(func.input_type, str)
    assert isinstance(func.output_type, str)
    assert isinstance(func.types, str)
    assert isinstance(result.code, str)


def test_get_function_details_multiple():
    """Test get_function_details for multiple functions."""
    from pctx_code_mode import CodeMode

    cm = CodeMode()
    cm.register_local_tool(
        namespace="math",
        name="add",
        callback=lambda params: {"result": params["a"] + params["b"]},
    )
    cm.register_local_tool(
        namespace="math",
        name="multiply",
        callback=lambda params: {"result": params["a"] * params["b"]},
    )

    result = cm.get_function_details(["Math.add", "Math.multiply"])

    assert len(result.functions) == 2


@pytest.mark.asyncio
async def test_execute_simple():
    """Test executing simple TypeScript code."""
    from pctx_code_mode import CodeMode

    cm = CodeMode()

    code = """
    async function run() {
        return {message: "hello"};
    }
    """

    result = await cm.execute(code)

    assert hasattr(result, 'success')
    assert hasattr(result, 'stdout')
    assert hasattr(result, 'stderr')
    assert hasattr(result, 'output')
    assert result.success is True
    assert result.output is not None
    assert result.output.get("message") == "hello"


@pytest.mark.asyncio
async def test_execute_with_local_tool():
    """Test executing code that calls a local tool."""
    from pctx_code_mode import CodeMode

    cm = CodeMode()
    cm.register_local_tool(
        namespace="math",
        name="add",
        description="Adds two numbers",
        callback=lambda params: {"result": params["a"] + params["b"]},
        input_schema={
            "type": "object",
            "properties": {
                "a": {"type": "number"},
                "b": {"type": "number"}
            }
        }
    )

    code = """
    async function run() {
        const result = await Math.add({a: 5, b: 3});
        return result;
    }
    """

    result = await cm.execute(code)

    assert result.success is True
    assert result.output is not None
    assert result.output.get("result") == 8


@pytest.mark.asyncio
async def test_execute_with_error():
    """Test executing code that throws an error."""
    from pctx_code_mode import CodeMode

    cm = CodeMode()

    code = """
    async function run() {
        throw new Error("Test error");
    }
    """

    result = await cm.execute(code)

    assert result.success is False
    assert len(result.stderr) > 0


@pytest.mark.asyncio
async def test_add_mcp_server():
    """Test adding an MCP server (will require actual server to fully test)."""
    from pctx_code_mode import CodeMode

    cm = CodeMode()

    # This will fail without a real server, but tests the API
    with pytest.raises(Exception):  # Connection error expected
        await cm.add_mcp_server(
            name="test_server",
            url="http://localhost:9999"
        )


def test_duplicate_tool_name_in_namespace():
    """Test that duplicate tool names in same namespace raise an error."""
    from pctx_code_mode import CodeMode

    cm = CodeMode()
    cm.register_local_tool(
        namespace="math",
        name="add",
        callback=lambda params: {"result": 1},
    )

    with pytest.raises(Exception):  # Should raise error for duplicate
        cm.register_local_tool(
            namespace="math",
            name="add",
            callback=lambda params: {"result": 2},
        )


def test_callback_return_types():
    """Test that callbacks can return different types."""
    from pctx_code_mode import CodeMode

    cm = CodeMode()

    # Dict return
    cm.register_local_tool(
        namespace="test",
        name="dict_return",
        callback=lambda params: {"key": "value"},
    )

    # Number return
    cm.register_local_tool(
        namespace="test",
        name="number_return",
        callback=lambda params: 42,
    )

    # String return
    cm.register_local_tool(
        namespace="test",
        name="string_return",
        callback=lambda params: "hello",
    )

    # List return
    cm.register_local_tool(
        namespace="test",
        name="list_return",
        callback=lambda params: [1, 2, 3],
    )


def test_callback_with_none_params():
    """Test callbacks can handle None params."""
    from pctx_code_mode import CodeMode

    cm = CodeMode()

    def no_params_callback(params):
        # params might be None
        return {"called": True}

    cm.register_local_tool(
        namespace="test",
        name="no_params",
        callback=no_params_callback,
    )