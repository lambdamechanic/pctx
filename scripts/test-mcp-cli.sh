#!/bin/bash
set -e

# CLI Integration Test Script for pctx mcp start
# Tests the full CLI with both stdio and HTTP MCP servers

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Create temp directory for test files
TEST_DIR=$(mktemp -d)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Cleanup function
cleanup() {
    echo -e "${YELLOW}Cleaning up...${NC}"
    if [ -f "$TEST_DIR/pctx-test.pid" ]; then
        kill $(cat "$TEST_DIR/pctx-test.pid") 2>/dev/null || true
    fi
    lsof -ti:8080 | xargs kill -9 2>/dev/null || true
    rm -rf "$TEST_DIR"
}

trap cleanup EXIT

echo -e "${GREEN}Starting CLI Integration Tests${NC}"
echo "======================================"

# Test 1: Start server (tests HTTP endpoint with parallel MCP initialization)
echo -e "\n${YELLOW}Test 1: Starting pctx MCP server${NC}"

# Create test config with both stdio and HTTP MCP servers
# This tests parallel initialization with mixed transport types
cat > "$TEST_DIR/pctx-test.json" <<'EOF'
{
  "name": "pctx-test",
  "version": "0.1.0",
  "servers": [
    {
      "name": "memory",
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-memory"]
    },
    {
      "name": "time",
      "url": "https://mcp.run/time"
    }
  ]
}
EOF

cd "$PROJECT_ROOT"
cargo run --bin pctx -- mcp start \
    --config "$TEST_DIR/pctx-test.json" \
    --no-banner \
    > "$TEST_DIR/pctx-test.log" 2>&1 &
echo $! > "$TEST_DIR/pctx-test.pid"

# Wait for server to be ready (check logs for initialization message)
echo "Waiting for server to start..."
for i in {1..60}; do
    # Check if server has initialized by looking for log message
    if grep -q "PCTX listening at" "$TEST_DIR/pctx-test.log" 2>/dev/null; then
        echo -e "${GREEN}✓ Server started successfully in $i seconds${NC}"
        break
    fi
    if [ $i -eq 60 ]; then
        echo -e "${RED}✗ Server failed to start within 60 seconds${NC}"
        echo "Server logs:"
        cat "$TEST_DIR/pctx-test.log"
        exit 1
    fi
    sleep 1
done

# Give server a moment to be fully ready
sleep 1

# Test 2: MCP endpoint check
echo -e "\n${YELLOW}Test 2: MCP endpoint check${NC}"
mcp_response=$(curl -s -X POST http://localhost:8080/mcp \
    -H "Content-Type: application/json" \
    -H "Accept: application/json, text/event-stream" \
    -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0.0"}}}')

if echo "$mcp_response" | grep -q "result"; then
    echo -e "${GREEN}✓ MCP endpoint is responding${NC}"
else
    echo -e "${RED}✗ MCP endpoint check failed${NC}"
    echo "Response: $mcp_response"
    echo "Server logs:"
    tail -20 "$TEST_DIR/pctx-test.log"
    exit 1
fi

# Test 3: List tools via MCP protocol
echo -e "\n${YELLOW}Test 3: List tools from MCP server${NC}"
response=$(curl -s -X POST http://localhost:8080/mcp \
    -H "Content-Type: application/json" \
    -H "Accept: application/json, text/event-stream" \
    -d '{
      "jsonrpc": "2.0",
      "id": 1,
      "method": "tools/list"
    }')

echo "Response preview: ${response:0:200}..."

# Check that we got tools back
if echo "$response" | grep -q '"tools"'; then
    echo -e "${GREEN}✓ Successfully listed tools from MCP server${NC}"

    # Count how many tools we found
    tool_count=$(echo "$response" | grep -o '"name"' | wc -l | tr -d ' ')
    echo "  Found $tool_count tools"
else
    echo -e "${RED}✗ No tools found in response${NC}"
    echo "Full response: $response"
    exit 1
fi

# Test 4: Verify MCP server initialization in logs
echo -e "\n${YELLOW}Test 4: Verify MCP server initialization${NC}"
echo "Checking server logs..."

if grep -q "Creating code mode interface" "$TEST_DIR/pctx-test.log"; then
    echo -e "${GREEN}✓ MCP server initialization logged${NC}"

    # Check if any servers were initialized
    if grep -q "Successfully initialized MCP server\|PCTX listening" "$TEST_DIR/pctx-test.log"; then
        echo -e "${GREEN}✓ Server started successfully${NC}"
    fi
else
    echo -e "${YELLOW}⚠ MCP server initialization logs not found${NC}"
fi

# Show summary
echo -e "\n${GREEN}======================================"
echo "✓ All tests PASSED!"
echo -e "======================================${NC}"
echo ""

if [ "${SHOW_LOGS}" = "1" ]; then
    echo -e "${YELLOW}Server logs:${NC}"
    echo "---"
    tail -20 "$TEST_DIR/pctx-test.log"
    echo "---"
    echo ""
fi

exit 0
