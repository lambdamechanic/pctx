"""Tests for create_output_schema"""

from pydantic import BaseModel
from pctx._tool import create_output_schema


def test_output_schema_simple_type():
    """Test creating output schema from simple return type"""

    def returns_int() -> int:
        return 42

    Model, wrapped = create_output_schema("IntOutput", returns_int)

    # Should wrap in data field
    assert wrapped
    instance = Model(data=42)
    assert getattr(instance, "data") == 42

    # Check schema
    schema = Model.model_json_schema()
    assert "data" in schema["properties"]
    assert schema["properties"]["data"]["type"] == "integer"
    assert schema["required"] == ["data"]


def test_output_schema_string_type():
    """Test creating output schema from string return type"""

    def returns_str() -> str:
        return "hello"

    Model, wrapped = create_output_schema("StrOutput", returns_str)

    assert wrapped
    instance = Model(data="hello world")
    assert getattr(instance, "data") == "hello world"

    schema = Model.model_json_schema()
    assert schema["properties"]["data"]["type"] == "string"


def test_output_schema_complex_type():
    """Test creating output schema from complex return type"""

    def returns_list() -> list[str]:
        return ["a", "b", "c"]

    Model, wrapped = create_output_schema("ListOutput", returns_list)

    assert wrapped
    instance = Model(data=["x", "y", "z"])
    assert getattr(instance, "data") == ["x", "y", "z"]

    schema = Model.model_json_schema()
    assert "data" in schema["properties"]
    assert schema["properties"]["data"]["type"] == "array"


def test_output_schema_dict_type():
    """Test creating output schema from dict return type"""

    def returns_dict() -> dict[str, int]:
        return {"a": 1, "b": 2}

    Model, wrapped = create_output_schema("DictOutput", returns_dict)

    assert wrapped
    instance = Model(data={"x": 10, "y": 20})
    assert getattr(instance, "data") == {"x": 10, "y": 20}


def test_output_schema_no_annotation():
    """Test creating output schema when no return annotation exists"""

    def no_return_type():
        return "something"

    Model, _ = create_output_schema("NoTypeOutput", no_return_type)

    # Should use Any type
    instance = Model(data="can be anything")
    assert getattr(instance, "data") == "can be anything"

    instance2 = Model(data=42)
    assert getattr(instance2, "data") == 42


def test_output_schema_with_pydantic_model():
    """Test that Pydantic model return types are used as-is"""

    class UserOutput(BaseModel):
        name: str
        age: int

    def returns_model() -> UserOutput:
        return UserOutput(name="Alice", age=30)

    Model, wrapped = create_output_schema("UserModelOutput", returns_model)

    # Should be the same class (not wrapped)
    assert not wrapped
    assert Model is UserOutput

    # Can instantiate directly
    instance = Model(name="Bob", age=25)
    assert instance.name == "Bob"
    assert instance.age == 25


def test_output_schema_with_nested_pydantic_model():
    """Test output schema when function returns nested Pydantic model"""

    class Address(BaseModel):
        street: str
        city: str

    class Person(BaseModel):
        name: str
        address: Address

    def returns_person() -> Person:
        return Person(name="Alice", address=Address(street="Main St", city="NYC"))

    Model, wrapped = create_output_schema("PersonOutput", returns_person)

    # Should return Person as-is
    assert not wrapped
    assert Model is Person


def test_output_schema_optional_type():
    """Test creating output schema from optional return type"""

    def returns_optional() -> str | None:
        return None

    Model, wrapped = create_output_schema("OptionalOutput", returns_optional)

    # Should wrap in data field
    assert wrapped
    instance1 = Model(data="value")
    assert getattr(instance1, "data") == "value"

    instance2 = Model(data=None)
    assert getattr(instance2, "data") is None


def test_output_schema_union_type():
    """Test creating output schema from union return type"""

    def returns_union() -> int | str:
        return 42

    Model, wrapped = create_output_schema("UnionOutput", returns_union)

    assert wrapped
    instance1 = Model(data=42)
    assert getattr(instance1, "data") == 42

    instance2 = Model(data="hello")
    assert getattr(instance2, "data") == "hello"


def test_output_schema_bool_type():
    """Test creating output schema from bool return type"""

    def returns_bool() -> bool:
        return True

    Model, wrapped = create_output_schema("BoolOutput", returns_bool)

    assert wrapped
    instance = Model(data=False)
    assert getattr(instance, "data") is False

    schema = Model.model_json_schema()
    assert schema["properties"]["data"]["type"] == "boolean"


def test_output_schema_async_function():
    """Test creating output schema from async function"""

    async def async_returns_str() -> str:
        return "async result"

    Model, wrapped = create_output_schema("AsyncOutput", async_returns_str)

    assert wrapped
    instance = Model(data="test")
    assert getattr(instance, "data") == "test"
