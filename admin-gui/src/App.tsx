import { createResource, createSignal, For, Match, Show, Switch } from "solid-js";

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

type LaunchState =
  | { tag: "idle" }
  | { tag: "launching" }
  | { tag: "launched"; port: number }
  | { tag: "error"; message: string };

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

async function fetchBrowsers(): Promise<DetectedBrowser[]> {
  const res = await fetch("/api/browsers");
  if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
  return res.json();
}

async function postLaunch(req: LaunchBrowserRequest): Promise<LaunchBrowserResponse> {
  const res = await fetch("/api/browsers/launch", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(req),
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

function ProfileRow(props: { browser: DetectedBrowser; profile: BrowserProfile }) {
  const [launch, setLaunch] = createSignal<LaunchState>({ tag: "idle" });

  const canLaunch = () =>
    props.browser.kind !== "safari" && !props.profile.is_running;

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
    } catch (e) {
      setLaunch({ tag: "error", message: String(e) });
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

export default function App() {
  const [browsers] = createResource(fetchBrowsers);

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
                          <ProfileRow browser={browser} profile={profile} />
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
    </main>
  );
}
