# Import the Rust extension module
from . import pctx_code_mode as _pctx_code_mode

# Re-export the CodeMode class
CodeMode = _pctx_code_mode.CodeMode

__all__ = ["CodeMode"]
