#!/bin/bash
set -e

echo "Testing Python bindings for pctx-code-mode..."

cd crates/code_mode_py_bindings

# Build the Python package with maturin
echo "Building Python package with maturin..."
uv run maturin develop

# Run pytest with uv
echo "Running Python tests..."
uv run pytest tests/ -v

echo "Python tests completed successfully!"
