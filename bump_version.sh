#!/bin/bash
# Usage: ./bump_version.sh 0.2.0
# Updates the version in both `release` and `Cargo.toml`.

set -e

if [ -z "$1" ]; then
    echo "Usage: $0 <new_version>"
    echo "Example: $0 0.2.0"
    exit 1
fi

NEW_VERSION="$1"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Update the release file
echo "$NEW_VERSION" > "$SCRIPT_DIR/release"

# Update Cargo.toml version field
sed -i "s/^version = \".*\"/version = \"$NEW_VERSION\"/" "$SCRIPT_DIR/Cargo.toml"

echo "Version updated to $NEW_VERSION in both release and Cargo.toml"
