# Privacy Policy - Chrome Agent Bridge

**Last updated:** March 14, 2026

## Overview

Chrome Agent Bridge is a browser extension that enables AI coding agents to control Chrome browser tabs through a local native messaging connection. This extension is designed for developer productivity and automation.

## Data Collection

**Chrome Agent Bridge does not collect, transmit, or store any personal data.**

Specifically:
- No browsing history is collected or transmitted
- No cookies or session data are accessed beyond what is needed to execute the current command
- No user credentials are collected or stored
- No analytics or telemetry data is sent to any server
- No data is shared with third parties

## How It Works

All communication happens **locally on your machine**:

1. The extension receives commands from a native messaging host running on your computer
2. Commands are executed in the active browser tab (e.g., navigate to a URL, click a button, read page content)
3. Results are returned to the native messaging host
4. The native messaging host communicates with the CLI tool via a local WebSocket (127.0.0.1)

No data ever leaves your computer through this extension.

## Permissions

The extension requests the following permissions:

- **tabs**: To list, create, and switch between browser tabs
- **activeTab**: To interact with the currently active tab
- **nativeMessaging**: To communicate with the local native messaging host
- **scripting**: To execute JavaScript in web pages for automation
- **storage**: To store extension configuration locally
- **webNavigation**: To detect page load events
- **webRequest**: To monitor network requests for page load detection
- **downloads**: To handle file downloads
- **Host permissions (<all_urls>)**: To inject content scripts into any web page for automation

All permissions are used exclusively for local browser automation. No data accessed through these permissions is transmitted externally.

## Contact

For questions about this privacy policy, please open an issue at:
https://github.com/1jehuang/chrome-agent-bridge

## Changes

This privacy policy may be updated occasionally. Changes will be posted to the GitHub repository.
