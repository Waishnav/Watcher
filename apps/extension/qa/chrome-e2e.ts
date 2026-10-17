/** Real, headless Chrome extension QA; never uses the user's browser profile.
 * Run after `cargo build --workspace` and `bun run build`.
 * Covers offline capture, isolated Native Messaging, restart, reconciliation
 * labels, and independently observed unfocused-window behavior.
 */
import { chromium, type BrowserContext, type Page } from 'playwright-core';
import { createServer } from 'node:http';
import { spawn, type ChildProcess } from 'node:child_process';
import { strict as assert } from 'node:assert';
import { existsSync, mkdirSync, mkdtempSync, writeFileSync, rmSync, chmodSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { once } from 'node:events';
import { setTimeout as sleep } from 'node:timers/promises';

const repo = resolve(import.meta.dir, '../../..');
const dist = resolve(repo, 'apps/extension/.output/chrome-mv3');
const host = resolve(repo, 'target/debug/watcher-native-host');
const daemon = resolve(repo, 'target/debug/watcherd');
const cli = resolve(repo, 'target/debug/watcher');
for (const path of [join(dist, 'manifest.json'), host, daemon, cli]) {
  assert(existsSync(path), `build required before QA: ${path}`);
}
// Branded Google Chrome builds can reject sideload flags; prefer Playwright's
// open-source Chromium for genuine extension background-worker tests.
const chromeExecutable = [process.env.CHROME_BIN, chromium.executablePath(), '/usr/bin/chromium']
  .find((item) => item && existsSync(item));
assert(chromeExecutable, 'Unbranded Chromium required; run: bunx playwright-core install chromium');

const qa = mkdtempSync(join(tmpdir(), 'watcher-browser-qa-'));
const profile = join(qa, 'chrome-profile');
const home = join(qa, 'home');
const runtime = join(qa, 'runtime');
const data = join(qa, 'daemon');
for (const folder of [profile, home, runtime, data]) {
  mkdirSync(folder, { recursive: true, mode: 0o700 });
}

const env: NodeJS.ProcessEnv = {
  ...process.env,
  HOME: home,
  XDG_CONFIG_HOME: join(home, '.config'),
  XDG_RUNTIME_DIR: runtime,
  WATCHER_DEV_HOME: data,
  XDG_SESSION_TYPE: 'wayland',
  XDG_CURRENT_DESKTOP: 'GNOME', // inert test-only collector, no compositor
};
for (const name of ['DISPLAY', 'WAYLAND_DISPLAY', 'HYPRLAND_INSTANCE_SIGNATURE', 'SWAYSOCK', 'DBUS_SESSION_BUS_ADDRESS']) {
  delete env[name];
}

const http = createServer((_req, res) => {
  res.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
  res.end('<html><title>Isolated browser QA</title><body>Watcher fixture</body></html>');
});
let context: BrowserContext | undefined;
let panel: Page | undefined;
let watcherd: ChildProcess | undefined;
let daemonOutput = '';
let serverPort = 0;
let extensionId = '';

function launchOptions() {
  return {
    executablePath: chromeExecutable,
    headless: true,
    env,
    args: [
      '--ozone-platform=headless',
      '--disable-background-networking',
      '--no-proxy-server',
      `--load-extension=${dist}`,
      `--disable-extensions-except=${dist}`,
      '--host-resolver-rules=MAP alpha.test 127.0.0.1, MAP beta.test 127.0.0.1, MAP gamma.test 127.0.0.1',
    ],
  };
}

async function openPanel() {
  context = await chromium.launchPersistentContext(profile, launchOptions());
  const worker = context.serviceWorkers()[0] ?? await context.waitForEvent('serviceworker', { timeout: 10_000 });
  const id = new URL(worker.url()).host;
  assert(id, 'Chromium did not load the extension background service worker');
  if (extensionId) assert.equal(id, extensionId, 'extension identity must persist across restart');
  extensionId = id;
  panel = await context.newPage();
  await panel.goto(`chrome-extension://${extensionId}/sidepanel.html`, { waitUntil: 'domcontentloaded' });
  await panel.getByText('Browser & device analytics').waitFor();
}

async function request(message: unknown): Promise<any> {
  assert(panel);
  return panel.evaluate((value) => (globalThis as any).chrome.runtime.sendMessage(value), message);
}
async function native(message: unknown): Promise<any> {
  assert(panel);
  return panel.evaluate((value) => (globalThis as any).chrome.runtime.sendNativeMessage('dev.watcher.bridge', value), message);
}
async function activate(url: string): Promise<number> {
  assert(panel);
  const tab: number = await panel.evaluate(async (destination) => {
    const created = await (globalThis as any).chrome.tabs.create({ url: destination, active: true });
    return created.id;
  }, url);
  await sleep(450);
  return tab;
}
async function eventually<T>(get: () => Promise<T>, test: (value: T) => boolean, label: string): Promise<T> {
  for (let attempt = 0; attempt < 35; attempt++) {
    const value = await get();
    if (test(value)) return value;
    await sleep(150);
  }
  throw new Error(`timed out waiting for ${label}`);
}

try {
  http.listen(0, '127.0.0.1');
  await once(http, 'listening');
  const addr = http.address();
  assert(addr && typeof addr !== 'string');
  serverPort = addr.port;
  await openPanel();
  console.log('PASS: Chromium loaded the WXT sidepanel in a temporary profile');

  // Native manifest is located in the isolated browser user-data-dir.
  // Never write registration to the actual user's HOME or system registry.
  const manifestDir = join(profile, 'NativeMessagingHosts');
  mkdirSync(manifestDir, { recursive: true, mode: 0o700 });
  const manifest = join(manifestDir, 'dev.watcher.bridge.json');
  writeFileSync(manifest, JSON.stringify({
    name: 'dev.watcher.bridge', description: 'Disposable Watcher QA host', type: 'stdio',
    path: host, allowed_origins: [`chrome-extension://${extensionId}/`],
  }));
  chmodSync(manifest, 0o600);
  const unavailable = await native({ op: 'status', version: 1 });
  assert.equal(unavailable.message, 'daemon_unavailable');
  console.log('PASS: native host is registered, but daemon reports offline');

  const alpha = `http://alpha.test:${serverPort}/`;
  const beta = `http://beta.test:${serverPort}/`;
  const gamma = `http://gamma.test:${serverPort}/`;
  await activate(alpha);
  await activate(beta);
  let observed = await eventually(
    () => request({ type: 'watcher-dashboard' }),
    (value) => value.local.pending > 0 && value.local.domains.some((d: any) => d.domain === 'alpha.test'),
    'offline alpha observations',
  );
  assert.equal(observed.online, false);
  assert(observed.local.domains.every((d: any) => ['alpha.test', 'beta.test'].includes(d.domain)),
    'unexpected website recorded in isolated browser');
  console.log(`PASS: offline alpha/beta tab transitions persisted (${observed.local.pending} pending)`);

  // Chromium's own window states are the ground truth, not time-based guesses.
  const newWindow = await panel!.evaluate((url) => (globalThis as any).chrome.windows.create({ url, focused: false }), gamma);
  const windows = await panel!.evaluate(() => (globalThis as any).chrome.windows.getAll());
  if (!newWindow.focused && windows.some((w: any) => w.focused && w.id !== newWindow.id)) {
    await sleep(900);
    observed = await request({ type: 'watcher-dashboard' });
    assert(!observed.local.domains.some((d: any) => d.domain === 'gamma.test'),
      'background/unfocused window was credited website usage');
    console.log('PASS: independently unfocused browser window received no domain time');
  } else {
    console.log('INCONCLUSIVE: headless Chromium did not create an independently unfocused window');
  }

  const pendingBeforeRestart = observed.local.pending;
  await context!.close();
  await openPanel();
  observed = await request({ type: 'watcher-dashboard' });
  assert(observed.local.pending >= pendingBeforeRestart,
    'browser restart lost unacknowledged intervals');
  assert.equal(observed.online, false);
  console.log('PASS: offline IndexedDB queue survived a complete Chromium restart');

  watcherd = spawn(daemon, ['--dev'], { env, cwd: repo, stdio: ['ignore', 'pipe', 'pipe'] });
  watcherd.stdout?.on('data', (data) => { daemonOutput += data.toString(); });
  watcherd.stderr?.on('data', (data) => { daemonOutput += data.toString(); });
  await eventually(async () => {
    if (watcherd!.exitCode !== null) throw new Error(`disposable daemon exited: ${daemonOutput}`);
    return existsSync(join(data, 'daemon.sock'));
  }, Boolean, 'daemon IPC socket');
  const online = await eventually(
    () => request({ type: 'watcher-dashboard' }),
    (value) => value.online && value.local.pending === 0,
    'native sync acknowledgement',
  );
  assert.equal(online.device?.active_ms, 0,
    'browser-only time must not become verified device active time');
  const from = Date.now() - 120_000;
  const to = Date.now() + 5_000;
  const browserReport = await native({ op: 'browser_report', version: 1, from_ms: from, to_ms: to });
  assert.equal(browserReport.kind, 'browser_report');
  assert(browserReport.domains.some((d: any) => d.domain === 'alpha.test'));
  const repeated = await request({ type: 'watcher-dashboard' });
  assert.equal(repeated.local.pending, 0);
  const reportAgain = await native({ op: 'browser_report', version: 1, from_ms: from, to_ms: to });
  assert.deepEqual(reportAgain.domains, browserReport.domains, 'repeated refresh duplicated browser history');
  console.log('PASS: Chrome Native Messaging -> daemon -> SQLite, idempotent replay, device time uninflated');
  console.log('All isolated Chrome extension acceptance checks passed.');
} finally {
  if (watcherd && watcherd.exitCode === null) {
    const shutdown = spawn(cli, ['--dev', 'shutdown'], { env, cwd: repo, stdio: 'ignore' });
    await new Promise((resolve) => shutdown.once('exit', resolve));
    await sleep(200);
    if (watcherd.exitCode === null) watcherd.kill('SIGTERM');
  }
  if (context) await context.close().catch(() => {});
  http.close();
  rmSync(qa, { recursive: true, force: true });
}
