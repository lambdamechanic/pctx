# from __future__ import annotations

from collections.abc import Awaitable, Callable
import inspect
import textwrap
from typing import Any, Annotated, get_type_hints


from pydantic import BaseModel, ConfigDict, SkipValidation, Field, create_model


class Tool(BaseModel):
    name: str
    """
    Unique name of tool that clearly specifies it's purpose
    """

    description: str = ""
    """
    Longer-form text which instructs the model how/why/when to use the tool.
    """

    input_schema: Annotated[BaseModel, SkipValidation] = Field(
        default=None, description="The tool schema."
    )

    output_schema: Annotated[BaseModel | None, SkipValidation] = Field(
        default=None, description="The return type schema."
    )

    func: Callable[..., str] | None
    """The function to run when the tool is called."""
    coroutine: Callable[..., Awaitable[str]] | None = None
    """The asynchronous version of the function."""

    @classmethod
    def from_func(
        cls,
        func: Callable | None = None,
        coroutine: Callable[..., Awaitable[Any]] | None = None,
        name: str | None = None,
        description: str | None = None,
    ) -> "Tool":
        """
        Creates a tool from a given function.
        """
        if func is not None:
            source_function = func
        elif coroutine is not None:
            source_function = coroutine
        else:
            msg = "Function and/or coroutine must be provided"
            raise ValueError(msg)

        if description is None:
            # use function doc string & remove indents
            _desc = textwrap.dedent(source_function.__doc__ or "")
        else:
            _desc = description

        name_ = name or source_function.__name__

        input_schema = create_input_schema(f"{name_}_Input", source_function)
        output_schema = create_output_schema(f"{name_}_Output", source_function)

        return cls(
            name=name_,
            description=_desc,
            func=func,
            coroutine=coroutine,
            input_schema=None if is_empty_schema(input_schema) else input_schema,
            output_schema=None if is_empty_schema(output_schema) else output_schema,
        )


_MODEL_CONFIG: ConfigDict = {"extra": "forbid", "arbitrary_types_allowed": True}


def create_input_schema(
    model_name: str,
    func: Callable,
) -> type[BaseModel]:
    """
    Creates pydantic model from function signature.

    Args:
        model_name: Name for the generated Pydantic model
        func: The function to extract parameters from

    Returns:
        A dynamically created Pydantic BaseModel class
    """
    sig = inspect.signature(func)

    # Build field definitions for create_model
    fields: dict[str, Any] = {}

    for param_name, param in sig.parameters.items():
        # Skip *args and **kwargs
        if param.kind in (
            inspect.Parameter.VAR_POSITIONAL,
            inspect.Parameter.VAR_KEYWORD,
        ):
            continue

        # Get type annotation (default to Any if not specified)
        annotation = (
            param.annotation if param.annotation != inspect.Parameter.empty else Any
        )

        # Determine if the parameter is required or has a default value
        if param.default == inspect.Parameter.empty:
            # Required field - use ... as the Pydantic sentinel for required
            fields[param_name] = (annotation, ...)
        else:
            # Optional field with default value
            fields[param_name] = (annotation, param.default)

    return create_model(model_name, __config__=_MODEL_CONFIG, **fields)


def create_output_schema(
    model_name: str,
    func: Callable,
) -> type[BaseModel]:
    """
    Creates pydantic model from function return type annotation.

    Args:
        model_name: Name for the generated Pydantic model
        func: The function to extract return type from

    Returns:
        A dynamically created Pydantic BaseModel class

    If the return type is already a BaseModel subclass, it's returned as-is.
    Otherwise, a wrapper model with a single 'data' field is created.
    """
    # Use get_type_hints to resolve string annotations to actual types
    # This handles cases where the calling code uses "from __future__ import annotations"
    try:
        type_hints = get_type_hints(func)
        return_annotation = type_hints.get("return", Any)
    except Exception:
        # Fallback to inspect if get_type_hints fails
        sig = inspect.signature(func)
        return_annotation = (
            sig.return_annotation if sig.return_annotation is not sig.empty else Any
        )

    # Check if return type is already a BaseModel subclass
    try:
        if isinstance(return_annotation, type) and issubclass(
            return_annotation, BaseModel
        ):
            return return_annotation
    except TypeError:
        # Not a class or can't check subclass
        pass

    # Wrap the return type in a model with a 'data' field
    fields: dict[str, Any] = {"data": (return_annotation, ...)}

    return create_model(model_name, __config__=_MODEL_CONFIG, **fields)


def is_empty_schema(schema: type[BaseModel]) -> bool:
    json_schema = schema.model_json_schema()

    return (
        json_schema.get("type") == "object"
        and len(json_schema.get("properties", {})) == 0
    )
