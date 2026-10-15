// TEST ONLY: current-window synthetic usage and persistent local alerts.
import { chromium, expect } from "@playwright/test";
import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
const dir = path.resolve(".local/ALERTS_TEST_ONLY_" + Date.now());
fs.mkdirSync(path.join(dir, "codex/sessions"), { recursive: true });
const db = path.join(dir, "tokenscope.sqlite3");
const init = spawnSync(
  "python",
  [
    "-c",
    "import sqlite3,pathlib,sys; c=sqlite3.connect(sys.argv[1]); [c.executescript(p.read_text(encoding='utf8')) for p in sorted(pathlib.Path('src-tauri/migrations').glob('*.sql'))]; c.execute(\"UPDATE app_settings SET value='false' WHERE key='fx_enabled'\"); c.commit()",
    db,
  ],
  { encoding: "utf8" },
);
if (init.status !== 0) throw Error(init.stderr);
const file = path.join(dir, "codex/sessions/TEST_ONLY.jsonl");
const event =
  JSON.stringify({
    type: "event_msg",
    timestamp: new Date().toISOString(),
    payload: {
      type: "token_count",
      info: {
        total_token_usage: {
          input_tokens: 1999990,
          cached_input_tokens: 1000000,
          output_tokens: 10,
          reasoning_output_tokens: 5,
          total_tokens: 2000000,
        },
        last_token_usage: {
          input_tokens: 1999990,
          cached_input_tokens: 1000000,
          output_tokens: 10,
          reasoning_output_tokens: 5,
          total_tokens: 2000000,
        },
      },
    },
  }) + "\n";
fs.writeFileSync(
  file,
  JSON.stringify({
    type: "session_meta",
    payload: { id: "ALERTS_TEST_ONLY", cwd: "/TEST_ONLY" },
  }) +
    "\n" +
    event,
);
// TEST ONLY: one independently identified response, 100 visible output tokens / 2.5 s.
const generationFile = path.join(
  dir,
  "codex/sessions/GENERATION_TEST_ONLY.jsonl",
);
function generationFixture(turn, response, total, tools = false) {
  const usage = {
    input_tokens: 100,
    cached_input_tokens: 50,
    output_tokens: 120,
    reasoning_output_tokens: 20,
    total_tokens: 220,
  };
  const cumulative = Object.fromEntries(
    Object.entries(usage).map(([k, v]) => [k, v * total]),
  );
  const at = new Date().toISOString();
  return [
    { type: "event_msg", payload: { type: "task_started", turn_id: turn } },
    {
      type: "turn_context",
      payload: { turn_id: turn, model: "TEST_ONLY_MODEL" },
    },
    ...(tools
      ? [
          {
            type: "response_item",
            payload: { type: "function_call", arguments: "TEST_ONLY" },
          },
        ]
      : []),
    {
      type: "token_usage_record",
      payload: {
        thread_id: "GENERATION_TEST_ONLY",
        response_id: response,
        turn_id: turn,
        usage,
        thread_token_usage: cumulative,
      },
    },
    {
      type: "event_msg",
      timestamp: at,
      payload: {
        type: "token_count",
        info: { total_token_usage: cumulative, last_token_usage: usage },
      },
    },
    {
      type: "event_msg",
      timestamp: at,
      payload: {
        type: "task_complete",
        turn_id: turn,
        duration_ms: 3000,
        time_to_first_token_ms: 500,
      },
    },
  ]
    .map((e) => JSON.stringify(e) + "\n")
    .join("");
}
fs.writeFileSync(
  generationFile,
  JSON.stringify({
    type: "session_meta",
    payload: { id: "GENERATION_TEST_ONLY", cwd: "/TEST_ONLY" },
  }) +
    "\n" +
    generationFixture("t1", "r1", 1),
);
let child, browser, page;
let launchIndex = 0;
const errors = [];
const exe =
  process.env.TOKENSCOPE_TEST_EXE ??
  path.resolve("src-tauri/target/debug/tokenscope.exe");
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
  const port = 9270 + launchIndex++;
  child = spawn(exe, [], {
    windowsHide: true,
    stdio: "ignore",
    env: {
      ...process.env,
      TOKENSCOPE_TEST_MODE: "1",
      TOKENSCOPE_DATA_DIR: dir,
      CODEX_HOME: path.join(dir, "codex"),
      TOKENSCOPE_CODEX_BIN: path.join(dir, "missing.exe"),
      WEBVIEW2_USER_DATA_FOLDER: path.join(
        dir,
        `webview-profile-${launchIndex}`,
      ),
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port} --remote-debugging-address=127.0.0.1`,
    },
  });
  for (let n = 0; n < 30; n++) {
    try {
      browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
      break;
    } catch {
      await new Promise((r) => setTimeout(r, 1000));
    }
  }
  if (!browser) throw Error("WebView unavailable");
  await expect
    .poll(() => browser.contexts().flatMap((c) => c.pages()).length, {
      timeout: 15000,
    })
    .toBeGreaterThan(0);
  page = browser.contexts().flatMap((c) => c.pages())[0];
  page.on("pageerror", (e) => errors.push(e.message));
  await expect(
    page.getByRole("heading", { name: "用量总览", exact: true }),
  ).toBeVisible();
}
const read = () =>
  page.evaluate(() => window.__TAURI_INTERNALS__.invoke("read_alerts"));
try {
  await launch();
  await expect(page.locator(".live-rate-values")).toContainText("1.75", {
    timeout: 15000,
  });
  await expect(page.locator(".live-rate-values")).toContainText("105 tokens/m");
  const generation = await page.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke("read_generation_speed", {
      filter: null,
    }),
  );
  expect(generation.tokens_per_second).toBe("40.00");
  expect(generation.tokens_per_minute).toBe("2400.00");
  const dashboard = await page.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke("read_dashboard", { filter: null }),
  );
  const modelTotal = dashboard.models.reduce(
    (sum, row) => sum + BigInt(row.totals.total),
    0n,
  );
  await expect(page.locator(".distribution-donut strong")).toHaveAttribute(
    "title",
    modelTotal.toLocaleString("zh-CN"),
  );
  await expect(
    page
      .getByRole("region", { name: "项目消耗", exact: true })
      .locator(".project-bubble"),
  ).toHaveCount(dashboard.projects.length);
  const bubble = page.locator(".project-bubble").first();
  await bubble.evaluate((el) => el.scrollIntoView({ block: "center" }));
  await page.waitForTimeout(200);
  const bubbleBox = await bubble.boundingBox();
  await page.mouse.move(
    bubbleBox.x + bubbleBox.width / 2,
    bubbleBox.y + bubbleBox.height / 2,
  );
  await expect(bubble).toHaveClass(/is-active/);
  await expect(page.getByRole("tooltip")).toBeVisible();
  if (
    !(await bubble.evaluate((el) => {
      const r = el.getBoundingClientRect();
      return el.contains(
        document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2),
      );
    }))
  )
    throw Error("Hovered project bubble is obscured");
  await page.keyboard.press("Escape");
  await bubble.focus();
  await expect(page.getByRole("tooltip")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("tooltip")).toHaveCount(0);
  await expect(page.locator(".live-rate")).toContainText(
    "可见输出吞吐率 · 近 60 秒",
  );
  await expect(page.locator(".live-rate")).toHaveAttribute(
    "title",
    /包含工具等待和空闲时间，不是模型解码速度/,
  );
  const consumption = await page.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke("read_usage_rate", {
      filter: { session: "codex:unknown:ALERTS_TEST_ONLY" },
    }),
  );
  if (consumption.tokens_per_minute !== "2000000")
    throw Error("Consumption regression");
  await page
    .locator(".provider-strip")
    .screenshot({ path: path.join(dir, "rate-test-only.png") });
  fs.appendFileSync(generationFile, generationFixture("t2", "r2", 2, true));
  await page.getByRole("button", { name: "刷新并采集", exact: true }).click();
  await expect(page.locator(".live-rate-values")).toContainText("3.42", {
    timeout: 15000,
  });
  await expect(page.locator(".live-rate-values")).toContainText("205 tokens/m");
  const complex = await page.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke("read_generation_speed", {
      filter: null,
    }),
  );
  expect(complex.status).toBe("complex_turn");
  expect(complex.tokens_per_second).toBeNull();
  await page.getByRole("button", { name: "预警", exact: true }).click();
  await expect(
    page.getByText("五分钟会话用量偏高", { exact: true }),
  ).toBeVisible({ timeout: 15000 });
  const first = await read();
  if (first.items.length !== 1 || first.unread !== 1)
    throw Error("Alert creation mismatch");
  await page.getByRole("button", { name: "标为已读", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "已读", exact: true }),
  ).toBeDisabled();
  await page.screenshot({
    path: ".local/alerts-test-only.png",
    fullPage: true,
  });
  await stop();
  await launch();
  await page.getByRole("button", { name: "预警", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "已读", exact: true }),
  ).toBeDisabled();
  fs.appendFileSync(file, event);
  await page.getByRole("button", { name: "刷新并采集", exact: true }).click();
  await page.waitForTimeout(800);
  let after = await read();
  if (
    after.items.length !== 1 ||
    after.unread !== 0 ||
    after.items[0].id !== first.items[0].id
  )
    throw Error("Restart/replay changed alert");
  await page.getByRole("button", { name: "设置与数据源", exact: true }).click();
  await page.getByRole("textbox", { name: "五分钟 Token 阈值" }).fill("0");
  await page.getByRole("button", { name: "保存监控规则", exact: true }).click();
  await expect(page.getByText("保存失败", { exact: false })).toBeVisible();
  if ((await read()).usage_threshold !== "1000000")
    throw Error("Invalid rule mutated settings");
  const thresholdInput = page.getByRole("textbox", {
    name: "五分钟 Token 阈值",
  });
  await thresholdInput.fill(" 1,250,000 ");
  await page.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke("set_alert_rules", {
      enabled: true,
      threshold: "1000000",
    }),
  );
  await page.waitForTimeout(400);
  await expect(thresholdInput).toHaveValue(" 1,250,000 ");
  await page.getByRole("button", { name: "保存监控规则", exact: true }).click();
  await expect(
    page.getByRole("status").filter({ hasText: "已保存并生效" }),
  ).toContainText("1,250,000");
  await expect
    .poll(async () => {
      const rule = await read();
      return [rule.enabled, rule.usage_threshold];
    })
    .toEqual([true, "1250000"]);
  await page.getByRole("button", { name: "总览", exact: true }).click();
  await page.getByRole("button", { name: "设置与数据源", exact: true }).click();
  await expect(thresholdInput).toHaveValue("1250000");
  await page
    .getByRole("textbox", { name: "五分钟 Token 阈值" })
    .fill("3000000");
  await page.getByRole("checkbox", { name: "启用本地监控规则" }).uncheck();
  await page.getByRole("button", { name: "保存监控规则", exact: true }).click();
  await expect(
    page.getByRole("checkbox", { name: "启用本地监控规则" }),
  ).not.toBeChecked();
  await expect
    .poll(async () => {
      const saved = await read();
      return [saved.enabled, saved.usage_threshold];
    })
    .toEqual([false, "3000000"]);
  await stop();
  await launch();
  after = await read();
  if (
    after.enabled ||
    after.usage_threshold !== "3000000" ||
    after.items.length !== 1 ||
    after.unread !== 0
  )
    throw Error("Rules/history lost on restart");
  if (errors.length) throw Error(errors.join("\n"));
  console.log(
    JSON.stringify({
      test_only: true,
      creation: true,
      generation_estimate_exact_and_complex_unknown: true,
      visible_output_throughput_includes_tool_turn: true,
      replay: true,
      ack_restart: true,
      rule_validation: true,
      rule_restart: true,
      frontend_errors: errors.length,
    }),
  );
} finally {
  await stop();
}
