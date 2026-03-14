use crate::config::VERSION;

/// Print short help message
pub fn print_help() {
    let help = r##"
chrome-agent-bridge v__VERSION__
Control Chrome browser from LLM agents

USAGE:
  chrome-browser <action> [params_json]
  chrome-browser <command>

COMMANDS:
  help, --help      Show this help
  docs              Show full documentation
  setup claude      Install Claude Code skill files
  setup generic     Print docs to stdout
  --version         Show version

ACTIONS:
  Session:     listTabs, newSession, setActiveTab, getActiveTab
  Navigation:  navigate, getContent, getInteractables, screenshot, evaluate, scroll
  Interaction: click, type, fillForm, waitFor
  Control:     fork, killFork, listForks, tryUntil, parallel
  Auth:        getAuthContext, requestAuth, listFrames
  Utility:     ping

EXAMPLES:
  chrome-browser ping
  chrome-browser newSession '{"url": "https://example.com"}'
  chrome-browser click '{"selector": "#btn"}'
  chrome-browser getContent '{"format": "annotated"}'

QUICK START:
  1. Install Chrome extension from extension/ folder
  2. chrome-browser ping                    # verify connection
  3. chrome-browser newSession '{"url": "https://google.com"}'
  4. chrome-browser click '{"text": "Sign in"}'
"##.replace("__VERSION__", VERSION);
    println!("{}", help);
}

/// Print full documentation
pub fn print_docs() {
    let docs = r##"# Chrome Agent Bridge

Control Chrome browser via WebSocket. Uses real browser with existing logins.

## Actions

### Session & Tab Management
- listTabs          List all open tabs
- newSession        Create new tab (returns content by default)
  - url             URL to navigate to
  - sandbox         true for private window (no cookies/cache)
- setActiveTab      Switch active tab
- getActiveTab      Get current tab info

### Navigation & Content
- navigate          Go to URL (returns content by default)
- getContent        Get page content (format: annotated, text, html)
- getInteractables  List clickable elements
- screenshot        Capture visible area
- evaluate          Execute JavaScript in the page main world
- scroll            Scroll page by x/y delta

### Interaction
- click             Click element (selector, text, or x/y)
- type              Type into input (selector, text, submit)
- fillForm          Fill form fields (inputs, textareas, selects)
- waitFor           Wait for element/text

## fillForm Usage

Fill form fields including text inputs, textareas, and <select> dropdowns:

```
chrome-browser fillForm '{"fields": [
  {"selector": "#name", "value": "John Doe"},
  {"selector": "#email", "value": "john@example.com"},
  {"selector": "#subject", "value": "support"},
  {"selector": "#message", "value": "Hello"}
]}'
```

NOTE: There is no "fill" command. Use fillForm with a fields array.

### Control Flow
- fork              Duplicate tab into multiple paths
- killFork          Close a fork
- listForks         List active forks
- tryUntil          Try alternatives until success
- parallel          Run commands on multiple URLs

### Auth
- getAuthContext    Detect login pages
- requestAuth       Request user auth approval

## Content Formats

getContent supports:
- annotated (default): Text with clickable elements marked
- text: Plain text
- html: Full HTML

## Fork Example

```
chrome-browser fork '{"paths": [
  {"name": "path-a", "commands": [{"action": "click", "params": {"text": "Option A"}}]},
  {"name": "path-b", "commands": [{"action": "click", "params": {"text": "Option B"}}]}
]}'

chrome-browser click '{"text": "Continue", "fork": "path-a"}'
chrome-browser killFork '{"fork": "path-b"}'
```

## Sandbox Mode

For reproducible testing without cached logins:

```
chrome-browser newSession '{"url": "https://example.com", "sandbox": true}'
```

This opens a private window with no cookies or cached data.
"##;
    println!("{}", docs);
}
