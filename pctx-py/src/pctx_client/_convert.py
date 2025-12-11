from collections.abc import Callable
from typing import Any, overload

from pctx_client._tool import AsyncTool, Tool


@overload
def tool(
    name_or_callable: str,
    *args: Any,
    namespace: str = "tools",
    description: str | None = None,
) -> Callable[[Callable], Tool | AsyncTool]: ...
@overload
def tool(
    name_or_callable: Callable,
    *args: Any,
    namespace: str = "tools",
    description: str | None = None,
) -> Tool | AsyncTool: ...


def tool(
    name_or_callable: str | Callable,
    *args: Any,
    namespace: str = "tools",
    description: str | None = None,
) -> Tool | AsyncTool | Callable[[Callable], Tool | AsyncTool]:
    """
    Decorator that prints the name of the function it wraps when called.
    """

    def _crate_tool_factory(tool_name: str) -> Callable[[Callable], Tool | AsyncTool]:
        """
        Creates a decorator which takes the callable & returns the tool

        Args:
            tool_name: the unique name of the tool

        Returns:
            A function that takes a callable & returns a base tool
        """

        def _tool_factory(fn: Callable) -> Tool | AsyncTool:
            tool_desc = description

            return Tool.from_func(
                func=fn,
                name=tool_name,
                namespace=namespace,
                description=tool_desc,
            )

        return _tool_factory

    if len(args) != 0:
        raise ValueError("Too many arguments for @tool decorator")

    if isinstance(name_or_callable, str):
        # decorator used with params
        # @tool("other_tool")
        # def some_tool():
        #     pass
        return _crate_tool_factory(name_or_callable)
    elif callable(name_or_callable) and hasattr(name_or_callable, "__name__"):
        # decorator used without params
        # @tool
        # def some_tool():
        #     pass
        return _crate_tool_factory(name_or_callable.__name__)(name_or_callable)
    else:
        raise ValueError(
            f"The first arg of the tool decorator must be a string or a callable with a __name__ attribute. Got {type(name_or_callable)}"
        )
