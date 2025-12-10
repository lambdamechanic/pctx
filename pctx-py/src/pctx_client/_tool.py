from abc import ABC, abstractmethod
import inspect
import asyncio
import textwrap
from collections.abc import Awaitable, Callable
from typing import Annotated, Any, get_type_hints

from pydantic import BaseModel, ConfigDict, Field, SkipValidation, create_model


class BaseTool(BaseModel):
    name: str
    """
    Unique name of tool that clearly specifies it's purpose
    """

    namespace: str
    """
    Namespace the tool belongs in
    """

    description: str = ""
    """
    Longer-form text which instructs the model how/why/when to use the tool.
    """

    input_schema: Annotated[type[BaseModel] | None, SkipValidation] = Field(
        default=None, description="The tool schema."
    )

    output_schema: Annotated[type[BaseModel] | None, SkipValidation] = Field(
        default=None, description="The return type schema."
    )
    output_data_wrapped: bool = False

    def validate_input(self, obj: Any):
        if self.input_schema is not None:
            self.input_schema.model_validate(obj)

    def validate_output(self, obj: Any):
        if self.output_schema is not None:
            self.output_schema.model_validate(obj)

    @classmethod
    def from_func(
        cls,
        func: Callable | Callable[..., Awaitable[Any]],
        name: str | None = None,
        namespace: str = "tools",
        description: str | None = None,
    ) -> "Tool | AsyncTool":
        """
        Creates a tool from a given function.
        """

        if description is None:
            # use function doc string & remove indents
            _desc = textwrap.dedent(func.__doc__ or "")
        else:
            _desc = description

        name_ = name or func.__name__

        in_schema = create_input_schema(f"{name_}_Input", func)
        out_schema, output_wrapped = create_output_schema(f"{name_}_Output", func)

        input_schema = None if is_empty_schema(in_schema) else in_schema
        output_schema = None if is_empty_schema(out_schema) else out_schema

        # Create concrete tool classes based on sync vs async
        if asyncio.iscoroutinefunction(func):
            # Asynchronous tool
            class _CoroutineTool(AsyncTool):
                """Concrete asynchronous tool wrapping a coroutine"""

                _coroutine: Callable[..., Awaitable[Any]] = staticmethod(func)

                async def _ainvoke(self, **kwargs: Any) -> Any:
                    return await self._coroutine(**kwargs)

            return _CoroutineTool(
                name=name_,
                namespace=namespace,
                description=_desc,
                input_schema=input_schema,
                output_schema=output_schema,
                output_data_wrapped=output_wrapped,
            )
        else:
            # Synchronous tool
            class _FunctionTool(Tool):
                """Synchronous tool wrapping decorated function"""

                _func: Callable = staticmethod(func)

                def _invoke(self, **kwargs: Any) -> Any:
                    return self._func(**kwargs)

            return _FunctionTool(
                name=name_,
                namespace=namespace,
                description=_desc,
                input_schema=input_schema,
                output_schema=output_schema,
                output_data_wrapped=output_wrapped,
            )


class Tool(BaseTool, ABC):
    """
    Synchronous tool base class
    """

    @abstractmethod
    def _invoke(self, **kwargs) -> Any:
        """
        Sync implementation of the tool.

        Subclasses must implement this method for synchronous execution.

        Args:
            *args: Positional arguments for the tool.
            **kwargs: Keyword arguments for the tool.

        Returns:
            The result of the tool execution.
        """

    def invoke(self, **kwargs: Any) -> Any:
        """
        Calls the synchronous function with the provided arguments.

        Args:
            **kwargs: Arguments to pass to the function

        Returns:
            The result of the function call

        Raises:
            ValueError: If no synchronous function is available
        """

        self.validate_input(kwargs)

        output = self._invoke(**kwargs)
        if self.output_data_wrapped:
            output = {"data": output}

        self.validate_output(output)

        return output


class AsyncTool(BaseTool, ABC):
    """
    Asynchronous tool base class
    """

    @abstractmethod
    async def _ainvoke(self, **kwargs) -> Any:
        """
        Async implementation of the tool.

        Subclasses must implement this method for asynchronous execution.

        Args:
            *args: Positional arguments for the tool.
            **kwargs: Keyword arguments for the tool.

        Returns:
            The result of the tool execution.
        """

    async def ainvoke(self, **kwargs: Any) -> Any:
        """
        Calls the asynchronous function with the provided arguments.

        Args:
            **kwargs: Arguments to pass to the function

        Returns:
            The result of the function call

        Raises:
            ValueError: If no synchronous function is available
        """

        self.validate_input(kwargs)

        output = await self._ainvoke(**kwargs)
        if self.output_data_wrapped:
            output = {"data": output}

        self.validate_output(output)

        return output


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
) -> tuple[type[BaseModel], bool]:
    """
    Creates pydantic model from function return type annotation.

    Args:
        model_name: Name for the generated Pydantic model
        func: The function to extract return type from

    Returns:
        A tuple of (Pydantic BaseModel class, bool indicating if output was wrapped)

    If the return type is already a BaseModel subclass, it's returned as-is with False.
    Otherwise, a wrapper model with a single 'data' field is created and True is returned.
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
            return_annotation,  # type: ignore
            BaseModel,
        ):
            return return_annotation, False
    except TypeError:
        # Not a class or can't check subclass
        pass

    # Wrap the return type in a model with a 'data' field
    fields: dict[str, Any] = {"data": (return_annotation, ...)}

    return create_model(model_name, __config__=_MODEL_CONFIG, **fields), True


def is_empty_schema(schema: type[BaseModel]) -> bool:
    json_schema = schema.model_json_schema()

    return (
        json_schema.get("type") == "object"
        and len(json_schema.get("properties", {})) == 0
    )
