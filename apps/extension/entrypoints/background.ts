import { browser } from 'wxt/browser';
import { localReport, pendingBatch, acknowledge, record } from '../src/lib/db';
import { safeDomain } from '../src/lib/domain';

const HOST = 'dev.watcher.bridge';
const VERSION = 1;
let serial: Promise<void> = Promise.resolve();
let syncing: Promise<void> | undefined;
let online = false;

async function snapshot(recovered = false): Promise<void> {
  let domain: string | null = null;
  try {
    const idle = await browser.idle.queryState(60);
    const window = await browser.windows.getLastFocused();
    if (idle === 'active' && window.focused && !window.incognito && window.id !== undefined) {
      const tabs = await browser.tabs.query({ active: true, windowId: window.id });
      if (!tabs[0]?.incognito) domain = safeDomain(tabs[0]?.url);
    }
  } catch {
    // Browser observation unavailable: close the previous interval, credit nothing.
  }
  await record(domain, Date.now(), recovered);
}

function enqueue(recovered = false) {
  serial = serial.then(() => snapshot(recovered)).catch((error) => {
    console.error('Watcher failed to store a browser interval', error);
  });
}

async function nativeRequest(message: object): Promise<any> {
  return browser.runtime.sendNativeMessage(HOST, message);
}

async function syncOnce(): Promise<void> {
  // All interval writes must finish before identifying unacknowledged work.
  await serial;
  try {
    const batch = await pendingBatch();
    if (!batch.length) {
      const response = await nativeRequest({ op: 'status', version: VERSION });
      online = response?.kind === 'status';
      return;
    }
    const response = await nativeRequest({
      op: 'browser_batch', version: VERSION,
      intervals: batch.map(({ synced: _synced, ...interval }) => interval),
    });
    if (response?.kind !== 'browser_ack' || response.received !== batch.length) {
      online = false;
      return;
    }
    await acknowledge(batch.map((row) => row.seq));
    online = true;
  } catch {
    online = false;
    // Neither missing native host nor missing daemon may delete queued data.
  }
}

function sync() {
  syncing ??= syncOnce().finally(() => { syncing = undefined; });
  return syncing;
}

export default defineBackground(() => {
  browser.tabs.onActivated.addListener(() => enqueue());
  browser.tabs.onUpdated.addListener((_id, info) => {
    if (info.url !== undefined || info.status === 'complete') enqueue();
  });
  browser.tabs.onRemoved.addListener(() => enqueue());
  browser.windows.onFocusChanged.addListener(() => enqueue());
  browser.windows.onRemoved.addListener(() => enqueue());
  browser.idle.onStateChanged.addListener(() => enqueue());
  browser.alarms.onAlarm.addListener((alarm) => {
    if (alarm.name === 'watcher-observe') enqueue();
    if (alarm.name === 'watcher-sync') void sync();
  });
  browser.runtime.onMessage.addListener(async (message: { type?: string }) => {
    if (message?.type !== 'watcher-dashboard') return undefined;
    // A user opening or refreshing the sidepanel should be able to reconnect
    // immediately instead of waiting for the next one-minute alarm.
    await sync();
    const now = Date.now();
    const beginning = new Date(now);
    beginning.setHours(0, 0, 0, 0);
    const local = await localReport(beginning.getTime(), now);
    // Native host availability must not prevent rendering browser-only usage.
    let device: unknown = null;
    if (online) {
      try {
        const response = await nativeRequest({
          op: 'report', version: VERSION, from_ms: beginning.getTime(), to_ms: now,
        });
        if (response?.kind === 'report') device = response.report;
        else online = false;
      } catch { online = false; }
    }
    return { local, online, device };
  });
  browser.action.onClicked.addListener((tab) => {
    // WXT builds `side_panel` for Chrome and `sidebar_action` for Firefox.
    // The browser action remains optional: users can open the panel normally.
    if (!import.meta.env.FIREFOX) {
      // SidePanel.open must be called directly from the user gesture.
      if (tab.windowId !== undefined) {
        void browser.sidePanel?.open({ windowId: tab.windowId }).catch(() => {});
      }
    } else {
      // The Firefox sidebar API is missing from WXT's Chromium-derived typings.
      const firefox = browser as unknown as { sidebarAction: { open(): Promise<void> } };
      void firefox.sidebarAction.open().catch(() => {});
    }
  });
  void browser.alarms.create('watcher-observe', { periodInMinutes: 1 });
  void browser.alarms.create('watcher-sync', { periodInMinutes: 1 });
  enqueue(true);
  void sync();
});
