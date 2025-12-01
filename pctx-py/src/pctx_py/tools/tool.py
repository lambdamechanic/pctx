from __future__ import annotations

from collections.abc import Awaitable, Callable
import textwrap
from typing import Any, Annotated


from pydantic import BaseModel, SkipValidation, Field

ArgsSchema = type[BaseModel] | dict[str, Any]


class Tool(BaseModel):
    name: str
    """
    Unique name of tool that clearly specifies it's purpose
    """

    description: str = ""
    """
    Longer-form text which instructs the model how/why/when to use the tool.
    """

    args_schema: Annotated[ArgsSchema | None, SkipValidation] = Field(
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

        return cls(
            name=name or source_function.__name__,
            description=_desc,
            func=func,
            coroutine=coroutine,
        )
