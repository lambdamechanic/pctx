"""Tests for the @tool decorator in pctx_py.tools.convert"""

from __future__ import annotations

import pytest
from pctx_py.tools.convert import tool
from pctx_py.tools.tool import Tool


# Basic functionality tests
def test_decorator_without_arguments() -> None:
    """Test @tool decorator used without parentheses"""

    @tool
    def simple_function() -> str:
        """A simple test function"""
        return "result"

    assert isinstance(simple_function, Tool)
    assert simple_function.name == "simple_function"
    assert simple_function.description == "A simple test function"
    assert simple_function.func is not None


def test_decorator_with_custom_name() -> None:
    """Test @tool decorator with custom name argument"""

    @tool("custom_name")
    def my_function() -> str:
        """Function with custom name"""
        return "result"

    assert isinstance(my_function, Tool)
    assert my_function.name == "custom_name"
    assert my_function.description == "Function with custom name"


def test_decorator_with_custom_description() -> None:
    """Test @tool decorator with custom description"""

    @tool("tool_name", description="Custom description here")
    def my_function() -> str:
        """Original docstring"""
        return "result"

    assert isinstance(my_function, Tool)
    assert my_function.name == "tool_name"
    assert my_function.description == "Custom description here"


def test_function_name_extraction() -> None:
    """Test that function names are correctly extracted"""

    @tool
    def calculate_sum() -> str:
        return "42"

    assert calculate_sum.name == "calculate_sum"


# Synchronous function tests


def test_sync_function_stored_correctly() -> None:
    """Test that sync functions are stored in func attribute"""

    @tool
    def sync_tool() -> str:
        """Sync function"""
        return "sync result"

    assert isinstance(sync_tool, Tool)
    assert sync_tool.func is not None
    assert sync_tool.coroutine is None
    assert sync_tool.func() == "sync result"


def test_sync_function_with_parameters() -> None:
    """Test sync function that accepts parameters"""

    @tool
    def add_numbers(a: int, b: int) -> str:
        """Adds two numbers"""
        return str(a + b)

    assert add_numbers.func is not None
    assert add_numbers.func(5, 3) == "8"


def test_sync_function_with_kwargs() -> None:
    """Test sync function with keyword arguments"""

    @tool
    def greet(name: str, greeting: str = "Hello") -> str:
        """Greets a person"""
        return f"{greeting}, {name}!"

    assert greet.func is not None
    assert greet.func("Alice") == "Hello, Alice!"
    assert greet.func("Bob", greeting="Hi") == "Hi, Bob!"


# Asynchronous function tests


@pytest.mark.asyncio
async def test_async_function_stored_correctly() -> None:
    """Test that async functions are stored in coroutine attribute"""

    @tool
    async def async_tool() -> str:
        """Async function"""
        return "async result"

    assert isinstance(async_tool, Tool)
    assert async_tool.func is None
    assert async_tool.coroutine is not None
    result = await async_tool.coroutine()
    assert result == "async result"


@pytest.mark.asyncio
async def test_async_function_with_parameters() -> None:
    """Test async function with parameters"""

    @tool
    async def fetch_data(url: str, timeout: int = 30) -> str:
        """Fetches data from URL"""
        return f"Data from {url} with timeout {timeout}"

    assert fetch_data.coroutine is not None
    result = await fetch_data.coroutine("https://example.com", timeout=60)
    assert result == "Data from https://example.com with timeout 60"


@pytest.mark.asyncio
async def test_async_function_with_custom_name() -> None:
    """Test async function with custom name"""

    @tool("async_fetcher")
    async def my_async_function() -> str:
        """Async function with custom name"""
        return "fetched"

    assert my_async_function.name == "async_fetcher"
    assert my_async_function.coroutine is not None


# Docstring handling tests


def test_docstring_becomes_description() -> None:
    """Test that function docstring becomes tool description"""

    @tool
    def documented_function() -> str:
        """This is a detailed description
        of what the function does."""
        return "result"

    assert "This is a detailed description" in documented_function.description
    assert "of what the function does." in documented_function.description


def test_indented_docstring_dedented() -> None:
    """Test that indented docstrings are properly dedented"""

    @tool
    def indented_doc() -> str:
        """
        First line
            Indented line
        Last line
        """
        return "result"

    # Verify dedenting occurred (no extra leading spaces)
    lines = indented_doc.description.strip().split("\n")
    assert lines[0] == "First line"
    assert "    Indented line" in indented_doc.description


def test_no_docstring_empty_description() -> None:
    """Test function without docstring has empty description"""

    @tool
    def no_doc() -> str:
        return "result"

    assert no_doc.description == ""


def test_custom_description_overrides_docstring() -> None:
    """Test that custom description overrides docstring"""

    @tool("func", description="Custom")
    def with_docstring() -> str:
        """Original docstring"""
        return "result"

    assert with_docstring.description == "Custom"


# Error handling tests


def test_too_many_arguments_raises_error() -> None:
    """Test that providing too many arguments raises ValueError"""

    with pytest.raises(ValueError, match="Too many arguments"):

        @tool("name", "extra_arg")
        def bad_function() -> str:
            return "result"


def test_invalid_first_argument_raises_error() -> None:
    """Test that invalid first argument raises ValueError"""

    with pytest.raises(
        ValueError, match="must be a string or a callable with a __name__"
    ):
        tool(123)  # type: ignore


def test_object_without_name_attribute_raises_error() -> None:
    """Test that callable without __name__ raises ValueError"""

    class CallableWithoutName:
        def __call__(self) -> str:
            return "result"

    obj = CallableWithoutName()
    with pytest.raises(
        ValueError, match="must be a string or a callable with a __name__"
    ):
        tool(obj)  # type: ignore


# Integration tests


def test_multiple_tools_independent() -> None:
    """Test that multiple decorated functions are independent"""

    @tool
    def tool_one() -> str:
        """First tool"""
        return "one"

    @tool
    def tool_two() -> str:
        """Second tool"""
        return "two"

    assert tool_one.name == "tool_one"
    assert tool_two.name == "tool_two"
    assert tool_one.func is not None
    assert tool_two.func is not None
    assert tool_one.func() == "one"
    assert tool_two.func() == "two"


def test_tool_is_tool_instance() -> None:
    """Test that decorated function is an instance of Tool"""

    @tool
    def my_tool() -> str:
        return "result"

    assert isinstance(my_tool, Tool)


def test_decorated_function_has_required_attributes() -> None:
    """Test that decorated function has all required Tool attributes"""

    @tool("test_tool", description="Test description")
    def my_function(x: int) -> str:
        return str(x)

    assert hasattr(my_function, "name")
    assert hasattr(my_function, "description")
    assert hasattr(my_function, "func")
    assert hasattr(my_function, "coroutine")
    assert my_function.name == "test_tool"
    assert my_function.description == "Test description"
