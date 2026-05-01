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

interface TabInfo {
  id: number;
  window_id: number;
  url: string;
  title: string;
  pinned: boolean;
  active: boolean;
  discarded: boolean;
  group_id: number | null;
}

interface ExtensionStatus {
  connected: boolean;
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

function TabRow(props: { tab: TabInfo }) {
  return (
    <div class="flex items-center gap-2 py-2 border-b border-base-200 last:border-0">
      <Show when={props.tab.pinned}>
        <span class="badge badge-soft badge-info badge-xs shrink-0">pinned</span>
      </Show>
      <Show when={props.tab.active}>
        <span class="badge badge-soft badge-success badge-xs shrink-0">active</span>
      </Show>
      <Show when={props.tab.discarded}>
        <span class="badge badge-soft badge-neutral badge-xs shrink-0">discarded</span>
      </Show>
      <Show when={props.tab.group_id !== null}>
        <span class="badge badge-soft badge-secondary badge-xs shrink-0 font-mono">
          g{props.tab.group_id}
        </span>
      </Show>
      <div class="flex flex-col flex-1 min-w-0">
        <span class="text-sm truncate" title={props.tab.title}>
          {props.tab.title || "(no title)"}
        </span>
        <a
          href={props.tab.url}
          target="_blank"
          rel="noopener noreferrer"
          class="text-xs text-base-content/50 hover:text-primary truncate"
          title={props.tab.url}
        >
          {props.tab.url}
        </a>
      </div>
    </div>
  );
}

export default function App() {
  const [version, setVersion] = createSignal(0);
  const [extensionConnected, setExtensionConnected] = createSignal(false);
  const [tabs, setTabs] = createSignal<TabInfo[]>([]);

  const [browsers, { mutate: setBrowsers }] = createResource(async () => {
    const res = await fetch("/api/browsers/running?since=0");
    if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
    const data: BrowserStateSnapshot = await res.json();
    setVersion(data.version);
    return data.browsers;
  });

  onMount(() => {
    let active = true;

    // Long-poll browser running state
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

    // Poll extension status and tabs
    (async function pollExtension() {
      while (active) {
        try {
          const statusRes = await fetch("/api/extension/status");
          if (statusRes.ok) {
            const status: ExtensionStatus = await statusRes.json();
            setExtensionConnected(status.connected);
            if (status.connected) {
              const tabsRes = await fetch("/api/extension/tabs");
              if (tabsRes.ok) setTabs(await tabsRes.json());
            } else {
              setTabs([]);
            }
          }
        } catch { /* backend not ready yet */ }
        await new Promise((r) => setTimeout(r, 3000));
      }
    })();

    onCleanup(() => { active = false; });
  });

  return (
    <main class="min-h-screen bg-base-200 p-8">
      <div class="mx-auto max-w-2xl flex flex-col gap-6">

        {/* Extension status */}
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

        {/* Install prompt */}
        <Show when={!extensionConnected()}>
          <div role="alert" class="alert alert-info">
            <span>
              Install the companion extension in your browser, then reload this page.
              The extension connects automatically to this backend on port 36960.
            </span>
          </div>
        </Show>

        {/* Tab list */}
        <Show when={extensionConnected()}>
          <div class="bg-base-100 border border-base-300 rounded-xl p-4">
            <h2 class="text-lg font-semibold mb-3 flex items-center gap-2">
              Open Tabs
              <span class="badge badge-ghost badge-sm">{tabs().length}</span>
            </h2>
            <Show
              when={tabs().length > 0}
              fallback={<p class="text-sm text-base-content/50">No tabs reported yet.</p>}
            >
              <For each={tabs()}>{(tab) => <TabRow tab={tab} />}</For>
            </Show>
          </div>
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
