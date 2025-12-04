"""Exceptions for PCTX Python client."""


class PctxError(Exception):
    """Base exception for PCTX client errors."""

    pass


class SessionError(PctxError):
    """Raised when WebSocket connection fails."""

    pass


class ConnectionError(PctxError):
    """Raised when WebSocket connection fails."""

    pass


class ExecutionError(PctxError):
    """Raised when code execution fails."""

    pass


class ToolError(PctxError):
    """Raised when tool registration or execution fails."""

    pass
