import { createResource, createSignal, For, Match, onCleanup, onMount, Show, Switch } from "solid-js";

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

interface LaunchBrowserRequest {
  kind: BrowserKind;
  executable: string;
  user_data_dir: string;
  profile_id: string;
  profile_path: string;
  debug_port: number | null;
}

interface LaunchBrowserResponse {
  debug_port: number;
  pid: number;
}

interface ConnectBrowserRequest {
  kind: BrowserKind;
  debug_port: number;
}

interface ConnectBrowserResponse {
  connection_id: string;
}

interface TabInfo {
  context_id: string;
  url: string;
  title: string;
  is_reddit: boolean;
}

interface RedditLoginStatus {
  context_id: string;
  is_logged_in: boolean;
  username: string | null;
}

interface BrowserStateSnapshot {
  browsers: DetectedBrowser[];
  version: number;
}

type LaunchState =
  | { tag: "idle" }
  | { tag: "launching" }
  | { tag: "launched"; port: number }
  | { tag: "error"; message: string };

type ConnectState =
  | { tag: "idle" }
  | { tag: "connecting" }
  | { tag: "connected"; connectionId: string }
  | { tag: "error"; message: string };

interface BrowserTab {
  tab: TabInfo;
  loginStatus: RedditLoginStatus | null;
  browserLabel: string;
  profileName: string;
  connectionId: string;
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

async function postLaunch(req: LaunchBrowserRequest): Promise<LaunchBrowserResponse> {
  const res = await fetch("/api/browsers/launch", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(req),
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

async function postConnect(req: ConnectBrowserRequest): Promise<ConnectBrowserResponse> {
  const res = await fetch("/api/browsers/connect", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(req),
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

async function getTabs(connectionId: string): Promise<TabInfo[]> {
  const res = await fetch(`/api/browsers/${connectionId}/tabs`);
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

async function inspectTab(connectionId: string, contextId: string): Promise<RedditLoginStatus> {
  const res = await fetch(`/api/browsers/${connectionId}/tabs/inspect`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ context_id: contextId }),
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

function ProfileRow(props: {
  browser: DetectedBrowser;
  profile: BrowserProfile;
  onLaunched?: (kind: BrowserKind, port: number) => void;
  onConnected?: (kind: BrowserKind, connectionId: string) => void;
}) {
  const [launch, setLaunch] = createSignal<LaunchState>({ tag: "idle" });
  const [connect, setConnect] = createSignal<ConnectState>({ tag: "idle" });

  const canLaunch = () =>
    props.browser.kind !== "safari" && !props.profile.is_running;

  const canConnect = () =>
    props.browser.kind !== "safari" && props.profile.is_running;

  async function startBrowser() {
    setLaunch({ tag: "launching" });
    try {
      const data = await postLaunch({
        kind: props.browser.kind,
        executable: props.browser.executable,
        user_data_dir: props.browser.user_data_dir,
        profile_id: props.profile.id,
        profile_path: props.profile.path,
        debug_port: null,
      });
      setLaunch({ tag: "launched", port: data.debug_port });
      props.onLaunched?.(props.browser.kind, data.debug_port);
    } catch (e) {
      setLaunch({ tag: "error", message: String(e) });
    }
  }

  async function connectBrowser() {
    setConnect({ tag: "connecting" });
    try {
      const port = prompt("Enter the browser's remote-debugging port:");
      if (!port) {
        setConnect({ tag: "idle" });
        return;
      }
      const debugPort = parseInt(port, 10);
      if (isNaN(debugPort)) {
        setConnect({ tag: "error", message: "Invalid port number" });
        return;
      }
      const conn = await postConnect({ kind: props.browser.kind, debug_port: debugPort });
      setConnect({ tag: "connected", connectionId: conn.connection_id });
      props.onConnected?.(props.browser.kind, conn.connection_id);
    } catch (e) {
      setConnect({ tag: "error", message: String(e) });
    }
  }

  return (
    <li class="flex items-center gap-3 py-2 border-b border-base-200 last:border-0">
      <span class="badge badge-ghost badge-sm shrink-0">{props.profile.name}</span>

      <Show when={props.profile.is_running}>
        <span class="badge badge-soft badge-success badge-sm shrink-0">running</span>
      </Show>

      <span class="text-xs text-base-content/50 font-mono break-all flex-1">
        {props.profile.path}
      </span>

      <Show when={canConnect()}>
        <Switch>
          <Match when={connect().tag === "idle"}>
            <button class="btn btn-xs btn-secondary shrink-0" onClick={connectBrowser}>
              Connect
            </button>
          </Match>
          <Match when={connect().tag === "connecting"}>
            <button class="btn btn-xs btn-secondary shrink-0" disabled>
              <span class="loading loading-spinner loading-xs" />
            </button>
          </Match>
          <Match when={connect().tag === "connected"}>
            <span class="badge badge-soft badge-success badge-sm shrink-0 font-mono cursor-default">
              Connected
            </span>
          </Match>
          <Match when={connect().tag === "error"}>
            <div
              class="tooltip tooltip-left"
              data-tip={(connect() as { tag: "error"; message: string }).message}
            >
              <button class="btn btn-xs btn-error shrink-0" onClick={connectBrowser}>
                Retry
              </button>
            </div>
          </Match>
        </Switch>
      </Show>

      <Show when={canLaunch()}>
        <Switch>
          <Match when={launch().tag === "idle"}>
            <button class="btn btn-xs btn-primary shrink-0" onClick={startBrowser}>
              Start
            </button>
          </Match>

          <Match when={launch().tag === "launching"}>
            <button class="btn btn-xs btn-primary shrink-0" disabled>
              <span class="loading loading-spinner loading-xs" />
            </button>
          </Match>

          <Match when={launch().tag === "launched"}>
            <span
              class="badge badge-soft badge-success badge-sm shrink-0 font-mono cursor-default"
              title={`BiDi WebSocket on port ${(launch() as { tag: "launched"; port: number }).port}`}
            >
              BiDi :{(launch() as { tag: "launched"; port: number }).port}
            </span>
          </Match>

          <Match when={launch().tag === "error"}>
            <div
              class="tooltip tooltip-left"
              data-tip={(launch() as { tag: "error"; message: string }).message}
            >
              <button class="btn btn-xs btn-error shrink-0" onClick={startBrowser}>
                Retry
              </button>
            </div>
          </Match>
        </Switch>
      </Show>
    </li>
  );
}

function BrowserTabRow(props: { tab: BrowserTab }) {
  const status = () => props.tab.loginStatus;
  const isReddit = () => props.tab.tab.is_reddit;

  return (
    <div class="flex items-center gap-3 py-2 border-b border-base-200 last:border-0">
      <span class="badge badge-ghost badge-sm shrink-0">{props.tab.profileName}</span>
      <span class="text-xs text-base-content/60 shrink-0 w-24">{props.tab.browserLabel}</span>

      <Show when={isReddit()}>
        <span class="badge badge-soft badge-warning badge-xs shrink-0">reddit</span>
      </Show>

      <a
        href={props.tab.tab.url}
        target="_blank"
        rel="noopener noreferrer"
        class="text-sm text-primary hover:underline truncate flex-1"
        title={props.tab.tab.url}
      >
        {props.tab.tab.url}
      </a>

      <Show when={isReddit() && status()}>
        <Show when={status()!.is_logged_in} fallback={
          <span class="badge badge-soft badge-error badge-sm shrink-0">Logged out</span>
        }>
          <span class="badge badge-soft badge-success badge-sm shrink-0">
            Logged in
            <Show when={status()!.username}>
              {" "}{status()!.username}
            </Show>
          </span>
        </Show>
      </Show>

      <Show when={isReddit() && !status()}>
        <span class="loading loading-spinner loading-xs shrink-0" />
      </Show>
    </div>
  );
}

export default function App() {
  const [version, setVersion] = createSignal(0);
  const [browserTabs, setBrowserTabs] = createSignal<BrowserTab[]>([]);
  const [scanErrors, setScanErrors] = createSignal<string[]>([]);

  const [browsers, { mutate: setBrowsers }] = createResource(async () => {
    const res = await fetch("/api/browsers/running?since=0");
    if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
    const data: BrowserStateSnapshot = await res.json();
    setVersion(data.version);
    return data.browsers;
  });

  onMount(() => {
    let active = true;

    (async function poll() {
      while (active && browsers.loading) {
        await new Promise((r) => setTimeout(r, 100));
      }
      if (!active) return;

      while (active) {
        try {
          const res = await fetch(`/api/browsers/running?since=${version()}`);
          if (!res.ok) {
            await new Promise((r) => setTimeout(r, 5000));
            continue;
          }
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

    onCleanup(() => {
      active = false;
    });
  });

  async function handleLaunched(kind: BrowserKind, port: number) {
    try {
      const conn = await postConnect({ kind, debug_port: port });
      await scanTabs(kind, conn.connection_id, BROWSER_LABEL[kind] || kind, "Default");
    } catch (e) {
      setScanErrors((prev) => [...prev, `Failed to connect: ${String(e)}`]);
    }
  }

  async function handleConnected(kind: BrowserKind, connectionId: string) {
    await scanTabs(kind, connectionId, BROWSER_LABEL[kind] || kind, "Default");
  }

  async function scanTabs(kind: BrowserKind, connectionId: string, browserLabel: string, profileName: string) {
    try {
      const tabs = await getTabs(connectionId);
      if (tabs.length === 0) {
        setScanErrors((prev) => [...prev, `No tabs found for ${browserLabel}`]);
        return;
      }

      const newTabs: BrowserTab[] = tabs.map((tab) => ({
        tab,
        loginStatus: null,
        browserLabel,
        profileName,
        connectionId,
      }));

      setBrowserTabs((prev) => {
        const existingIds = new Set(newTabs.map((t) => t.tab.context_id));
        const kept = prev.filter((t) => !existingIds.has(t.tab.context_id));
        return [...kept, ...newTabs];
      });

      for (const bt of newTabs) {
        if (bt.tab.is_reddit) {
          try {
            const status = await inspectTab(connectionId, bt.tab.context_id);
            setBrowserTabs((prev) =>
              prev.map((t) =>
                t.tab.context_id === bt.tab.context_id ? { ...t, loginStatus: status } : t
              )
            );
          } catch (e) {
            setScanErrors((prev) => [...prev, `Failed to inspect Reddit tab: ${String(e)}`]);
          }
        }
      }
    } catch (e) {
      setScanErrors((prev) => [...prev, `Failed to scan tabs: ${String(e)}`]);
    }
  }

  return (
    <main class="min-h-screen bg-base-200 p-8">
      <div class="mx-auto max-w-2xl">
        <h1 class="text-2xl font-bold mb-6">Detected Browsers</h1>

        <Show when={browsers.loading}>
          <div class="flex justify-center py-16">
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
                  <span
                    class={`badge badge-soft ${BROWSER_BADGE_COLOR[browser.kind]} shrink-0`}
                  >
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
                    fallback={
                      <p class="text-sm text-base-content/50 py-1">No profiles found.</p>
                    }
                  >
                    <ul class="flex flex-col">
                      <For each={browser.profiles}>
                        {(profile) => (
                          <ProfileRow
                            browser={browser}
                            profile={profile}
                            onLaunched={handleLaunched}
                            onConnected={handleConnected}
                          />
                        )}
                      </For>
                    </ul>
                  </Show>
                </div>
              </details>
            )}
          </For>
        </div>

        <Show when={scanErrors().length > 0}>
          <div class="mt-8">
            <h2 class="text-xl font-bold mb-4 flex items-center gap-2">
              <span class="status status-error status-sm" />
              Scan Errors
            </h2>
            <div class="bg-base-100 border border-base-300 rounded-xl p-4">
              <For each={scanErrors()}>
                {(err) => (
                  <div class="text-sm text-error py-1 font-mono break-all">{err}</div>
                )}
              </For>
            </div>
          </div>
        </Show>

        <Show when={browserTabs().length > 0}>
          <div class="mt-8">
            <h2 class="text-xl font-bold mb-4 flex items-center gap-2">
              <span class="status status-success status-sm" />
              Open Tabs
            </h2>

            <div class="bg-base-100 border border-base-300 rounded-xl p-4">
              <For each={browserTabs()}>
                {(tab) => <BrowserTabRow tab={tab} />}
              </For>
            </div>
          </div>
        </Show>
      </div>
    </main>
  );
}
