# TypeScript-Go Binaries

This directory contains platform-specific TypeScript-Go binaries and TypeScript library definition files.

## Contents

- `tsgo-darwin-arm64` - macOS ARM64 binary
- `tsgo-darwin-x64` - macOS x64 binary
- `tsgo-linux-x64` - Linux x64 binary
- `tsgo-win32-x64.exe` - Windows x64 binary
- `lib.*.d.ts` - TypeScript library definition files (platform-independent)

## Updating Binaries

To update to a new version of TypeScript-Go:

```bash
uv run scripts/update-tsgo.py <release-tag>
```

For example:
```bash
uv run scripts/update-tsgo.py 2025-11-04
```

The script will:
1. Download binaries for all supported platforms
2. Extract and rename them appropriately
3. Extract TypeScript lib files
4. Clean up temporary files

## Source

Binaries are from: https://github.com/sxzz/tsgo-releases
