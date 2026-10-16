import { BoxRenderable, TextRenderable, createCliRenderer } from "@opentui/core";
import { createConnection } from "node:net";

type Status = {
  kind: "status";
  snapshot: {
    app: string | null;
    input: string;
    state: string;
    focus_available: boolean;
    input_available: boolean;
    paused: boolean;
  };
  collector: string;
};
type Report = {
  kind: "report";
  report: {
    active_ms: number;
    idle_ms: number;
    unknown_ms: number;
    paused_ms: number;
    apps: Array<{ app: string; duration_ms: number }>;
  };
};
type Timeline = {
  kind: "timeline";
  intervals: Array<{ start_ms: number; end_ms: number; app: string | null; state: string }>;
};
type Reply = Status | Report | Timeline | { kind: "ok"; message: string } | { kind: "error"; message: string };
type Page = "today" | "week" | "timeline" | "status";

const socketPath = process.env.WATCHER_SOCKET;
function send(request: object): Promise<Reply> {
  if (!socketPath) return Promise.reject(new Error("WATCHER_SOCKET is missing. Launch via watcher tui."));
  return new Promise((resolve, reject) => {
    const client = createConnection(socketPath);
    let received = "";
    const timeout = setTimeout(() => client.destroy(new Error("daemon response timed out")), 4000);
    const finish = () => { clearTimeout(timeout); client.destroy(); };
    client.on("connect", () => client.write(JSON.stringify({ ...request, version: 1 }) + "\n"));
    client.on("data", chunk => {
      received += chunk.toString("utf8");
      const newline = received.indexOf("\n");
      if (newline < 0) {
        if (received.length > 1024 * 1024) client.destroy(new Error("oversized daemon response"));
        return;
      }
      try {
        resolve(JSON.parse(received.slice(0, newline)) as Reply);
      } catch (error) {
        reject(error);
      }
      finish();
    });
    client.on("error", error => { finish(); reject(error); });
    client.on("end", () => { if (!received.includes("\n")) reject(new Error("daemon closed connection")); });
  });
}

const duration = (ms: number) => {
  const seconds = Math.max(0, Math.floor(ms / 1000));
  return String(Math.floor(seconds / 3600)) + "h " + String(Math.floor(seconds / 60) % 60).padStart(2, "0") +
    "m " + String(seconds % 60).padStart(2, "0") + "s";
};
function bounds(page: Page) {
  const now = new Date();
  const from = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  if (page === "week") {
    from.setDate(from.getDate() - ((from.getDay() + 6) % 7));
  }
  return { from_ms: from.getTime(), to_ms: Date.now() };
}
function renderReply(reply: Reply, page: Page): string {
  if (reply.kind === "error") throw new Error(reply.message);
  if (reply.kind === "status") {
    const s = reply.snapshot;
    return [
      "Collector       " + reply.collector,
      "Activity        " + s.state,
      "Foreground      " + (s.app ?? "none"),
      "Focus signal    " + (s.focus_available ? "available" : "unavailable"),
      "Input idle      " + (s.input_available ? "available" : "unavailable"),
      "Tracking        " + (s.paused ? "PAUSED" : "enabled"),
      "",
      s.input_available && s.focus_available
        ? "Both signals connected."
        : "Coverage incomplete. Unknown time is not active time.",
    ].join("\n");
  }
  if (reply.kind === "timeline") {
    return [
      "TIME       STATE       APPLICATION         DURATION",
      "────────────────────────────────────────────────────────",
      ...reply.intervals.slice(-18).map(interval => {
        const stamp = new Date(interval.start_ms).toLocaleTimeString();
        return stamp.padEnd(11) + interval.state.padEnd(12) +
          (interval.app ?? "—").slice(0, 18).padEnd(20) + duration(interval.end_ms - interval.start_ms);
      }),
      ...(reply.intervals.length === 0 ? ["No intervals recorded yet."] : []),
    ].join("\n");
  }
  if (reply.kind === "report") {
    const r = reply.report;
    return [
      (page === "week" ? "THIS WEEK" : "TODAY") + "  ·  ACTIVE INTERACTION",
      "",
      "  " + duration(r.active_ms),
      "",
      "Idle           " + duration(r.idle_ms),
      "Unavailable    " + duration(r.unknown_ms),
      "Paused         " + duration(r.paused_ms),
      "",
      "APPLICATIONS",
      "────────────────────────────────────────────",
      ...r.apps.slice(0, 15).map(app => "  " + app.app.slice(0, 25).padEnd(27) + duration(app.duration_ms)),
      ...(r.apps.length ? [] : ["  No activity recorded yet."]),
    ].join("\n");
  }
  return reply.message;
}

const renderer = await createCliRenderer({ exitOnCtrlC: true, targetFps: 20 });
const root = new BoxRenderable(renderer, { flexDirection: "column", padding: 2, gap: 1 });
const title = new TextRenderable(renderer, { content: "WATCHER / TODAY", fg: "#79B8FF" });
const body = new TextRenderable(renderer, { content: "Connecting to watcherd…" });
const footer = new TextRenderable(renderer, {
  content: "1 Today   2 Week   3 Timeline   4 Status   r Refresh   q Quit",
  fg: "#A8B7C7",
});
root.add(title);
root.add(body);
root.add(footer);
renderer.root.add(root);

let page: Page = "today";
let generation = 0;
let closed = false;
async function refresh() {
  const id = ++generation;
  try {
    const request = page === "status" ? { op: "status" }
      : page === "timeline" ? { op: "timeline", ...bounds(page), limit: 150 }
      : { op: "report", ...bounds(page) };
    const reply = await send(request);
    if (closed || id !== generation) return;
    title.content = "WATCHER / " + page.toUpperCase();
    body.content = renderReply(reply, page);
  } catch (error) {
    if (!closed && id === generation) body.content =
      "Watcher unavailable\n\n" + String(error) + "\n\nStart watcherd, then press r to retry.";
  }
}
const timer = setInterval(() => void refresh(), 2000);
renderer.keyInput.on("keypress", key => {
  if (key.name === "q") {
    closed = true;
    clearInterval(timer);
    renderer.destroy();
    return;
  }
  const pages: Record<string, Page> = { "1": "today", "2": "week", "3": "timeline", "4": "status" };
  if (key.name in pages) page = pages[key.name];
  if (key.name === "r" || key.name in pages) void refresh();
});
await refresh();
