#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MANIFEST_SRC="${ROOT_DIR}/native-host/manifest.json"

TARGET_DIRS=(
  "${HOME}/.config/google-chrome/NativeMessagingHosts"
  "${HOME}/.config/chromium/NativeMessagingHosts"
  "${HOME}/.config/chromium-dev/NativeMessagingHosts"
)

# Set CHROME_EXTENSION_ID to avoid placeholder in manifest (e.g., "abcdefghijklmnoabcdef" ).
EXTENSION_ID="${CHROME_EXTENSION_ID:-""}"

# Determine which host binary to use:
# 1. If running from dev, use target/release binary
# 2. If installed via cargo, use ~/.cargo/bin binary
# 3. Fall back to node.js host if Rust binary not found
if [[ -x "${ROOT_DIR}/rust-cli/target/release/chrome-agent-bridge-host" ]]; then
  HOST_PATH="${ROOT_DIR}/rust-cli/target/release/chrome-agent-bridge-host"
  echo "Using Rust host from: ${HOST_PATH}"
elif [[ -x "${HOME}/.cargo/bin/chrome-agent-bridge-host" ]]; then
  HOST_PATH="${HOME}/.cargo/bin/chrome-agent-bridge-host"
  echo "Using installed Rust host from: ${HOST_PATH}"
elif [[ -x "${ROOT_DIR}/native-host/host.js" ]]; then
  HOST_PATH="${ROOT_DIR}/native-host/host.js"
  echo "Warning: Falling back to Node.js host (deprecated)"
else
  echo "Error: No host binary found. Build with:"
  echo "  cd ${ROOT_DIR}/rust-cli && cargo build --release --bin chrome-agent-bridge-host"
  exit 1
fi

for TARGET_DIR in "${TARGET_DIRS[@]}"; do
  mkdir -p "${TARGET_DIR}"
  TARGET_PATH="${TARGET_DIR}/chrome_agent_bridge.json"

  # Read manifest template and substitute path
  MANIFEST_CONTENT=$(cat "${MANIFEST_SRC}")
  MANIFEST_CONTENT="${MANIFEST_CONTENT/__HOST_PATH__/${HOST_PATH}}"

  if [[ -n "${EXTENSION_ID}" ]]; then
    MANIFEST_CONTENT="${MANIFEST_CONTENT/__CHROME_EXTENSION_ID__/${EXTENSION_ID}}"
  else
    MANIFEST_CONTENT="${MANIFEST_CONTENT/__CHROME_EXTENSION_ID__/PUT_YOUR_EXTENSION_ID_HERE}"
  fi

  printf "%s\n" "${MANIFEST_CONTENT}" > "${TARGET_PATH}"
  echo "Installed native host manifest to ${TARGET_PATH}"
done

echo "Host path: ${HOST_PATH}"
if [[ -z "${EXTENSION_ID}" ]]; then
  echo "Set CHROME_EXTENSION_ID to your extension ID to avoid a placeholder in installed manifests."
fi
