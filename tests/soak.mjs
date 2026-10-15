// TEST ONLY: isolated 30-minute native collector endurance and restart run.
import { chromium, expect } from "@playwright/test";
import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
const root = process.cwd();
const minutes = Number(process.argv[2] ?? 30);
const port = Number(process.argv[3] ?? 9230);
if (!Number.isFinite(minutes) || minutes < 1 || minutes > 1440)
  throw Error("invalid duration");
const dir = path.join(root, ".local", "SOAK_TEST_ONLY_" + Date.now());
fs.mkdirSync(dir, { recursive: true });
const exe = path.join(dir, "TokenScope.exe");
fs.copyFileSync(
  process.env.TOKENSCOPE_TEST_EXE ??
    path.join(root, "artifacts/windows/TokenScope.exe"),
  exe,
);
const db = path.join(dir, "tokenscope.sqlite3");
const initialize = spawnSync(
  "python",
  [
    "-c",
    "import sqlite3,pathlib,sys; c=sqlite3.connect(sys.argv[1]); [c.executescript(p.read_text(encoding='utf8')) for p in sorted(pathlib.Path('src-tauri/migrations').glob('*.sql'))]; c.execute(\"UPDATE app_settings SET value='false' WHERE key='fx_enabled'\"); c.commit()",
    db,
  ],
  { encoding: "utf8" },
);
if (initialize.status !== 0) throw Error(initialize.stderr);
const home = path.join(dir, "codex");
const source = path.join(home, "sessions"),
  offline = path.join(home, "sessions-offline");
for (const p of [source, offline])
  if (!path.resolve(p).startsWith(path.resolve(dir) + path.sep))
    throw Error("unsafe fixture move");
fs.mkdirSync(source, { recursive: true });
const file = path.join(source, "TEST_ONLY.jsonl");
fs.writeFileSync(
  file,
  JSON.stringify({
    type: "session_meta",
    payload: { id: "SOAK_TEST_ONLY", cwd: "/SOAK_TEST_ONLY" },
  }) +
    "\n" +
    JSON.stringify({
      type: "turn_context",
      payload: { model: "gpt-6.1-sol", turn_id: "SOAK_TEST_ONLY_TURN" },
    }) +
    "\n",
);
let child,
  browser,
  page,
  cdp,
  n = 0,
  restarts = 0,
  disconnects = 0;
const errors = [],
  memory = [],
  heap = [];
async function stop() {
  if (browser) {
    await browser.close();
    browser = undefined;
  }
  if (child) {
    child.kill();
    if (child.exitCode === null && child.signalCode === null)
      await new Promise((r) => child.once("exit", r));
    child = undefined;
  }
}
async function launch() {
  child = spawn(exe, [], {
    env: {
      ...process.env,
      WEBVIEW2_USER_DATA_FOLDER: path.join(dir, "webview-profile"),
      TOKENSCOPE_TEST_MODE: "1",
      CODEX_HOME: home,
      TOKENSCOPE_DATA_DIR: dir,
      TOKENSCOPE_CODEX_BIN: path.join(dir, "missing.exe"),
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:
        "--remote-debugging-port=" +
        port +
        " --remote-debugging-address=127.0.0.1",
    },
    windowsHide: true,
    stdio: "ignore",
  });
  for (let i = 0; i < 30; i++) {
    try {
      browser = await chromium.connectOverCDP("http://127.0.0.1:" + port);
      break;
    } catch {
      await new Promise((r) => setTimeout(r, 1000));
    }
  }
  if (!browser) throw Error("soak webview unavailable");
  page = browser.contexts().flatMap((c) => c.pages())[0];
  page.on("pageerror", (e) => errors.push(e.message));
  cdp = await page.context().newCDPSession(page);
  await cdp.send("Performance.enable");
  await expect(page.locator(".live-pill")).toHaveText("本地采集已连接", {
    timeout: 15000,
  });
}
const invoke = (method, args) =>
  page.evaluate(
    ([method, args]) => window.__TAURI_INTERNALS__.invoke(method, args),
    [method, args],
  );
const event = (n) =>
  JSON.stringify({
    type: "event_msg",
    timestamp: new Date(Date.UTC(2026, 9, 9) + n * 1000).toISOString(),
    payload: {
      type: "token_count",
      info: {
        total_token_usage: {
          input_tokens: n * 100,
          cached_input_tokens: n * 50,
          output_tokens: n * 10,
          reasoning_output_tokens: n * 2,
          total_tokens: n * 110,
        },
        last_token_usage: {
          input_tokens: 100,
          cached_input_tokens: 50,
          output_tokens: 10,
          reasoning_output_tokens: 2,
          total_tokens: 110,
        },
      },
    },
  });
const started = Date.now();
let lastReport = started;
try {
  await launch();
  while (Date.now() - started < minutes * 60000) {
    n++;
    if (n === 1) {
      const v = JSON.parse(event(n));
      v.payload.info.last_token_usage = v.payload.info.total_token_usage;
      fs.appendFileSync(file, JSON.stringify(v) + "\n");
    } else if (n % 10 === 0) {
      fs.appendFileSync(file, event(n));
      await invoke("retry_ingest");
      await new Promise((r) => setTimeout(r, 3300));
      const d = await invoke("read_dashboard", { filter: null });
      if (d.totals.total !== String((n - 1) * 110))
        throw Error("partial line was counted");
      fs.appendFileSync(file, "\n");
    } else fs.appendFileSync(file, event(n) + "\n");
    await invoke("retry_ingest");
    await expect
      .poll(
        async () => {
          const d = await invoke("read_dashboard", { filter: null });
          return d.totals.total;
        },
        { timeout: 15000 },
      )
      .toBe(String(n * 110));
    const d = await invoke("read_dashboard", { filter: { grain: "minute" } });
    if (
      d.uncertain !== 0 ||
      d.snapshots !== n ||
      d.totals.cached !== String(n * 50) ||
      d.totals.reasoning !== String(n * 2)
    )
      throw Error("soak counters mismatch");
    if (
      d.timeline.reduce((sum, g) => sum + BigInt(g.totals.total), 0n) !==
      BigInt(n * 110)
    )
      throw Error("soak trend mismatch");
    if (n % 50 === 0) {
      fs.renameSync(source, offline);
      await invoke("retry_ingest");
      await expect(page.locator(".live-pill")).toHaveText("等待数据源", {
        timeout: 15000,
      });
      const kept = await invoke("read_dashboard", { filter: null });
      if (kept.totals.total !== String(n * 110))
        throw Error("history lost on source disconnect");
      fs.renameSync(offline, source);
      await invoke("retry_ingest");
      await expect(page.locator(".live-pill")).toHaveText("本地采集已连接", {
        timeout: 15000,
      });
      disconnects++;
    }
    if (n % 100 === 0) {
      await stop();
      await launch();
      restarts++;
      const restored = await invoke("read_dashboard", { filter: null });
      if (restored.totals.total !== String(n * 110))
        throw Error("restart duplicated or lost records");
    }
    if (n % 10 === 0) {
      const sample = spawnSync(
        "powershell",
        [
          "-NoProfile",
          "-Command",
          "(Get-Process -Id " + child.pid + ").WorkingSet64",
        ],
        { encoding: "utf8", windowsHide: true },
      );
      const rss = Number(sample.stdout.trim());
      if (Number.isFinite(rss)) memory.push(rss);
      const metrics = await cdp.send("Performance.getMetrics");
      heap.push(
        metrics.metrics.find((m) => m.name === "JSHeapUsedSize")?.value ?? 0,
      );
    }
    if (errors.length) throw Error(errors.join("\n"));
    if (Date.now() - lastReport >= 60000) {
      console.log(
        JSON.stringify({
          elapsed_minutes: ((Date.now() - started) / 60000).toFixed(1),
          events: n,
          restarts,
          disconnects,
          last_rust_rss: memory.at(-1),
          last_js_heap: heap.at(-1),
        }),
      );
      lastReport = Date.now();
    }
    await new Promise((r) => setTimeout(r, 3000));
  }
  const report = {
    test_only: true,
    exe_sha256: crypto
      .createHash("sha256")
      .update(fs.readFileSync(exe))
      .digest("hex"),
    elapsed_seconds: (Date.now() - started) / 1000,
    events: n,
    restarts,
    disconnects,
    frontend_errors: errors.length,
    rust_working_set_samples: memory,
    renderer_heap_samples: heap,
    exact_counters: true,
  };
  fs.writeFileSync(
    path.join(dir, "report.json"),
    JSON.stringify(report, null, 2),
  );
  console.log("SOAK PASSED", JSON.stringify(report));
} finally {
  await stop();
}
