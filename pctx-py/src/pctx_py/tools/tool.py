from __future__ import annotations

from collections.abc import Awaitable, Callable
import inspect
import textwrap
from typing import Any, Annotated


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

    args_schema: Annotated[BaseModel, SkipValidation] = Field(
        default=None, description="The tool schema."
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
    ) -> Tool:
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
        args_schema = create_pydantic_model_from_func(name_, source_function)

        return cls(
            name=name_,
            description=_desc,
            func=func,
            coroutine=coroutine,
            args_schema=args_schema,
        )


_MODEL_CONFIG: ConfigDict = {"extra": "forbid", "arbitrary_types_allowed": True}


def create_pydantic_model_from_func(
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
