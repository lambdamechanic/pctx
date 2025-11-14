FROM node:22-slim

# Build argument to bust cache when needed
ARG CACHEBUST=1

WORKDIR /app

# Install curl for health checks
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*

# Install pctx globally (using --no-audit --no-fund to speed up)
RUN npm install -g --no-audit --no-fund @portofcontext/pctx

# Copy all files at once to avoid cache issues
COPY . /app/
RUN npm ci --only=production --no-audit --no-fund && chmod +x /app/start.sh

# Create pctx.json if it doesn't exist (in case it was gitignored)
RUN if [ ! -f /app/pctx.json ]; then \
    echo '{"name":"nasa-example","description":"Example configuration for NASA MCP Server with pctx","servers":[{"name":"nasa","url":"http://127.0.0.1:3000/mcp"}]}' > /app/pctx.json; \
    fi

# Set environment variables
ENV NODE_ENV=production
ENV NASA_MCP_PORT=3000
ENV PCTX_PORT=8080
ENV HOST=0.0.0.0

# Expose only pctx port
EXPOSE 8080

# Run the startup script
CMD ["/app/start.sh"]
