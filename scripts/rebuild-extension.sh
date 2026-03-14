#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EXT_DIR="${ROOT_DIR}/extension"
ZIPPED="/tmp/chrome-agent-bridge.zip"
PROFILE_DIR="${HOME}/.config/google-chrome/"

echo "Packaging Chrome extension to ${ZIPPED}..."
cd "${EXT_DIR}"
zip -r "${ZIPPED}" * -q

if [[ -d "${PROFILE_DIR}" ]]; then
  echo "Installed packed extension: ${ZIPPED}"
  echo "Load it at: chrome://extensions -> load unpacked (for dev) or use your browser-side packaging pipeline."
else
  echo "Profile dir not found; only created package at ${ZIPPED}"
fi
