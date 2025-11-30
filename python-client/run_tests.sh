#!/bin/bash
#
# Run PCTX Python client tests
#
# This script starts a PCTX server if needed and runs the test suite.
#

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
PCTX_PORT="${PCTX_PORT:-8080}"
PCTX_HOST="${PCTX_HOST:-127.0.0.1}"
MCP_URL="http://${PCTX_HOST}:${PCTX_PORT}/mcp"
WS_URL="ws://${PCTX_HOST}:${PCTX_PORT}/local-tools"

echo -e "${GREEN}PCTX Python Client Test Runner${NC}"
echo "=============================="
echo ""

# Check if PCTX server is already running
if curl -s "${MCP_URL}" > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} PCTX server is already running at ${MCP_URL}"
    SERVER_RUNNING=true
else
    echo -e "${YELLOW}⚠${NC} PCTX server not detected at ${MCP_URL}"
    SERVER_RUNNING=false
fi

# Function to start PCTX server
start_server() {
    echo -e "${YELLOW}Starting PCTX server...${NC}"

    # Check if cargo is available
    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}✗${NC} cargo not found. Please install Rust."
        exit 1
    fi

    # Start server in background
    cd ../.. # Go to pctx root
    cargo run --bin pctx -- start --port ${PCTX_PORT} --no-banner > /tmp/pctx-test-server.log 2>&1 &
    SERVER_PID=$!
    cd python-client

    echo "Server PID: $SERVER_PID"
    echo "Waiting for server to start..."

    # Wait for server to be ready (max 30 seconds)
    for i in {1..30}; do
        if curl -s "${MCP_URL}" > /dev/null 2>&1; then
            echo -e "${GREEN}✓${NC} Server started successfully"
            return 0
        fi
        sleep 1
    done

    echo -e "${RED}✗${NC} Server failed to start within 30 seconds"
    echo "Server logs:"
    cat /tmp/pctx-test-server.log
    kill $SERVER_PID 2>/dev/null || true
    exit 1
}

# Function to stop server
stop_server() {
    if [ ! -z "$SERVER_PID" ]; then
        echo -e "${YELLOW}Stopping PCTX server (PID: $SERVER_PID)...${NC}"
        kill $SERVER_PID 2>/dev/null || true
        wait $SERVER_PID 2>/dev/null || true
        echo -e "${GREEN}✓${NC} Server stopped"
    fi
}

# Trap to ensure server is stopped on exit
trap stop_server EXIT

# Start server if not running
if [ "$SERVER_RUNNING" = false ]; then
    if [ "$PCTX_SKIP_SERVER_START" = "1" ]; then
        echo -e "${YELLOW}⚠${NC} Server not running and PCTX_SKIP_SERVER_START=1, tests may fail"
    else
        start_server
    fi
fi

# Install dependencies
echo ""
echo "Installing dependencies..."
pip install -e ".[dev]" -q

# Export URLs for tests
export PCTX_MCP_URL="$MCP_URL"
export PCTX_WS_URL="$WS_URL"

# Run tests
echo ""
echo -e "${GREEN}Running tests...${NC}"
echo ""

if [ "$1" = "--verbose" ] || [ "$1" = "-v" ]; then
    pytest tests/ -v
else
    pytest tests/
fi

TEST_RESULT=$?

echo ""
if [ $TEST_RESULT -eq 0 ]; then
    echo -e "${GREEN}✓ All tests passed!${NC}"
else
    echo -e "${RED}✗ Some tests failed${NC}"
fi

exit $TEST_RESULT
