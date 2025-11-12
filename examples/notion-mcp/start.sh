#!/bin/bash
set -e

# Create a temporary file to capture the auth token
NOTION_LOG=$(mktemp)

echo "Starting Notion MCP server on port ${NOTION_MCP_PORT}..."
npx -y @notionhq/notion-mcp-server --transport http --port ${NOTION_MCP_PORT} > "$NOTION_LOG" 2>&1 &
NOTION_PID=$!

echo "Waiting for Notion MCP server to start and capturing auth token..."
NOTION_AUTH_TOKEN=""
for i in {1..30}; do
  # Check if the auth token has been generated
  if grep -q "Generated auth token:" "$NOTION_LOG"; then
    NOTION_AUTH_TOKEN=$(grep "Generated auth token:" "$NOTION_LOG" | sed 's/.*Generated auth token: //')
    echo "Captured auth token from Notion MCP server"
    break
  fi

  if [ $i -eq 30 ]; then
    echo "Failed to capture auth token from Notion MCP server"
    cat "$NOTION_LOG"
    rm "$NOTION_LOG"
    exit 1
  fi
  sleep 1
done

# Wait a bit more for the server to fully start
sleep 2

# Verify server is responding
if ! curl -s -f -H "Authorization: Bearer ${NOTION_AUTH_TOKEN}" http://127.0.0.1:${NOTION_MCP_PORT}/mcp > /dev/null 2>&1; then
  echo "Notion MCP server is not responding"
  cat "$NOTION_LOG"
  rm "$NOTION_LOG"
  exit 1
fi

echo "Notion MCP server is ready!"

# Export the auth token for pctx to use
export NOTION_MCP_AUTH_TOKEN="${NOTION_AUTH_TOKEN}"

echo "Starting pctx on port ${PCTX_PORT}..."
pctx --config app/pctx.json start --port ${PCTX_PORT} --host 0.0.0.0 > >(tee -a "$NOTION_LOG") 2>&1 &
PCTX_PID=$!

# Keep tailing the log
tail -f "$NOTION_LOG" &
TAIL_PID=$!

# Function to handle shutdown
shutdown() {
  echo "Shutting down services..."
  kill $TAIL_PID $NOTION_PID $PCTX_PID 2>/dev/null || true
  wait $NOTION_PID $PCTX_PID 2>/dev/null || true
  rm -f "$NOTION_LOG"
  exit 0
}

trap shutdown SIGTERM SIGINT

# Wait for either process to exit
wait -n $NOTION_PID $PCTX_PID
EXIT_CODE=$?

# If either process exits, shut down the other
shutdown
