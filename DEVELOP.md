# DEVELOP

## What This Is

A Tauri desktop app that helps users scout and discover content from the web. A companion browser extension connects to the local backend, giving it access to open tabs, DOM content, and page scrolling — without relaunching the browser or triggering any remote-debugging UI.

## Goals

- Detect installed browsers and their profiles (Chrome, Firefox) from the filesystem
- List all open tabs with full metadata (title, URL, pinned, active, discarded, tab group)
- Read HTML/DOM content from any tab via injected content scripts
- Scroll pages to trigger infinite scroll (briefly activating tab when needed)
- Open new tabs, activate tabs
- Surface extracted content via the admin-gui for the user to act on

## Tech Stack

| Layer | Tech |
|---|---|
| Desktop shell | Tauri 2.x |
| Backend | Rust + Actix-web |
| Browser automation | WebExtension (JS/TS, Manifest V3, Chrome + Firefox) |
| Extension ↔ Backend | WebSocket to `localhost:PORT` |
| Admin UI | SolidJS + Vite + TailwindCSS + DaisyUI |
| DB | SQLite (rusqlite + refinery migrations) |
| Shared types | `shared-types` crate → generated TypeScript via `ts-rs` |

## Browser Extension

The extension is the sole automation layer. It replaces BiDi/CDP entirely — no debug ports, no red address bar, no `navigator.webdriver`.

### Architecture

- **Background service worker** (MV3): maintains a WebSocket connection to the backend, receives commands, dispatches to content scripts, reports results back.
- **Content scripts**: injected on demand into specific tabs to read DOM or scroll. Run in an isolated world — invisible to page JavaScript.

### Communication Flow

```
Admin GUI → Backend (REST) → Extension background (WebSocket) → Content script (tab)
                                                               ← DOM / scroll result
```

The backend is the command issuer. The extension background worker holds the persistent WebSocket connection and is the bridge into the browser.

### What the Extension Can Do

| Capability | API | Notes |
|---|---|---|
| List all tabs (incl. discarded) | `browser.tabs.query({})` | Returns pinned, active, groupId, discarded state |
| Tab groups | `chrome.tabGroups` (Chrome), Firefox partial | Chrome MV3 only for now |
| Activate a tab | `browser.tabs.update(id, { active: true })` | Needed for IntersectionObserver-based infinite scroll |
| Read DOM | `browser.scripting.executeScript()` in isolated world | Page cannot detect this |
| Scroll page | `document.scrollingElement.scrollTop` via content script | scroll-event sites work in background; IntersectionObserver sites need tab active |
| Open new tab | `browser.tabs.create({ url })` | |

### Scrolling Strategy

- **First**: inject content script and read whatever is already loaded. No activation needed.
- **Infinite scroll (scroll-event based)**: set `scrollTop` from background tab — works without activation.
- **Infinite scroll (IntersectionObserver based, e.g. Reddit new layout)**: briefly activate tab, scroll in increments, read new DOM, switch away. User can be offered a "gentle scroll" option that controls pacing.

### Detection Surface

- Extension itself: not detectable by pages (no web-accessible resources, no DOM injection at rest).
- DOM reading via content script: not detectable.
- Scroll via content script: `scrollTop` manipulation is the same as a user scrolling — no detectable difference.
- No `navigator.webdriver` flag set (unlike BiDi/CDP).

### User Installation

The extension must be installed by the user. The desktop app will guide them to the browser's extension page. For development, the extension is loaded unpacked directly from the repo.

Target: Chrome, Edge, Brave (same extension), Firefox (same codebase, minor API shim).

## Type-Driven Workflow

1. Define types in `shared-types/src/*.rs`
2. Regenerate TS: `cargo run -p shared-types --bin generate_api_types` → `admin-gui/src/types/api.ts`
3. Implement backend handler
4. Implement admin-gui against generated types

Start from `shared-types`, never UI-first.

## Running Locally

```bash
./run-amplify-app.sh   # installs tauri deps, runs cargo tauri dev
```

Load the extension unpacked from `extension/` in your browser's developer mode.

## Configuration

Priority order: **env var → `project.conf` → `server.env`**

- `AMPLIFY_BACKEND_PATH` — override backend binary location (dev/testing)
- `project.conf` — local ports, DB path
- Ports: backend `36960`, admin-gui `36980`

## Structure

- `backend/` — Rust, Actix-web API + content orchestration logic
- `admin-gui/` — SolidJS frontend (Tauri webview)
- `extension/` — Browser extension (MV3, Chrome + Firefox)
- `shared-types/` — Rust types → generated TypeScript
- `tauri/` — Tauri desktop shell (manages backend sidecar lifecycle)
- `tauri/scripts/dev-admin-gui.sh` — builds backend binary, starts Vite
