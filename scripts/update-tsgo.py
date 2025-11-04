#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""
Update TypeScript-Go binaries for all platforms.

Usage:
    uv run scripts/update-tsgo.py 2025-11-04
"""

import sys
import os
import urllib.request
import tarfile
import zipfile
import shutil
from pathlib import Path


PLATFORMS = {
    "darwin-arm64": {"archive": "tsgo-darwin-arm64.tar.gz", "binary": "tsgo"},
    "darwin-x64": {"archive": "tsgo-darwin-amd64.tar.gz", "binary": "tsgo"},
    "linux-x64": {"archive": "tsgo-linux-amd64.tar.gz", "binary": "tsgo"},
    "win32-x64": {"archive": "tsgo-windows-amd64.zip", "binary": "tsgo.exe"},
}

BASE_URL = "https://github.com/sxzz/tsgo-releases/releases/download"


def download_file(url: str, dest: Path) -> None:
    """Download a file from a URL to a destination path."""
    print(f"Downloading {url}...")
    with urllib.request.urlopen(url) as response:
        with open(dest, "wb") as f:
            f.write(response.read())


def extract_tarball(archive_path: Path, dest_dir: Path) -> None:
    """Extract a tar.gz archive."""
    print(f"Extracting {archive_path}...")
    with tarfile.open(archive_path, "r:gz") as tar:
        tar.extractall(dest_dir)


def extract_zip(archive_path: Path, dest_dir: Path) -> None:
    """Extract a zip archive."""
    print(f"Extracting {archive_path}...")
    with zipfile.ZipFile(archive_path, "r") as zip_ref:
        zip_ref.extractall(dest_dir)


def main():
    if len(sys.argv) != 2:
        print("Usage: uv run scripts/update-tsgo.py <release-tag>")
        print("Example: uv run scripts/update-tsgo.py 2025-11-04")
        sys.exit(1)

    release_tag = sys.argv[1]

    # Determine paths
    script_dir = Path(__file__).parent
    repo_root = script_dir.parent
    bin_dir = repo_root / "crates" / "sdk_runner" / "bin"
    temp_dir = bin_dir / "temp"

    # Create temp directory
    temp_dir.mkdir(parents=True, exist_ok=True)

    try:
        # Download and extract all platforms
        for platform_name, info in PLATFORMS.items():
            archive_name = info["archive"]
            binary_name = info["binary"]
            url = f"{BASE_URL}/{release_tag}/{archive_name}"

            # Download archive
            archive_path = temp_dir / archive_name
            download_file(url, archive_path)

            # Extract archive
            if archive_name.endswith(".tar.gz"):
                extract_tarball(archive_path, temp_dir)
            elif archive_name.endswith(".zip"):
                extract_zip(archive_path, temp_dir)

            # Move binary to bin directory with platform-specific name
            extension = ".exe" if platform_name.startswith("win32") else ""
            final_binary_name = f"tsgo-{platform_name}{extension}"
            src_binary = temp_dir / binary_name
            dest_binary = bin_dir / final_binary_name

            if src_binary.exists():
                shutil.move(str(src_binary), str(dest_binary))
                print(f"Installed {final_binary_name}")

                # Set executable permissions on Unix platforms
                if not platform_name.startswith("win32"):
                    os.chmod(dest_binary, 0o755)
            else:
                print(f"Warning: Binary {binary_name} not found in {archive_name}")

        # Extract lib.d.ts files from one of the archives (they're the same across platforms)
        print("\nExtracting TypeScript lib files...")
        darwin_archive = temp_dir / PLATFORMS["darwin-arm64"]["archive"]

        if darwin_archive.exists():
            with tarfile.open(darwin_archive, "r:gz") as tar:
                # Extract only lib.*.d.ts files
                for member in tar.getmembers():
                    if member.name.startswith("lib.") and member.name.endswith(".d.ts"):
                        member.name = os.path.basename(member.name)
                        tar.extract(member, bin_dir)
                        print(f"  Extracted {member.name}")

        print("\n✓ Successfully updated all TypeScript binaries!")
        print(f"  Location: {bin_dir}")

    finally:
        # Clean up temp directory
        if temp_dir.exists():
            shutil.rmtree(temp_dir)
            print("\n✓ Cleaned up temporary files")


if __name__ == "__main__":
    main()
