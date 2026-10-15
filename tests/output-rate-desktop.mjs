import { chromium, expect } from "@playwright/test";
import { spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

// All generated logs, database and screenshots remain in a new isolated directory.
const root = path.resolve(".local", `OUTPUT_RATE_TEST_ONLY_${Date.now()}`);
const source = path.join(root, "codex", "sessions");
fs.mkdirSync(source, { recursive: true });
const json = (v) => JSON.stringify(v) + "\n";
const counts = (output, reasoning) => ({
  input_tokens: 10000,
  cached_input_tokens: 8000,
  output_tokens: output,
  reasoning_output_tokens: reasoning,
  total_tokens: 10000 + output,
});
const usage = (at, output, reasoning) =>
  json({
    type: "event_msg",
    timestamp: at,
    payload: {
      type: "token_count",
      info: {
        total_token_usage: counts(output, reasoning),
        last_token_usage: counts(output, reasoning),
      },
    },
  });
const meta = (id, project) =>
  json({
    type: "session_meta",
    payload: { id, cwd: `/TEST_ONLY/${project}` },
  }) +
  json({
    type: "turn_context",
    payload: { turn_id: id + "-turn", model: "test-model" },
  });
for (const [i, name] of [
  "Aurora",
  "Ocean",
  "Atlas",
  "Orchid",
  "Solstice",
  "Rose",
  "Lagoon",
  "Meadow",
].entries()) {
  fs.writeFileSync(
    path.join(source, `${i}.jsonl`),
    meta(`visual-${i}`, name) +
      usage(new Date(Date.now() - 86400000).toISOString(), 20000 * (8 - i), 0),
  );
}
const file = path.join(source, "live.jsonl");
fs.writeFileSync(
  file,
  meta("live", "Live workspace") +
    json({
      type: "event_msg",
      payload: { type: "task_started", turn_id: "live-turn" },
    }) +
    json({ type: "response_item", payload: { type: "function_call" } }),
);
const port = 9256;
const child = spawn(
  process.env.TOKENSCOPE_TEST_EXE ??
    path.resolve("src-tauri/target/release/tokenscope.exe"),
  [],
  {
    windowsHide: true,
    stdio: "ignore",
    env: {
      ...process.env,
      TOKENSCOPE_TEST_MODE: "1",
      TOKENSCOPE_DATA_DIR: root,
      CODEX_HOME: path.dirname(source),
      TOKENSCOPE_CODEX_BIN: path.join(root, "absent.exe"),
      WEBVIEW2_USER_DATA_FOLDER: path.join(root, "webview"),
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port} --remote-debugging-address=127.0.0.1`,
    },
  },
);
let browser;
try {
  for (let i = 0; i < 30; i++) {
    try {
      browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
      break;
    } catch {
      await new Promise((r) => setTimeout(r, 1000));
    }
  }
  if (!browser) throw Error("WebView not available");
  const page = browser.contexts().flatMap((c) => c.pages())[0];
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  const values = page.locator(".live-rate-values strong");
  await expect(page.locator(".live-pill")).toHaveText("本地采集已连接", {
    timeout: 20000,
  });
  await expect(values.first()).toHaveText("0.00", { timeout: 15000 });
  await expect(page.locator(".live-rate")).toContainText("非模型解码速度");
  const base = Date.now() - 44000;
  fs.appendFileSync(file, usage(new Date(base).toISOString(), 200, 80));
  await expect(values.first()).toHaveText("2.00", { timeout: 12000 });
  await expect(values.last()).toHaveText("120");
  const second = usage(new Date(base + 1000).toISOString(), 600, 240);
  fs.appendFileSync(file, second.trimEnd());
  await page.waitForTimeout(3500);
  await expect(values.first()).toHaveText("2.00");
  fs.appendFileSync(file, "\n");
  await expect(values.first()).toHaveText("6.00", { timeout: 12000 });
  await expect(values.last()).toHaveText("360");
  const strict = await page.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke("read_generation_speed"),
  );
  expect(strict.tokens_per_second).toBeNull();
  await page
    .locator(".provider-strip")
    .screenshot({ path: path.join(root, "output-rate.png") });
  // A quiet source must age out without any manual refresh or new notifications.
  await expect(values.first()).toHaveText("0.00", { timeout: 30000 });
  await expect(values.last()).toHaveText("0");
  console.log(
    "automatic append, partial line, multi/tool turn, exact output subsets and expiry: passed",
  );
  const panel = page.locator(".project-bubble-panel");
  await panel.scrollIntoViewIfNeeded();
  await page.mouse.move(3, 3);
  const bubbles = page.locator(".project-bubble");
  await expect(bubbles).toHaveCount(9);
  const styles = await bubbles.evaluateAll((els) =>
    els.map((el) => ({
      color: el.style.getPropertyValue("--bubble-color"),
      motion: getComputedStyle(el, "::before").animationName,
    })),
  );
  expect(new Set(styles.map((s) => s.color)).size).toBe(8);
  expect(styles.every((s) => s.motion === "bubble-breathe")).toBe(true);
  await panel.screenshot({ path: path.join(root, "bubbles-light.png") });
  const first = bubbles.first();
  const box = await first.boundingBox();
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await expect(first).toHaveClass(/is-active/);
  await expect(page.getByRole("tooltip")).toBeVisible();
  expect(
    await first.evaluate(
      (el) => getComputedStyle(el, "::before").animationPlayState,
    ),
  ).toBe("paused");
  await first.focus();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("tooltip")).toHaveCount(0);
  await page.mouse.move(3, 3);
  await page.getByRole("button", { name: "切换深色主题" }).click();
  await panel.scrollIntoViewIfNeeded();
  await panel.screenshot({ path: path.join(root, "bubbles-dark.png") });
  await page.emulateMedia({ reducedMotion: "reduce" });
  expect(
    await first.evaluate(
      (el) => getComputedStyle(el, "::before").animationName,
    ),
  ).toBe("none");
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("Emulation.setDeviceMetricsOverride", {
    width: 860,
    height: 640,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await panel.scrollIntoViewIfNeeded();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth > innerWidth,
    ),
  ).toBe(false);
  await panel.screenshot({ path: path.join(root, "bubbles-compact.png") });
  expect(errors).toEqual([]);
  fs.writeFileSync(
    path.join(root, "report.json"),
    JSON.stringify(
      {
        passed: true,
        frontend_errors: errors,
        scenarios: [
          "live output append",
          "partial line",
          "expiry",
          "bubble light/dark",
          "hover/keyboard",
          "reduced motion",
          "860px",
        ],
      },
      null,
      2,
    ),
  );
  console.log(
    "bubble motion, distinct palette, hover/keyboard, themes and compact layout: passed",
    root,
  );
} finally {
  await browser?.close();
  child.kill();
}
