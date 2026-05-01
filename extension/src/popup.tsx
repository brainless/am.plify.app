import { createSignal, onCleanup, onMount, Show } from "solid-js";
import { render } from "solid-js/web";

function App() {
  const [connected, setConnected] = createSignal(false);
  const [tabCount, setTabCount] = createSignal(0);

  onMount(() => {
    chrome.storage.local.get(["connected", "tabCount"], (result) => {
      setConnected(result.connected ?? false);
      setTabCount(result.tabCount ?? 0);
    });

    const listener = (
      changes: Record<string, chrome.storage.StorageChange>
    ) => {
      if ("connected" in changes) setConnected(changes.connected.newValue ?? false);
      if ("tabCount" in changes) setTabCount(changes.tabCount.newValue ?? 0);
    };

    chrome.storage.onChanged.addListener(listener);
    onCleanup(() => chrome.storage.onChanged.removeListener(listener));
  });

  return (
    <div style={styles.root}>
      <div style={styles.header}>
        <span style={styles.logo}>am.plify</span>
        <Show
          when={connected()}
          fallback={
            <span style={{ ...styles.badge, ...styles.badgeError }}>
              ● disconnected
            </span>
          }
        >
          <span style={{ ...styles.badge, ...styles.badgeOk }}>
            ● connected
          </span>
        </Show>
      </div>

      <Show when={connected()}>
        <div style={styles.stat}>
          {tabCount()} tab{tabCount() !== 1 ? "s" : ""} tracked
        </div>
      </Show>

      <Show when={!connected()}>
        <div style={styles.hint}>
          Open the am.plify desktop app to connect.
        </div>
      </Show>
    </div>
  );
}

const styles: Record<string, Record<string, string>> = {
  root: {
    "min-width": "220px",
    padding: "14px 16px",
    "font-family": "system-ui, -apple-system, sans-serif",
    "font-size": "13px",
    color: "#e2e8f0",
    background: "#1e293b",
  },
  header: {
    display: "flex",
    "align-items": "center",
    "justify-content": "space-between",
    "margin-bottom": "8px",
  },
  logo: {
    "font-weight": "600",
    "font-size": "15px",
    "letter-spacing": "-0.3px",
  },
  badge: {
    "font-size": "11px",
    padding: "2px 7px",
    "border-radius": "999px",
    "font-weight": "500",
  },
  badgeOk: {
    background: "#14532d",
    color: "#86efac",
  },
  badgeError: {
    background: "#450a0a",
    color: "#fca5a5",
  },
  stat: {
    color: "#94a3b8",
    "font-size": "12px",
  },
  hint: {
    color: "#64748b",
    "font-size": "12px",
    "line-height": "1.5",
  },
};

render(() => <App />, document.getElementById("root")!);
