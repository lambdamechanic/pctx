.PHONY: help release docs update-mcp-client

# Default target - show help when running just 'make'
.DEFAULT_GOAL := help

help:
	@echo "pctx dev scripts"
	@echo ""
	@echo "Available targets:"
	@echo "  make docs              - Generate CLI documentation"
	@echo "  make release           - Interactive release script (bump version, update changelog)"
	@echo ""

# Generate CLI documentation
docs:
	@./scripts/generate-cli-docs.sh

# Interactive release workflow
release:
	@./release.sh

# Update MCP client
update-mcp-client:
	@./scripts/update-mcp-client.py

