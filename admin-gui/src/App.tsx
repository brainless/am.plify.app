import { createResource, createSignal, For, onCleanup, onMount, Show } from "solid-js";

type BrowserKind =
  | "chrome"
  | "chromium"
  | "brave"
  | "edge"
  | "firefox"
  | "firefox_developer_edition"
  | "firefox_nightly"
  | "safari";

interface BrowserProfile {
  id: string;
  name: string;
  path: string;
  is_running: boolean;
}

interface DetectedBrowser {
  kind: BrowserKind;
  executable: string;
  user_data_dir: string;
  profiles: BrowserProfile[];
  is_running: boolean;
}

interface BrowserStateSnapshot {
  browsers: DetectedBrowser[];
  version: number;
}

interface ExtensionStatus {
  connected: boolean;
}

// Tauri globals — available because tauri.conf.json sets withGlobalTauri: true
declare const window: Window & {
  __TAURI__?: {
    core: { invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown> };
    opener?: { openUrl: (url: string) => Promise<void> };
    shell?: { open: (url: string) => Promise<void> };
  };
};

function isTauri(): boolean {
  return typeof window.__TAURI__ !== "undefined";
}

async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return window.__TAURI__!.core.invoke(cmd, args) as Promise<T>;
}

async function tauriOpen(url: string): Promise<void> {
  const t = window.__TAURI__!;
  if (t.opener) return t.opener.openUrl(url);
  if (t.shell) return t.shell.open(url);
}

// URL that shows the extension management page for a given browser kind
function extensionsPageUrl(kind: BrowserKind): string {
  switch (kind) {
    case "brave": return "brave://extensions";
    case "edge": return "edge://extensions";
    case "firefox":
    case "firefox_developer_edition":
    case "firefox_nightly":
      return "about:debugging#/runtime/this-firefox";
    default: return "chrome://extensions";
  }
}

const BROWSER_LABEL: Record<BrowserKind, string> = {
  chrome: "Google Chrome",
  chromium: "Chromium",
  brave: "Brave",
  edge: "Microsoft Edge",
  firefox: "Firefox",
  firefox_developer_edition: "Firefox Developer Edition",
  firefox_nightly: "Firefox Nightly",
  safari: "Safari",
};

const BROWSER_BADGE_COLOR: Record<BrowserKind, string> = {
  chrome: "badge-info",
  chromium: "badge-neutral",
  brave: "badge-success",
  edge: "badge-secondary",
  firefox: "badge-warning",
  firefox_developer_edition: "badge-warning",
  firefox_nightly: "badge-warning",
  safari: "badge-primary",
};

function isFirefox(kind: BrowserKind): boolean {
  return kind === "firefox" || kind === "firefox_developer_edition" || kind === "firefox_nightly";
}

function ChromeInstallSteps(props: {
  browser: DetectedBrowser;
  extensionPath: string | null;
  onOpenPage: () => void;
  onCopy: () => void;
  copied: boolean;
}) {
  return (
    <div class="flex flex-col gap-3">
      <ol class="text-sm text-base-content/70 flex flex-col gap-1.5 list-decimal list-inside">
        <li>Open the extensions page</li>
        <li>Enable <strong>Developer mode</strong> (toggle, top-right)</li>
        <li>Click <strong>Load unpacked</strong> — select the folder below</li>
      </ol>
      <Show when={props.extensionPath}>
        <div class="flex items-center gap-2 bg-base-200 rounded-lg px-3 py-2">
          <code class="text-xs font-mono flex-1 truncate text-base-content/70">
            {props.extensionPath}
          </code>
          <button class="btn btn-xs btn-ghost shrink-0" onClick={props.onCopy}>
            {props.copied ? "Copied!" : "Copy path"}
          </button>
        </div>
      </Show>
      <button
        class={`btn btn-sm btn-soft ${BROWSER_BADGE_COLOR[props.browser.kind]} self-start`}
        onClick={props.onOpenPage}
      >
        Open {BROWSER_LABEL[props.browser.kind]} Extensions
      </button>
    </div>
  );
}

function FirefoxInstallSteps(props: {
  browser: DetectedBrowser;
  extensionPath: string | null;
  xpiPath: string | null;
  xpiError: string | null;
  onOpenDebugging: () => void;
  onOpenAddons: () => void;
  onPackageXpi: () => void;
  onCopyManifestPath: () => void;
  copiedManifest: boolean;
}) {
  const manifestPath = () =>
    props.extensionPath ? props.extensionPath + "/manifest.json" : null;

  return (
    <div class="flex flex-col gap-4">
      {/* Option A */}
      <div class="flex flex-col gap-2">
        <p class="text-sm font-medium">Option A — Temporary (resets on browser restart)</p>
        <ol class="text-sm text-base-content/70 flex flex-col gap-1.5 list-decimal list-inside">
          <li>Open <code class="font-mono bg-base-200 px-1 rounded">about:debugging</code></li>
          <li>Click <strong>This Firefox</strong></li>
          <li>Click <strong>Load Temporary Add-on…</strong></li>
          <li>Select the <code class="font-mono bg-base-200 px-1 rounded">manifest.json</code> below</li>
        </ol>
        <Show when={manifestPath()}>
          <div class="flex items-center gap-2 bg-base-200 rounded-lg px-3 py-2">
            <code class="text-xs font-mono flex-1 truncate text-base-content/70">
              {manifestPath()}
            </code>
            <button class="btn btn-xs btn-ghost shrink-0" onClick={props.onCopyManifestPath}>
              {props.copiedManifest ? "Copied!" : "Copy path"}
            </button>
          </div>
        </Show>
        <button
          class="btn btn-sm btn-soft badge-warning self-start"
          onClick={props.onOpenDebugging}
        >
          Open about:debugging in {BROWSER_LABEL[props.browser.kind]}
        </button>
      </div>

      <div class="divider text-xs text-base-content/40 my-0">or</div>

      {/* Option B */}
      <div class="flex flex-col gap-2">
        <p class="text-sm font-medium">
          Option B — Permanent{" "}
          <span class="text-xs font-normal text-base-content/50">(Firefox Developer Edition only, no signing needed)</span>
        </p>
        <ol class="text-sm text-base-content/70 flex flex-col gap-1.5 list-decimal list-inside">
          <li>Package the extension as a <code class="font-mono bg-base-200 px-1 rounded">.xpi</code> file</li>
          <li>
            In Firefox: open <code class="font-mono bg-base-200 px-1 rounded">about:addons</code>{" "}
            → gear icon → <strong>Install Add-on From File</strong>
          </li>
          <li>Select the packaged <code class="font-mono bg-base-200 px-1 rounded">.xpi</code></li>
        </ol>

        <Show when={props.xpiPath}>
          <div class="flex items-center gap-2 bg-base-200 rounded-lg px-3 py-2">
            <code class="text-xs font-mono flex-1 truncate text-base-content/70">
              {props.xpiPath}
            </code>
          </div>
        </Show>
        <Show when={props.xpiError}>
          <p class="text-xs text-error">{props.xpiError}</p>
        </Show>

        <div class="flex gap-2 flex-wrap">
          <button class="btn btn-sm btn-soft badge-warning self-start" onClick={props.onPackageXpi}>
            Package as .xpi
          </button>
          <Show when={props.xpiPath}>
            <button class="btn btn-sm btn-soft self-start" onClick={props.onOpenAddons}>
              Open about:addons in {BROWSER_LABEL[props.browser.kind]}
            </button>
          </Show>
        </div>
      </div>
    </div>
  );
}

function InstallBanner(props: { browsers: DetectedBrowser[] }) {
  const [extensionPath, setExtensionPath] = createSignal<string | null>(null);
  const [pathError, setPathError] = createSignal<string | null>(null);
  const [copiedPath, setCopiedPath] = createSignal(false);
  const [copiedManifest, setCopiedManifest] = createSignal(false);
  const [xpiPath, setXpiPath] = createSignal<string | null>(null);
  const [xpiError, setXpiError] = createSignal<string | null>(null);

  const inTauri = isTauri();
  const runningBrowsers = () =>
    props.browsers.filter((b) => b.is_running && b.kind !== "safari");

  onMount(async () => {
    if (!inTauri) return;
    try {
      setExtensionPath(await tauriInvoke<string>("get_extension_path"));
    } catch (e) {
      setPathError(String(e));
    }
  });

  const copyPath = async () => {
    const path = extensionPath();
    if (!path) return;
    await navigator.clipboard.writeText(path);
    setCopiedPath(true);
    setTimeout(() => setCopiedPath(false), 2000);
  };

  const copyManifestPath = async () => {
    const path = extensionPath();
    if (!path) return;
    await navigator.clipboard.writeText(path + "/manifest.json");
    setCopiedManifest(true);
    setTimeout(() => setCopiedManifest(false), 2000);
  };

  const openExtensionsPage = async (browser: DetectedBrowser) => {
    if (!inTauri) return;
    await tauriInvoke("open_extensions_page", {
      browser: browser.kind,
      executable: browser.executable,
    });
  };

  const openAddons = async (browser: DetectedBrowser) => {
    if (!inTauri) return;
    await tauriInvoke("open_url_in_browser", {
      url: "about:addons",
      executable: browser.executable,
    });
  };

  const packageXpi = async () => {
    setXpiError(null);
    try {
      const path = await tauriInvoke<string>("package_extension_xpi");
      setXpiPath(path);
    } catch (e) {
      setXpiError(String(e));
    }
  };

  return (
    <div class="card bg-base-100 border border-warning/40">
      <div class="card-body gap-4 py-5">
        <div class="flex items-start gap-3">
          <span class="text-warning text-xl mt-0.5">⚠</span>
          <div>
            <p class="font-semibold">Extension not connected</p>
            <p class="text-sm text-base-content/60 mt-1">
              Install the am.plify browser extension — it connects to this app automatically.
            </p>
          </div>
        </div>

        <Show when={pathError()}>
          <p class="text-xs text-error">{pathError()}</p>
        </Show>

        <Show when={inTauri} fallback={
          <p class="text-sm text-base-content/50">
            Open this app via the Amplify desktop app for guided installation.
          </p>
        }>
          <Show
            when={runningBrowsers().length > 0}
            fallback={
              <p class="text-xs text-base-content/50">
                No running browsers detected — launch a browser first, then reload.
              </p>
            }
          >
            <For each={runningBrowsers()}>
              {(browser) => (
                <div class="flex flex-col gap-3">
                  <p class="text-sm font-semibold flex items-center gap-2">
                    <span class={`badge badge-soft badge-sm ${BROWSER_BADGE_COLOR[browser.kind]}`}>
                      {BROWSER_LABEL[browser.kind]}
                    </span>
                  </p>
                  <Show
                    when={isFirefox(browser.kind)}
                    fallback={
                      <ChromeInstallSteps
                        browser={browser}
                        extensionPath={extensionPath()}
                        onOpenPage={() => openExtensionsPage(browser)}
                        onCopy={copyPath}
                        copied={copiedPath()}
                      />
                    }
                  >
                    <FirefoxInstallSteps
                      browser={browser}
                      extensionPath={extensionPath()}
                      xpiPath={xpiPath()}
                      xpiError={xpiError()}
                      onOpenDebugging={() => openExtensionsPage(browser)}
                      onOpenAddons={() => openAddons(browser)}
                      onPackageXpi={packageXpi}
                      onCopyManifestPath={copyManifestPath}
                      copiedManifest={copiedManifest()}
                    />
                  </Show>
                </div>
              )}
            </For>
          </Show>
        </Show>
      </div>
    </div>
  );
}

export default function App() {
  const [version, setVersion] = createSignal(0);
  const [extensionConnected, setExtensionConnected] = createSignal(false);

  const [browsers, { mutate: setBrowsers }] = createResource(async () => {
    const res = await fetch("/api/browsers/running?since=0");
    if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
    const data: BrowserStateSnapshot = await res.json();
    setVersion(data.version);
    return data.browsers;
  });

  onMount(() => {
    let active = true;

    (async function pollBrowsers() {
      while (active && browsers.loading) {
        await new Promise((r) => setTimeout(r, 100));
      }
      if (!active) return;
      while (active) {
        try {
          const res = await fetch(`/api/browsers/running?since=${version()}`);
          if (!res.ok) { await new Promise((r) => setTimeout(r, 5000)); continue; }
          const data: BrowserStateSnapshot = await res.json();
          if (data.version !== version()) {
            setVersion(data.version);
            setBrowsers(data.browsers);
          }
        } catch {
          await new Promise((r) => setTimeout(r, 5000));
        }
      }
    })();

    (async function pollExtension() {
      while (active) {
        try {
          const res = await fetch("/api/extension/status");
          if (res.ok) {
            const status: ExtensionStatus = await res.json();
            setExtensionConnected(status.connected);
          }
        } catch { /* backend not ready */ }
        await new Promise((r) => setTimeout(r, 3000));
      }
    })();

    onCleanup(() => { active = false; });
  });

  return (
    <main class="min-h-screen bg-base-200 p-8">
      <div class="mx-auto max-w-2xl flex flex-col gap-6">

        {/* Header */}
        <div class="flex items-center gap-3">
          <h1 class="text-2xl font-bold">am.plify</h1>
          <Show
            when={extensionConnected()}
            fallback={
              <span class="badge badge-soft badge-error gap-1">
                <span class="status status-error status-xs" />
                Extension not connected
              </span>
            }
          >
            <span class="badge badge-soft badge-success gap-1">
              <span class="status status-success status-xs" />
              Extension connected
            </span>
          </Show>
        </div>

        {/* Install guide */}
        <Show when={!extensionConnected() && !browsers.loading}>
          <InstallBanner browsers={browsers() ?? []} />
        </Show>

        {/* Detected browsers */}
        <div>
          <h2 class="text-lg font-semibold mb-3">Detected Browsers</h2>

          <Show when={browsers.loading}>
            <div class="flex justify-center py-8">
              <span class="loading loading-spinner loading-md" />
            </div>
          </Show>

          <Show when={browsers.error}>
            <div role="alert" class="alert alert-error">
              <span>Failed to load browsers: {String(browsers.error)}</span>
            </div>
          </Show>

          <Show when={!browsers.loading && !browsers.error && browsers()?.length === 0}>
            <div role="alert" class="alert alert-info">
              <span>No browsers detected on this machine.</span>
            </div>
          </Show>

          <div class="flex flex-col gap-3">
            <For each={browsers()}>
              {(browser) => (
                <details
                  class="collapse collapse-arrow bg-base-100 border border-base-300 rounded-xl"
                  open
                >
                  <summary class="collapse-title flex items-center gap-3 min-h-0 py-3">
                    <span class={`badge badge-soft ${BROWSER_BADGE_COLOR[browser.kind]} shrink-0`}>
                      {BROWSER_LABEL[browser.kind]}
                    </span>
                    <Show when={browser.is_running}>
                      <span class="badge badge-soft badge-success badge-sm shrink-0">running</span>
                    </Show>
                    <span class="text-xs text-base-content/50 font-mono truncate">
                      {browser.executable}
                    </span>
                  </summary>
                  <div class="collapse-content pb-2">
                    <Show
                      when={browser.profiles.length > 0}
                      fallback={<p class="text-sm text-base-content/50 py-1">No profiles found.</p>}
                    >
                      <ul class="flex flex-col">
                        <For each={browser.profiles}>
                          {(profile) => (
                            <li class="flex items-center gap-3 py-2 border-b border-base-200 last:border-0">
                              <span class="badge badge-ghost badge-sm shrink-0">{profile.name}</span>
                              <Show when={profile.is_running}>
                                <span class="badge badge-soft badge-success badge-sm shrink-0">running</span>
                              </Show>
                              <span class="text-xs text-base-content/50 font-mono break-all flex-1">
                                {profile.path}
                              </span>
                            </li>
                          )}
                        </For>
                      </ul>
                    </Show>
                  </div>
                </details>
              )}
            </For>
          </div>
        </div>

      </div>
    </main>
  );
}
