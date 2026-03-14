#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "Preparing Chrome extension + metadata for Web Store submission"

echo "1) Packaging extension:"
cd "${ROOT_DIR}/extension"
zip -r /tmp/chrome-agent-bridge.zip * -q

echo "2) Generated: /tmp/chrome-agent-bridge.zip"

echo "3) Placeholder extension ID support: ensure extension ID is set when installing native host"

echo "4) If publishing, replace icons/* with store-compliant artwork (128+ recommended) and remove .git artifacts."
