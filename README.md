# Chrome Agent Bridge

Let AI coding agents control your real Chrome browser - with your existing logins, cookies, and extensions intact.

## How It Works

```
AI Agent  →  CLI (chrome-browser)  →  WebSocket  →  Native Host  →  Chrome Extension  →  Browser
```

The extension runs inside Chrome and executes commands (navigate, click, type, screenshot, etc.) sent by the CLI. Your agent calls the CLI, the CLI talks to the native messaging host via WebSocket, and the host relays commands to the extension.

## Install

### 1. Install the Chrome Extension

Install from the [Chrome Web Store](https://chrome.google.com/webstore) (search "Chrome Agent Bridge").

Or load unpacked for development:
1. Go to `chrome://extensions`
2. Enable "Developer mode"
3. Click "Load unpacked" and select the `extension/` directory

### 2. Build and Install the Rust CLI + Native Host

```bash
cd rust-cli
cargo build --release

# Install the CLI
cp target/release/browser ~/.local/bin/chrome-browser

# Install the native messaging host
cp target/release/chrome-agent-bridge-host ~/.local/bin/

# Register the native messaging host with Chrome
./scripts/install-native-host.sh
```

### 3. Verify

```bash
chrome-browser ping
# Should return: {"ok": true, "pong": true}
```

## CLI Commands

```bash
chrome-browser ping                          # Check connection
chrome-browser navigate '{"url": "..."}'     # Go to URL
chrome-browser getContent '{"format": "annotated"}'  # Read page with clickable elements
chrome-browser click '{"selector": "..."}'   # Click element
chrome-browser type '{"selector": "...", "text": "..."}'  # Type into input
chrome-browser fillForm '{"fields": [...]}'  # Fill form fields
chrome-browser screenshot '{}'               # Capture page
chrome-browser eval '{"code": "..."}'        # Run JavaScript
chrome-browser getUrl                        # Get current URL
chrome-browser getTabs                       # List open tabs
chrome-browser switchTab '{"tabId": 123}'    # Switch to tab
chrome-browser scroll '{"direction": "down"}'  # Scroll page
chrome-browser hover '{"selector": "..."}'   # Hover over element
chrome-browser select '{"selector": "...", "value": "..."}' # Select dropdown
chrome-browser waitForElement '{"selector": "..."}' # Wait for element
chrome-browser uploadFile '{"selector": "...", "path": "..."}' # Upload file
chrome-browser back                          # Go back
chrome-browser forward                       # Go forward
chrome-browser reload                        # Reload page
```

## For AI Agents

Install the skill file for your coding agent:

```bash
# Claude Code
mkdir -p ~/.claude/skills/chrome-agent-bridge
cp SKILL.md ~/.claude/skills/chrome-agent-bridge/SKILL.md

# Or use the setup command
chrome-browser setup claude
```

## Architecture

- **Chrome Extension** (Manifest V3) - Service worker + content scripts that execute browser commands
- **Native Messaging Host** (Rust) - Bridges between the extension and WebSocket clients
- **CLI** (Rust) - Sends commands via WebSocket, returns JSON results

## Privacy

This extension does not collect, transmit, or store any user data. All communication happens locally between the CLI, native host, and extension. See [PRIVACY.md](extension/PRIVACY.md).

## License

MIT
