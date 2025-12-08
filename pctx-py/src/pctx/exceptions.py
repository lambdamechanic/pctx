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
