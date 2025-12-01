"""Tests for create_pydantic_model_from_func"""

from __future__ import annotations

from pydantic import BaseModel, ValidationError
import pytest
from pctx_py.tools.tool import create_pydantic_model_from_func


def test_simple_function_signature():
    """Test creating model from simple function signature"""

    def add(a: int, b: int) -> int:
        return a + b

    Model = create_pydantic_model_from_func("AddModel", add)

    # Test valid input
    instance = Model(a=5, b=10)
    assert getattr(instance, "a") == 5
    assert getattr(instance, "b") == 10

    # Test missing required field
    with pytest.raises(ValidationError):
        Model(a=5)

    # Test invalid types
    with pytest.raises(ValidationError):
        Model(a="foo", b="bar")


def test_function_with_defaults():
    """Test creating model from function with default values"""

    def greet(name: str, greeting: str = "Hello") -> str:
        return f"{greeting}, {name}!"

    Model = create_pydantic_model_from_func("GreetModel", greet)

    # Test with all parameters
    instance1 = Model(name="Alice", greeting="Hi")
    assert getattr(instance1, "name") == "Alice"
    assert getattr(instance1, "greeting") == "Hi"

    # Test with only required parameter
    instance2 = Model(name="Bob")
    assert getattr(instance2, "name") == "Bob"
    assert getattr(instance2, "greeting") == "Hello"


def test_function_without_type_hints():
    """Test creating model from function without type hints"""

    def no_types(x, y=5):
        return x + y

    Model = create_pydantic_model_from_func("NoTypesModel", no_types)

    # Should work with Any type
    instance = Model(x="test", y=10)
    assert getattr(instance, "x") == "test"
    assert getattr(instance, "y") == 10


def test_function_with_args_kwargs():
    """Test that *args and **kwargs are skipped"""

    def flexible(a: int, *args, b: str = "default", **kwargs) -> None:
        pass

    Model = create_pydantic_model_from_func("FlexibleModel", flexible)

    # Only 'a' and 'b' should be in the model
    instance = Model(a=42, b="test")
    assert getattr(instance, "a") == 42
    assert getattr(instance, "b") == "test"

    # Should not accept arbitrary fields (extra='forbid')
    with pytest.raises(ValidationError):
        Model(a=42, b="test", extra_field="should fail")


def test_complex_types():
    """Test with complex type annotations"""

    def process(items: list[str], count: int = 0) -> None:
        pass

    Model = create_pydantic_model_from_func("ProcessModel", process)

    instance = Model(items=["a", "b", "c"], count=3)
    assert getattr(instance, "items") == ["a", "b", "c"]
    assert getattr(instance, "count") == 3

    # Test validation
    with pytest.raises(ValidationError):
        Model(items="not a list")


def test_model_is_basemodel():
    """Test that created model is a proper BaseModel"""

    def dummy(x: int) -> None:
        pass

    Model = create_pydantic_model_from_func("DummyModel", dummy)

    assert issubclass(Model, BaseModel)
    assert getattr(Model, "__name__") == "DummyModel"


def test_model_json_schema():
    """Test that the model can generate JSON schema"""

    def example(name: str, age: int, active: bool = True) -> None:
        pass

    Model = create_pydantic_model_from_func("ExampleModel", example)

    schema = Model.model_json_schema()
    assert "properties" in schema
    assert "name" in schema["properties"]
    assert "age" in schema["properties"]
    assert "active" in schema["properties"]
    assert schema["required"] == ["name", "age"]


def test_async_function_signature():
    """Test creating model from async function signature"""

    async def fetch_data(url: str, timeout: int = 30) -> str:
        """Fetches data from a URL"""
        return f"Data from {url}"

    Model = create_pydantic_model_from_func("FetchModel", fetch_data)

    # Test with all parameters
    instance1 = Model(url="https://example.com", timeout=60)
    assert getattr(instance1, "url") == "https://example.com"
    assert getattr(instance1, "timeout") == 60

    # Test with only required parameter
    instance2 = Model(url="https://test.com")
    assert getattr(instance2, "url") == "https://test.com"
    assert getattr(instance2, "timeout") == 30

    # Test validation
    with pytest.raises(ValidationError):
        Model(timeout=30)  # Missing required 'url'


def test_nested_pydantic_model():
    """Test creating model with nested Pydantic model as input type"""

    class Address(BaseModel):
        street: str
        city: str
        zipcode: str

    class ContactInfo(BaseModel):
        email: str
        phone: str | None = None

    def create_user(
        name: str, address: Address, contact: ContactInfo, age: int = 18
    ) -> None:
        pass

    Model = create_pydantic_model_from_func("UserModel", create_user)

    # Test with valid nested models
    address_data = Address(street="123 Main St", city="Boston", zipcode="02101")
    contact_data = ContactInfo(email="test@example.com", phone="555-1234")

    instance = Model(name="Alice", address=address_data, contact=contact_data, age=25)

    assert getattr(instance, "name") == "Alice"
    assert getattr(instance, "address") == address_data
    assert getattr(instance, "contact") == contact_data
    assert getattr(instance, "age") == 25

    # Test with dict input (Pydantic auto-converts)
    instance2 = Model(
        name="Bob",
        address={"street": "456 Oak Ave", "city": "NYC", "zipcode": "10001"},
        contact={"email": "bob@test.com"},
    )

    assert getattr(instance2, "name") == "Bob"
    assert isinstance(getattr(instance2, "address"), Address)
    assert getattr(instance2, "address").city == "NYC"
    assert isinstance(getattr(instance2, "contact"), ContactInfo)
    assert getattr(instance2, "contact").phone is None

    # Test validation of nested model
    with pytest.raises(ValidationError):
        Model(
            name="Charlie",
            address={"street": "789 Pine Rd"},  # Missing required fields
            contact={"email": "charlie@test.com"},
        )
