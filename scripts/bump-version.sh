#!/usr/bin/env bash
# bump-version.sh — Update version across all project manifests
#
# Usage:
#   ./scripts/bump-version.sh <version>
#
# Example:
#   ./scripts/bump-version.sh 1.2.3
#
# Updates version in:
#   - src/package.json          (JSON "version" field)
#   - src-tauri/Cargo.toml      (TOML version field)
#   - src-tauri/tauri.conf.json (JSON "version" field)

set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Usage: $0 <version>"
  echo "Example: $0 1.2.3"
  exit 1
fi

VERSION="$1"

# Validate semver-like format (X.Y.Z with optional pre-release/build)
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?(\+[a-zA-Z0-9.]+)?$ ]]; then
  echo "Error: '$VERSION' is not a valid semver version (expected X.Y.Z)"
  exit 1
fi

# Resolve project root (parent of scripts/)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

PACKAGE_JSON="$PROJECT_ROOT/src/package.json"
CARGO_TOML="$PROJECT_ROOT/src-tauri/Cargo.toml"
TAURI_CONF="$PROJECT_ROOT/src-tauri/tauri.conf.json"

# Check all files exist
for f in "$PACKAGE_JSON" "$CARGO_TOML" "$TAURI_CONF"; do
  if [[ ! -f "$f" ]]; then
    echo "Error: File not found: $f"
    exit 1
  fi
done

echo "Bumping version to $VERSION ..."

# --- src/package.json ---
# Use sed to replace the "version" line (handles whitespace)
sed -i 's/^\([[:space:]]*"version":\).*/\1 "'"$VERSION"'",/' "$PACKAGE_JSON"
echo "  ✓ $PACKAGE_JSON"

# --- src-tauri/Cargo.toml ---
# Replace version = "..." under [package]
sed -i 's/^\(version[[:space:]]*=\).*/\1 "'"$VERSION"'"/' "$CARGO_TOML"
echo "  ✓ $CARGO_TOML"

# --- src-tauri/tauri.conf.json ---
sed -i 's/^\([[:space:]]*"version":\).*/\1 "'"$VERSION"'",/' "$TAURI_CONF"
echo "  ✓ $TAURI_CONF"

echo ""
echo "Done! All files updated to version $VERSION"
echo "Run 'git diff' to review changes."
