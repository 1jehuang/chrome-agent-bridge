#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MANIFEST_SRC="${ROOT_DIR}/native-host/manifest.json"

for target in ${HOME}/.config/google-chrome/NativeMessagingHosts ${HOME}/.config/chromium/NativeMessagingHosts ${HOME}/.config/chromium-dev/NativeMessagingHosts; do
  mkdir -p "${target}"
  cp "${MANIFEST_SRC}" "${target}/chrome_agent_bridge.json"
  echo "Copied manifest template to ${target}/chrome_agent_bridge.json"
done
  echo "Set allowed_origins manually to: chrome-extension://<extension_id>/"
echo "Done."
