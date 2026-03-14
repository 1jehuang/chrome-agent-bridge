---
name: chrome-browser
description: Control the user's real Chrome browser with existing logins and cookies. Use for navigation, clicking, typing, screenshots, file upload, reading page content, and page-context JavaScript.
allowed-tools: Bash, Read, Write
---

# Chrome Agent Bridge

Use the `chrome-browser` CLI to control the user's actual Chrome session through the installed Chrome Agent Bridge extension.

## Setup checklist

Before first use:

1. Install the Chrome extension from the repo or Chrome Web Store.
2. Build the Rust binaries.
3. Install the native messaging host manifest.
4. Verify with `chrome-browser ping`.

## Core commands

```bash
chrome-browser ping
chrome-browser navigate '{"url":"https://example.com"}'
chrome-browser getContent '{"format":"annotated"}'
chrome-browser click '{"text":"Sign in"}'
chrome-browser type '{"selector":"input[name=q]","text":"hello","submit":true}'
chrome-browser fillForm '{"fields":[{"selector":"#email","value":"a@b.com"}]}'
chrome-browser screenshot '{}'
chrome-browser evaluate '{"script":"return document.title"}'
chrome-browser scroll '{"y":500}'
chrome-browser uploadFile '{"selector":"input[type=file]","path":"/tmp/file.pdf"}'
```

## Recommended workflow

1. `chrome-browser ping`
2. `chrome-browser navigate ...`
3. `chrome-browser getContent '{"format":"annotated"}'`
4. Use selectors or visible text from the annotated output for `click` / `type` / `fillForm`
5. Use `screenshot` when visual confirmation matters

## Notes

- This controls the **real Chrome browser**, not a headless copy.
- Authenticated sessions and cookies are preserved because actions happen inside the user's actual browser.
- Chrome Web Store pages themselves may restrict extension automation.
- `evaluate` runs in the page's main world and can return JSON-serializable values.

## Supported actions

- `ping`
- `navigate`
- `getContent`
- `getInteractables`
- `click`
- `type`
- `fillForm`
- `waitFor`
- `scroll`
- `evaluate`
- `screenshot`
- `listTabs`
- `newSession`
- `setActiveTab`
- `getActiveTab`
- `listFrames`
- `uploadFile`
- `dropFile`
- `reload`
- `fork`
- `killFork`
- `listForks`
- `parallel`
- `tryUntil`
