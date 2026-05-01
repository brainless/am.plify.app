import type { TabInfo, FromExtension, ToExtension } from "./types";

const WS_URL = "ws://localhost:36960/api/extension/ws";

let socket: WebSocket | null = null;
let reconnectDelay = 1000;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

function detectBrowser(): string {
  const ua = navigator.userAgent;
  if (ua.includes("Edg/")) return "edge";
  if (ua.includes("Brave")) return "brave";
  if (ua.includes("Chrome")) return "chrome";
  if (ua.includes("Firefox")) return "firefox";
  return "unknown";
}

async function getAllTabs(): Promise<TabInfo[]> {
  const tabs = await chrome.tabs.query({});
  return tabs.map((tab) => ({
    id: tab.id ?? -1,
    window_id: tab.windowId ?? -1,
    url: tab.url ?? "",
    title: tab.title ?? "",
    pinned: tab.pinned,
    active: tab.active,
    discarded: tab.discarded ?? false,
    group_id:
      tab.groupId !== undefined && tab.groupId >= 0 ? tab.groupId : null,
  }));
}

function send(msg: FromExtension) {
  if (socket?.readyState === WebSocket.OPEN) {
    socket.send(JSON.stringify(msg));
  }
}

async function pushTabs() {
  const tabs = await getAllTabs();
  send({ type: "tabList", tabs });
  chrome.storage.local.set({ tabCount: tabs.length });
}

function setConnected(connected: boolean) {
  chrome.storage.local.set({ connected });
}

function connect() {
  if (reconnectTimer !== null) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }

  socket = new WebSocket(WS_URL);

  socket.onopen = async () => {
    reconnectDelay = 1000;
    send({ type: "connected", browser: detectBrowser() });
    await pushTabs();
    setConnected(true);
  };

  socket.onmessage = async (event) => {
    let msg: ToExtension;
    try {
      msg = JSON.parse(event.data as string);
    } catch {
      return;
    }
    if (msg.type === "listTabs") {
      await pushTabs();
    }
    // readDom, scrollTab, activateTab, openTab handled in future phases
  };

  socket.onclose = () => {
    setConnected(false);
    scheduleReconnect();
  };

  socket.onerror = () => {
    socket?.close();
  };
}

function scheduleReconnect() {
  reconnectTimer = setTimeout(() => connect(), reconnectDelay);
  reconnectDelay = Math.min(reconnectDelay * 2, 30_000);
}

// Push fresh tab list on any tab lifecycle event
const pushDebounceMs = 300;
let pushTimer: ReturnType<typeof setTimeout> | null = null;
function debouncedPush() {
  if (pushTimer !== null) clearTimeout(pushTimer);
  pushTimer = setTimeout(() => pushTabs(), pushDebounceMs);
}

chrome.tabs.onCreated.addListener(debouncedPush);
chrome.tabs.onRemoved.addListener(debouncedPush);
chrome.tabs.onActivated.addListener(debouncedPush);
chrome.tabs.onUpdated.addListener((_id, changeInfo) => {
  if (changeInfo.status === "complete" || changeInfo.title || changeInfo.url) {
    debouncedPush();
  }
});

connect();
