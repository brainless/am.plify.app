# DEVELOP

## What This Is

A Tauri desktop app that automates installed browsers on the user's machine — detecting profiles, connecting via CDP (Chrome) and WebDriver BiDi (Firefox), and orchestrating tab/page interactions. The admin-gui is the primary UI; the Rust backend handles browser automation and data.

## Goals

- Detect installed browsers and their profiles (Chrome, Firefox) from the filesystem
- Re-launch browsers with remote debugging enabled (user confirms before relaunch)
- List open tabs, check if a URL/site is already open
- Read HTML, extract data, scroll pages, capture XHR/fetch responses
- Click elements, navigate URLs, open new tabs
- Surface extracted data via the admin-gui

## Tech Stack

| Layer | Tech |
|---|---|
| Desktop shell | Tauri 2.x |
| Backend | Rust + Actix-web |
| Browser automation | `rustenium` (BiDi + CDP, Chrome & Firefox) |
| Admin UI | SolidJS + Vite + TailwindCSS + DaisyUI |
| DB | SQLite (rusqlite + refinery migrations) |
| Shared types | `shared-types` crate → generated TypeScript via `ts-rs` |

## Browser Automation Notes

- Chrome: CDP via chromedriver. Re-launch with `--remote-debugging-port` + correct `--user-data-dir` + `--profile-directory`.
- Firefox: WebDriver BiDi natively — no geckodriver needed.
- Profile discovery: read `Local State` JSON (Chrome) or `profiles.ini` (Firefox) before re-launching.
- XHR response bodies: use `add_preload_script()` to inject a fetch/XHR monkey-patch since rustenium only intercepts at `BeforeRequestSent`. Re-evaluate if this proves too fragile.
- Background tab reads work for static HTML; activate the tab when scroll-triggered lazy loading is needed.
- `rustenium` is early-stage — if blocked, fallback is Playwright as a managed sidecar subprocess.

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

Tauri launches the backend sidecar and starts the admin-gui Vite dev server automatically.

## Configuration

Priority order: **env var → `project.conf` → `server.env`**

- `AMPLIFY_BACKEND_PATH` — override backend binary location (dev/testing)
- `project.conf` — local ports, DB path, browser config
- Ports: backend `36960`, admin-gui `36980`

## Structure

- `backend/` — Rust, Actix-web API + browser automation logic
- `admin-gui/` — SolidJS frontend (Tauri webview)
- `shared-types/` — Rust types → generated TypeScript
- `tauri/` — Tauri desktop shell (manages backend sidecar lifecycle)
- `tauri/scripts/dev-admin-gui.sh` — builds backend binary, starts Vite
