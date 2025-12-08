"""Tests for the @tool decorator in pctx_py.tools.convert"""

from __future__ import annotations

import pytest
from pctx import tool, Tool


# ============================================================================
# SECTION 1: REGISTRATION TESTS
# Tests for Tool attributes: name, description, args_schema, func, coroutine
# ============================================================================


def test_registration_basic_sync_function() -> None:
    """Test basic tool registration with sync function"""

    @tool
    def simple_function() -> str:
        """A simple test function"""
        return "result"

    assert isinstance(simple_function, Tool)
    assert simple_function.name == "simple_function"
    assert simple_function.description == "A simple test function"
    assert simple_function.func is not None
    assert simple_function.coroutine is None
    assert simple_function.input_schema is None


def test_registration_basic_async_function() -> None:
    """Test basic tool registration with async function"""

    @tool
    async def async_function() -> str:
        """An async test function"""
        return "async result"

    assert isinstance(async_function, Tool)
    assert async_function.name == "async_function"
    assert async_function.description == "An async test function"
    assert async_function.func is None
    assert async_function.coroutine is not None
    assert async_function.input_schema is None


def test_registration_custom_name() -> None:
    """Test tool registration with custom name"""

    @tool("custom_name")
    def my_function() -> str:
        """Function with custom name"""
        return "result"

    assert my_function.name == "custom_name"
    assert my_function.description == "Function with custom name"


def test_registration_custom_description() -> None:
    """Test tool registration with custom description"""

    @tool("tool_name", description="Custom description here")
    def my_function() -> str:
        """Original docstring"""
        return "result"

    assert my_function.name == "tool_name"
    assert my_function.description == "Custom description here"


def test_registration_with_parameters() -> None:
    """Test tool registration with function parameters in args_schema"""

    @tool
    def add_numbers(a: int, b: int, c: str = "default") -> str:
        """Adds two numbers"""
        return str(a + b)

    # Check args_schema includes parameters
    assert (
        add_numbers.input_schema is not None
        and add_numbers.input_schema.model_json_schema()
        == {
            "title": "add_numbers_Input",
            "type": "object",
            "required": ["a", "b"],
            "properties": {
                "a": {"title": "A", "type": "integer"},
                "b": {"title": "B", "type": "integer"},
                "c": {"title": "C", "type": "string", "default": "default"},
            },
            "additionalProperties": False,
        }
    )


def test_registration_docstring_becomes_description() -> None:
    """Test that function docstring becomes tool description"""

    @tool
    def documented_function() -> str:
        """This is a detailed description
        of what the function does."""
        return "result"

    assert "This is a detailed description" in documented_function.description
    assert "of what the function does." in documented_function.description


def test_registration_indented_docstring_dedented() -> None:
    """Test that indented docstrings are properly dedented"""

    @tool
    def indented_doc() -> str:
        """
        First line
            Indented line
        Last line
        """
        return "result"

    lines = indented_doc.description.strip().split("\n")
    assert lines[0] == "First line"
    assert "    Indented line" in indented_doc.description


def test_registration_no_docstring() -> None:
    """Test function without docstring has empty description"""

    @tool
    def no_doc() -> str:
        return "result"

    assert no_doc.description == ""


def test_registration_custom_description_overrides_docstring() -> None:
    """Test that custom description overrides docstring"""

    @tool("func", description="Custom")
    def with_docstring() -> str:
        """Original docstring"""
        return "result"

    assert with_docstring.description == "Custom"


def test_registration_multipletools_independent() -> None:
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
    assert tool_one.description == "First tool"
    assert tool_two.description == "Second tool"


def test_registration_error_too_many_arguments() -> None:
    """Test that providing too many arguments raises ValueError"""

    with pytest.raises(ValueError, match="Too many arguments"):

        @tool("name", "extra_arg")
        def bad_function() -> str:
            return "result"


def test_registration_error_invalid_first_argument() -> None:
    """Test that invalid first argument raises ValueError"""

    with pytest.raises(
        ValueError, match="must be a string or a callable with a __name__"
    ):
        tool(123)  # type: ignore


def test_registration_error_callable_without_name() -> None:
    """Test that callable without __name__ raises ValueError"""

    class CallableWithoutName:
        def __call__(self) -> str:
            return "result"

    obj = CallableWithoutName()
    with pytest.raises(
        ValueError, match="must be a string or a callable with a __name__"
    ):
        tool(obj)  # type: ignore


# ============================================================================
# SECTION 2: CALLING FUNCTIONS
# Tests for actually calling the registered sync and async functions
# ============================================================================


def test_calling_sync_function_no_parameters() -> None:
    """Test calling sync function with no parameters"""

    @tool
    def synctool() -> str:
        """Sync function"""
        return "sync result"

    assert synctool.func is not None
    result = synctool.invoke()
    assert result == {"data": "sync result"}


def test_calling_sync_function_with_positional_args() -> None:
    """Test calling sync function with positional arguments"""

    @tool
    def add_numbers(a: int, b: int) -> str:
        """Adds two numbers"""
        return str(a + b)

    assert add_numbers.func is not None
    result = add_numbers.invoke(a=5, b=3)
    assert result == {"data": "8"}


def test_calling_sync_function_with_kwargs() -> None:
    """Test calling sync function with keyword arguments"""

    @tool
    def greet(name: str, greeting: str = "Hello") -> str:
        """Greets a person"""
        return f"{greeting}, {name}!"

    assert greet.func is not None

    # Test with default
    result1 = greet.invoke(name="Alice")
    assert result1 == {"data": "Hello, Alice!"}

    # Test with custom kwarg
    result2 = greet.invoke(name="Bob", greeting="Hi")
    assert result2 == {"data": "Hi, Bob!"}


def test_calling_sync_function_with_mixed_args() -> None:
    """Test calling sync function with both positional and keyword arguments"""

    @tool
    def process(x: int, y: int, multiplier: int = 2) -> str:
        """Process two numbers"""
        return str((x + y) * multiplier)

    assert process.func is not None

    # Test with default multiplier
    result1 = process.invoke(x=3, y=4)
    assert result1 == {"data": "14"}  # (3 + 4) * 2

    # Test with custom multiplier
    result2 = process.invoke(x=3, y=4, multiplier=3)
    assert result2 == {"data": "21"}  # (3 + 4) * 3


@pytest.mark.asyncio
async def test_calling_async_function_no_parameters() -> None:
    """Test calling async function with no parameters"""

    @tool
    async def asynctool() -> str:
        """Async function"""
        return "async result"

    assert asynctool.coroutine is not None
    result = await asynctool.ainvoke()
    assert result == {"data": "async result"}


@pytest.mark.asyncio
async def test_calling_async_function_with_parameters() -> None:
    """Test calling async function with parameters"""

    @tool
    async def fetch_data(url: str, timeout: int = 30) -> str:
        """Fetches data from URL"""
        return f"Data from {url} with timeout {timeout}"

    assert fetch_data.coroutine is not None

    # Test with custom timeout
    result = await fetch_data.ainvoke(url="https://example.com", timeout=60)
    assert result == {"data": "Data from https://example.com with timeout 60"}


@pytest.mark.asyncio
async def test_calling_async_function_with_defaults() -> None:
    """Test calling async function using default parameters"""

    @tool
    async def fetch_data(url: str, timeout: int = 30, retries: int = 3) -> str:
        """Fetches data from URL"""
        return f"URL: {url}, timeout: {timeout}, retries: {retries}"

    assert fetch_data.coroutine is not None

    # Test with all defaults
    result1 = await fetch_data.ainvoke(url="https://test.com")
    assert result1 == {"data": "URL: https://test.com, timeout: 30, retries: 3"}

    # Test with partial kwargs
    result2 = await fetch_data.ainvoke(url="https://test.com", retries=5)
    assert result2 == {"data": "URL: https://test.com, timeout: 30, retries: 5"}


def test_calling_sync_function_multiple_calls() -> None:
    """Test that sync function can be called multiple times"""

    call_count = 0

    @tool
    def counter() -> str:
        nonlocal call_count
        call_count += 1
        return f"Call {call_count}"

    assert counter.func is not None

    assert counter.invoke() == {"data": "Call 1"}
    assert counter.invoke() == {"data": "Call 2"}
    assert counter.invoke() == {"data": "Call 3"}


@pytest.mark.asyncio
async def test_calling_async_function_multiple_calls() -> None:
    """Test that async function can be called multiple times"""

    call_count = 0

    @tool
    async def async_counter() -> str:
        nonlocal call_count
        call_count += 1
        return f"Async call {call_count}"

    assert async_counter.coroutine is not None

    assert await async_counter.ainvoke() == {"data": "Async call 1"}
    assert await async_counter.ainvoke() == {"data": "Async call 2"}
    assert await async_counter.ainvoke() == {"data": "Async call 3"}


# ============================================================================
# SECTION 3: VALIDATION TESTS
# Tests for input validation with invoke/ainvoke methods
# ============================================================================


def test_validation_missing_required_parameter() -> None:
    """Test that missing required parameters raise ValidationError"""
    from pydantic import ValidationError

    @tool
    def add_numbers(a: int, b: int) -> str:
        """Adds two numbers"""
        return str(a + b)

    # Missing parameter 'b'
    with pytest.raises(ValidationError) as exc_info:
        add_numbers.invoke(a=5)

    assert "b" in str(exc_info.value)


def test_validation_wrong_type_parameter() -> None:
    """Test that wrong type parameters raise ValidationError"""
    from pydantic import ValidationError

    @tool
    def add_numbers(a: int, b: int) -> str:
        """Adds two numbers"""
        return str(a + b)

    # Wrong type for parameter 'b'
    with pytest.raises(ValidationError) as exc_info:
        add_numbers.invoke(a=5, b="not_an_int")

    assert (
        "b" in str(exc_info.value).lower()
        or "validation" in str(exc_info.value).lower()
    )


def test_validation_extra_parameter() -> None:
    """Test that extra parameters raise ValidationError"""
    from pydantic import ValidationError

    @tool
    def add_numbers(a: int, b: int) -> str:
        """Adds two numbers"""
        return str(a + b)

    # Extra parameter 'c' not defined in schema
    with pytest.raises(ValidationError) as exc_info:
        add_numbers.invoke(a=5, b=3, c=10)

    assert "extra" in str(exc_info.value).lower() or "c" in str(exc_info.value).lower()


def test_validation_valid_input_with_defaults() -> None:
    """Test that valid input with defaults passes validation"""

    @tool
    def greet(name: str, greeting: str = "Hello") -> str:
        """Greets a person"""
        return f"{greeting}, {name}!"

    # Should not raise any validation error
    result = greet.invoke(name="Alice")
    assert result == {"data": "Hello, Alice!"}


def test_validation_valid_input_all_parameters() -> None:
    """Test that valid input with all parameters passes validation"""

    @tool
    def process(x: int, y: int, multiplier: int = 2) -> str:
        """Process two numbers"""
        return str((x + y) * multiplier)

    # Should not raise any validation error
    result = process.invoke(x=3, y=4, multiplier=5)
    assert result == {"data": "35"}


@pytest.mark.asyncio
async def test_validation_async_missing_required_parameter() -> None:
    """Test that async functions validate missing required parameters"""
    from pydantic import ValidationError

    @tool
    async def fetch_data(url: str, timeout: int = 30) -> str:
        """Fetches data from URL"""
        return f"Data from {url} with timeout {timeout}"

    # Missing required parameter 'url'
    with pytest.raises(ValidationError) as exc_info:
        await fetch_data.ainvoke(timeout=60)

    assert "url" in str(exc_info.value)


@pytest.mark.asyncio
async def test_validation_async_wrong_type_parameter() -> None:
    """Test that async functions validate parameter types"""
    from pydantic import ValidationError

    @tool
    async def fetch_data(url: str, timeout: int = 30) -> str:
        """Fetches data from URL"""
        return f"Data from {url} with timeout {timeout}"

    # Wrong type for 'timeout' parameter
    with pytest.raises(ValidationError) as exc_info:
        await fetch_data.ainvoke(url="https://example.com", timeout="not_an_int")

    assert (
        "timeout" in str(exc_info.value).lower()
        or "validation" in str(exc_info.value).lower()
    )


@pytest.mark.asyncio
async def test_validation_async_valid_input() -> None:
    """Test that async functions accept valid input"""

    @tool
    async def fetch_data(url: str, timeout: int = 30) -> str:
        """Fetches data from URL"""
        return f"Data from {url} with timeout {timeout}"

    # Should not raise any validation error
    result = await fetch_data.ainvoke(url="https://example.com", timeout=60)
    assert result == {"data": "Data from https://example.com with timeout 60"}
