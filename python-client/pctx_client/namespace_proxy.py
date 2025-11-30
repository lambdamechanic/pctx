"""
Namespace proxy for clean Python API to registered tools.

Allows calling tools like: AsyncTools.triple(7)
Instead of: CALLABLE_TOOLS.execute('AsyncTools.triple', {value: 7})
"""

from typing import Any, Callable, Dict


class ToolProxy:
    """Proxy for a specific tool that can be called like a regular function."""

    def __init__(self, namespace: str, name: str, execute_fn: Callable):
        """
        Initialize tool proxy.

        Args:
            namespace: Tool namespace
            name: Tool name
            execute_fn: Function to execute the tool
        """
        self._namespace = namespace
        self._name = name
        self._execute_fn = execute_fn
        self._full_name = f"{namespace}.{name}"

    async def __call__(self, *args, **kwargs) -> Any:
        """
        Call the tool with Python arguments.

        Supports both positional and keyword arguments:
        - tool(arg1, arg2) -> {arg1, arg2}
        - tool(a=1, b=2) -> {a: 1, b: 2}
        - tool(value) -> {value}  # single positional becomes value param
        """
        # Convert Python args to tool parameters
        if args and kwargs:
            raise ValueError(f"Cannot mix positional and keyword arguments when calling {self._full_name}")

        if kwargs:
            params = kwargs
        elif len(args) == 1:
            # Single positional argument - use as 'value' parameter
            params = args[0] if isinstance(args[0], dict) else {"value": args[0]}
        elif len(args) > 1:
            # Multiple positional - use as array
            params = {"args": list(args)}
        else:
            params = {}

        return await self._execute_fn(self._full_name, params)


class NamespaceProxy:
    """Proxy for a namespace that provides attribute-based access to tools."""

    def __init__(self, namespace: str, execute_fn: Callable):
        """
        Initialize namespace proxy.

        Args:
            namespace: Namespace name
            execute_fn: Function to execute tools
        """
        self._namespace = namespace
        self._execute_fn = execute_fn
        self._tools: Dict[str, ToolProxy] = {}

    def _add_tool(self, name: str):
        """Add a tool to this namespace."""
        tool = ToolProxy(self._namespace, name, self._execute_fn)
        self._tools[name] = tool
        # Make it accessible as attribute
        setattr(self, name, tool)

    def __getattr__(self, name: str) -> ToolProxy:
        """Get a tool by attribute access."""
        if name.startswith('_'):
            raise AttributeError(f"'{type(self).__name__}' object has no attribute '{name}'")

        if name in self._tools:
            return self._tools[name]

        raise AttributeError(
            f"Tool '{self._namespace}.{name}' not found. "
            f"Available tools: {', '.join(self._tools.keys())}"
        )

    def __dir__(self):
        """List available tools for tab completion."""
        return list(self._tools.keys())
