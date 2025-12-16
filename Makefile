.PHONY: help release docs test-python-client

# Default target - show help when running just 'make'
.DEFAULT_GOAL := help

help:
	@echo "pctx dev scripts"
	@echo ""
	@echo "Available targets:"
	@echo "  make docs                  - Generate CLI and Python documentation"
	@echo "  make test-python-client    - Run Python client tests"
	@echo "  make release               - Interactive release script (bump version, update changelog)"
	@echo ""

# Generate CLI and Python documentation
docs:
	@./scripts/generate-cli-docs.sh
	@echo ""
	@echo "Building Python Sphinx documentation..."
	@cd pctx-py && uv run sphinx-build -b html docs docs/_build/html
	@echo ""
	@echo "✓ Documentation built successfully!"

# Run Python client tests
test-python-client:
	@cd pctx-py && uv run pytest tests/ -v

# Interactive release workflow
release:
	@./release.sh


