import { chromium, expect } from "@playwright/test";
import { spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
const root = process.cwd();
const exe =
  process.env.TOKENSCOPE_TEST_EXE ??
  path.join(root, "src-tauri/target/debug/tokenscope.exe");
async function test(mode, port) {
  const isolated = mode !== "real";
  const dataRoot = path.join(
    root,
    ".local",
    mode === "recovery" || mode === "blocked"
      ? mode + "-test-only-" + Date.now()
      : mode + "-runtime",
  );
  fs.mkdirSync(dataRoot, { recursive: true });
  const env = {
    ...process.env,
    WEBVIEW2_USER_DATA_FOLDER: path.join(dataRoot, "webview-profile"),
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:
      "--remote-debugging-port=" +
      port +
      " --remote-debugging-address=127.0.0.1",
  };
  if (mode === "real") {
    // Match Explorer launch: Codex's injected CLI PATH is absent.
    env.PATH = (env.PATH ?? "")
      .split(path.delimiter)
      .filter((p) => !/OpenAI[\\/]Codex[\\/]bin/i.test(p))
      .join(path.delimiter);
    delete env.TOKENSCOPE_CODEX_BIN;
  }
  if (isolated) {
    env.TOKENSCOPE_TEST_MODE = "1";
    env.TOKENSCOPE_DATA_DIR = dataRoot;
    env.CODEX_HOME = path.join(dataRoot, "codex-missing");
    env.TOKENSCOPE_CODEX_BIN = path.join(dataRoot, "missing-codex.exe");
  }
  if (mode === "blocked")
    fs.mkdirSync(path.join(dataRoot, "tokenscope.sqlite3"));
  const child = spawn(exe, [], { env, windowsHide: true, stdio: "ignore" });
  let browser;
  let page;
  try {
    for (let i = 0; i < 30; i++) {
      try {
        browser = await chromium.connectOverCDP("http://127.0.0.1:" + port);
        break;
      } catch {
        await new Promise((r) => setTimeout(r, 1000));
      }
    }
    if (!browser) throw Error("WebView unavailable");
    page = browser.contexts().flatMap((c) => c.pages())[0];
    console.log(mode, "URL", page.url());
    if (page.url().includes("1420"))
      throw Error("Standalone build still uses dev server");
    const errors = [];
    page.on("pageerror", (e) => errors.push(e.message));
    await expect(
      page.getByRole("heading", { name: "用量总览", exact: true }),
    ).toBeVisible();
    if (mode === "blocked") {
      await expect(page.locator(".live-pill")).toHaveText("采集异常 · 将重试", {
        timeout: 15000,
      });
      const db = path.resolve(dataRoot, "tokenscope.sqlite3");
      const failedHealth = await page.evaluate(() =>
        window.__TAURI_INTERNALS__.invoke("read_health"),
      );
      expect(failedHealth.database_path).toBe(db);
      expect(failedHealth.last_error).toContain("打开数据库失败");
      const moved = path.resolve(dataRoot, "blocked-empty-directory");
      for (const p of [db, moved])
        if (!p.startsWith(path.resolve(dataRoot) + path.sep))
          throw Error("Unsafe fixture path");
      fs.renameSync(db, moved);
      await page
        .getByRole("button", { name: "刷新并采集", exact: true })
        .click();
      await expect(page.locator(".live-pill")).toHaveText("等待数据源", {
        timeout: 15000,
      });
      expect(
        await page.evaluate(
          async () =>
            (await window.__TAURI_INTERNALS__.invoke("read_health")).last_error,
        ),
      ).toBeNull();
      await expect(page.locator(".metric strong").first()).toHaveText("—");
      await expect(page.locator(".live-rate-values")).toContainText("—");
      console.log(
        "blocked DB startup stays responsive and retries after obstacle is removed: passed",
      );
    } else if (isolated) {
      await expect(
        page.getByText("未发现 Codex 会话目录。", { exact: false }),
      ).toBeVisible({ timeout: 15000 });
      await expect(page.locator(".metric strong").first()).toHaveText("—");
      await expect(page.getByText("额度不可用", { exact: true })).toBeVisible({
        timeout: 25000,
      });
      console.log("absence/CLI unavailable: honest empty state");
      if (mode === "recovery") {
        // Synthetic fixtures are isolated from the user's real Codex home and DB.
        const source = path.join(env.CODEX_HOME, "sessions");
        const away = path.join(env.CODEX_HOME, "sessions-offline");
        for (const target of [source, away]) {
          if (
            !path.resolve(target).startsWith(path.resolve(dataRoot) + path.sep)
          )
            throw Error("Fixture path escaped test workspace");
        }
        fs.mkdirSync(source, { recursive: true });
        const file = path.join(source, "TEST_ONLY.jsonl");
        const usage = (n, sec) =>
          JSON.stringify({
            type: "event_msg",
            timestamp: "2026-10-09T00:00:" + sec + "Z",
            payload: {
              type: "token_count",
              info: {
                total_token_usage: {
                  input_tokens: n,
                  cached_input_tokens: 0,
                  output_tokens: 0,
                  reasoning_output_tokens: 0,
                  total_tokens: n,
                },
                last_token_usage: {
                  input_tokens: n,
                  cached_input_tokens: 0,
                  output_tokens: 0,
                  reasoning_output_tokens: 0,
                  total_tokens: n,
                },
              },
            },
          });
        fs.writeFileSync(
          file,
          JSON.stringify({
            type: "session_meta",
            payload: { id: "TEST_ONLY_RECOVERY", cwd: "/TEST_ONLY_FIXTURE" },
          }) +
            "\n" +
            usage(110, "01") +
            "\n",
        );
        await page
          .getByRole("button", { name: "刷新并采集", exact: true })
          .click();
        await expect(page.locator(".live-pill")).toHaveText("本地采集已连接", {
          timeout: 15000,
        });
        await expect(page.locator(".metric strong").first()).toHaveAttribute(
          "title",
          "110",
        );
        fs.renameSync(source, away);
        await expect(page.locator(".live-pill")).toHaveText("等待数据源", {
          timeout: 15000,
        });
        await expect(page.locator(".metric strong").first()).toHaveAttribute(
          "title",
          "110",
        );
        fs.renameSync(away, source);
        await page
          .getByRole("button", { name: "刷新并采集", exact: true })
          .click();
        await expect(page.locator(".live-pill")).toHaveText("本地采集已连接", {
          timeout: 15000,
        });
        await expect
          .poll(() =>
            page.evaluate(() =>
              window.__TAURI_INTERNALS__
                .invoke("read_health")
                .then((h) => h.collector.recoveries),
            ),
          )
          .toBe(2);
        fs.appendFileSync(file, usage(220, "02"));
        await page.evaluate(() =>
          window.__TAURI_INTERNALS__.invoke("retry_ingest"),
        );
        await page.waitForTimeout(3500);
        await expect(page.locator(".metric strong").first()).toHaveAttribute(
          "title",
          "110",
        );
        fs.appendFileSync(file, "\n");
        await page
          .getByRole("button", { name: "刷新并采集", exact: true })
          .click();
        await expect(page.locator(".metric strong").first()).toHaveAttribute(
          "title",
          "220",
          { timeout: 15000 },
        );
        await page.evaluate(() =>
          window.__TAURI_INTERNALS__.invoke("retry_ingest"),
        );
        await page.waitForTimeout(3500);
        await expect(page.locator(".metric strong").first()).toHaveAttribute(
          "title",
          "220",
        );
        await page.getByRole("button", { name: "时间线", exact: true }).click();
        await page
          .getByRole("combobox", { name: "趋势粒度" })
          .selectOption("minute");
        await expect(
          page.getByRole("img", { name: /1 分钟间隔聚合/ }),
        ).toBeVisible();
        console.log(
          "TEST ONLY: source disappearance/reappearance, manual retry, retained history, partial-line recovery, replay: passed",
        );
      }
    } else {
      await page.evaluate(() =>
        window.__TAURI_INTERNALS__.invoke("set_fx_enabled", { enabled: true }),
      );
      await page.evaluate(() =>
        window.__TAURI_INTERNALS__.invoke("refresh_fx"),
      );
      await expect(page.locator(".session-link").first()).toBeVisible({
        timeout: 30000,
      });
      const d = await page.evaluate(() =>
        window.__TAURI_INTERNALS__.invoke("read_dashboard", { filter: null }),
      );
      await expect(page.locator(".metric strong").first()).toHaveAttribute(
        "title",
        BigInt(d.totals.total).toLocaleString("zh-CN"),
      );
      await expect(page.getByText("上次同步", { exact: false })).toBeVisible({
        timeout: 30000,
      });
      await expect(page.locator(".live-rate-values")).toContainText("tokens/s");
      await expect(page.locator(".live-rate-values")).toContainText("tokens/m");
      await expect
        .poll(
          async () => {
            const rate = await page.evaluate(() =>
              window.__TAURI_INTERNALS__.invoke("read_output_rate"),
            );
            const text = await page.locator(".live-rate-values").innerText();
            if (rate.unknown_events > 0 && rate.confirmed_events === 0)
              return text.includes("—");
            const format = (value) => {
              const [whole, fraction] = value.split(".");
              return (
                BigInt(whole).toLocaleString("zh-CN") +
                (fraction === undefined ? "" : "." + fraction)
              );
            };
            return (
              text.includes(format(rate.tokens_per_second)) &&
              text.includes(format(rate.tokens_per_minute))
            );
          },
          { timeout: 15000 },
        )
        .toBe(true);
      console.log(
        "real visible-output throughput matches exact Rust IPC/unknown: passed",
      );
      await page
        .locator(".provider-strip")
        .screenshot({ path: path.join(root, ".local/live-rate-real.png") });
      await expect(
        page.getByRole("heading", { name: "API 等价参考估算" }),
      ).toBeVisible();
      await expect(page.getByText("价格覆盖", { exact: false })).toBeVisible();
      await expect(
        page.getByText("参考汇率：已同步", { exact: false }),
      ).toBeVisible({ timeout: 25000 });
      await page
        .getByRole("combobox", { name: "估算币别" })
        .selectOption("HKD");
      await expect(page.locator(".cost-total")).toContainText("HKD");
      await page
        .getByRole("combobox", { name: "估算币别" })
        .selectOption("CNY");
      await expect(page.locator(".cost-total")).toContainText("CNY");
      await expect(
        page.getByRole("heading", { name: "活跃热力图" }),
      ).toBeVisible();
      await expect(page.locator("button.heatmap-cell")).toHaveCount(365);
      const cell = page.locator("button.heatmap-cell").last();
      await cell.focus();
      await page.keyboard.press("Enter");
      await expect(cell).toHaveAttribute("aria-pressed", "true");
      await page.keyboard.press("ArrowLeft");
      await expect(
        page.locator("button.heatmap-cell").nth(357),
      ).toHaveAttribute("aria-pressed", "true");
      if (d.candles.length !== d.timeline.length)
        throw Error("Candle alignment mismatch");
      const heatSum = d.activity.days.reduce(
        (n, day) => n + BigInt(day.total),
        0n,
      );
      if (heatSum.toString() !== d.activity.total)
        throw Error("Heatmap total mismatch");
      await expect(
        page.getByRole("img", { name: "OpenAI 供应商标识" }).first(),
      ).toBeVisible();
      console.log(
        "365 local-date cells, exact heatmap totals, keyboard selection and supplier icons: passed",
      );
      await expect(
        page.getByRole("navigation", { name: "主导航" }).getByRole("button"),
      ).toHaveCount(5);
      await expect(
        page.getByRole("button", { name: "套餐额度", exact: true }),
      ).toHaveCount(0);
      await expect(page.locator(".trend")).toHaveCount(0);
      await expect(
        page.getByRole("combobox", { name: "趋势粒度" }),
      ).toHaveCount(0);
      await expect(
        page.getByRole("heading", { name: "Codex 套餐额度" }),
      ).toHaveCount(1);
      const selectedModel = await page
        .getByRole("combobox", { name: "模型筛选" })
        .inputValue();
      await page
        .getByRole("button", { name: "查看消耗时间线", exact: false })
        .click();
      await expect(
        page.getByRole("heading", { name: "消耗时间线", exact: true }),
      ).toBeVisible();
      await expect(
        page.getByRole("combobox", { name: "模型筛选" }),
      ).toHaveValue(selectedModel);
      await page.getByRole("button", { name: "总览", exact: true }).click();
      console.log(
        "five navigation entries, unique quota in overview, unique trend in timeline, filter-preserving drilldown: passed",
      );
      await page.evaluate(() => scrollTo(0, 0));
      await expect(
        page
          .locator(".metric")
          .filter({ hasText: "输入 Token" })
          .locator(".lucide-arrow-up-right"),
      ).toBeVisible();
      await expect(
        page
          .locator(".metric")
          .filter({ hasText: "输出 Token" })
          .locator(".lucide-arrow-down-left"),
      ).toBeVisible();
      const sessionLogo = page
        .locator(".session-icon .provider-mark.openai svg")
        .first();
      const modelLogo = page
        .locator(".model-tag .provider-mark.openai svg")
        .first();
      await expect(sessionLogo).toHaveAttribute("width", "20");
      await expect(modelLogo).toHaveAttribute("width", "20");
      if (
        (await sessionLogo.evaluate((el) => getComputedStyle(el).color)) !==
        (await modelLogo.evaluate((el) => getComputedStyle(el).color))
      ) {
        throw Error("Session and model logo colors differ");
      }
      await page.screenshot({
        path: ".local/final-overview.png",
        fullPage: true,
      });
      const projectBubble = page.locator(".project-bubble").first();
      await projectBubble.evaluate((el) =>
        el.scrollIntoView({ block: "center" }),
      );
      await page.waitForTimeout(200);
      const projectBubbleBox = await projectBubble.boundingBox();
      await page.mouse.move(
        projectBubbleBox.x + projectBubbleBox.width / 2,
        projectBubbleBox.y + projectBubbleBox.height / 2,
      );
      await expect(projectBubble).toHaveClass(/is-active/);
      await expect(page.getByRole("tooltip")).toBeVisible();
      await page
        .locator(".project-bubble-panel")
        .screenshot({ path: ".local/project-bubbles.png" });
      await page.mouse.move(5, 5);
      await page.getByRole("button", { name: "切换深色主题" }).click();
      await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
      await page.waitForTimeout(250);
      await page.screenshot({ path: ".local/ui-dark.png", fullPage: true });
      await page.getByRole("button", { name: "切换浅色主题" }).click();
      const cdp = await page.context().newCDPSession(page);
      await cdp.send("Emulation.setDeviceMetricsOverride", {
        width: 860,
        height: 640,
        deviceScaleFactor: 1,
        mobile: false,
      });
      await page.waitForTimeout(300);
      if (
        await page.evaluate(
          () => document.documentElement.scrollWidth > innerWidth,
        )
      )
        throw Error("Window-level horizontal overflow at minimum width");
      await page.screenshot({ path: ".local/ui-compact.png", fullPage: true });
      await cdp.send("Emulation.clearDeviceMetricsOverride");
      await page.keyboard.press("Alt+2");
      await expect(
        page.getByRole("heading", { name: "会话与轮次", exact: true }),
      ).toBeVisible();
      await page.getByRole("button", { name: "会话", exact: true }).click();
      await page.locator(".session-link").first().click();
      await expect(page.getByRole("dialog")).toBeVisible();
      await page.keyboard.press("Escape");
      await expect(page.getByRole("dialog")).toBeHidden();
      await page
        .getByRole("combobox", { name: "模型筛选" })
        .selectOption({ index: 1 });
      const modelBeforeNavigation = await page
        .getByRole("combobox", { name: "模型筛选" })
        .inputValue();
      await page.waitForTimeout(600);
      await page.getByRole("button", { name: "时间线", exact: true }).click();
      await expect(
        page.getByRole("combobox", { name: "模型筛选" }),
      ).toHaveValue(modelBeforeNavigation);
      await expect(page.locator(".trend")).toHaveCount(1);
      await expect(
        page.getByRole("heading", { name: "活跃热力图" }),
      ).toHaveCount(0);
      await expect(
        page.getByRole("heading", { name: "Codex 套餐额度" }),
      ).toHaveCount(0);
      await expect(
        page.getByRole("heading", { name: "API 等价参考估算" }),
      ).toHaveCount(0);
      const peakData = await page.evaluate(
        (model) =>
          window.__TAURI_INTERNALS__.invoke("read_dashboard", {
            filter: { model, grain: "day" },
          }),
        modelBeforeNavigation,
      );
      for (const [button, chart] of [
        ["柱状", "柱状图"],
        ["K 线", "K 线图"],
        ["折线", "折线图"],
      ]) {
        await page.getByRole("button", { name: button, exact: true }).click();
        await expect(
          page.getByRole("img", { name: new RegExp(chart + "，") }),
        ).toBeVisible();
        await expect(
          page.getByRole("button", { name: button, exact: true }),
        ).toHaveAttribute("aria-pressed", "true");
        const values =
          chart === "K 线图"
            ? peakData.candles.map((c) => c?.high ?? "0")
            : peakData.timeline.map((g) => g.totals.total);
        const maximum = values.reduce(
          (best, value, index) =>
            BigInt(value) > BigInt(best.value) ? { value, index } : best,
          { value: "0", index: -1 },
        );
        if (maximum.index >= 0) {
          await expect(page.locator(".trend-peak")).toHaveAttribute(
            "data-peak-value",
            maximum.value,
          );
          await expect(page.locator(".trend-peak")).toHaveAttribute(
            "data-peak-index",
            String(maximum.index),
          );
          await expect(page.locator(".trend-peak")).toHaveAttribute(
            "title",
            new RegExp(BigInt(maximum.value).toLocaleString("zh-CN")),
          );
          await expect(page.locator(".trend-peak")).toContainText(
            chart === "K 线图" ? "单条增量峰值" : "桶总量峰值",
          );
        }
        await page.waitForTimeout(350);
        await page.evaluate(() => scrollTo(0, 0));
        await page.screenshot({
          path: `.local/chart-${chart}.png`,
          fullPage: true,
        });
      }
      console.log(
        "all three modes: exact peak value/index match Rust aggregates: passed",
      );
      await expect(
        page.getByRole("heading", { name: "计量事件" }),
      ).toBeVisible();
      const eventsHeight = await page
        .locator(".timeline-events .table-scroll")
        .evaluate((el) => el.getBoundingClientRect().height);
      if (eventsHeight > 601)
        throw Error("Timeline event table is not bounded");
      const timelineCdp = await page.context().newCDPSession(page);
      await timelineCdp.send("Emulation.setDeviceMetricsOverride", {
        width: 860,
        height: 640,
        deviceScaleFactor: 1,
        mobile: false,
      });
      await page.waitForTimeout(200);
      if (
        await page.evaluate(
          () => document.documentElement.scrollWidth > innerWidth,
        )
      )
        throw Error("Timeline window overflow");
      await timelineCdp.send("Emulation.clearDeviceMetricsOverride");
      for (const grain of ["minute", "hour", "day"]) {
        await page
          .getByRole("combobox", { name: "趋势粒度" })
          .selectOption(grain);
        const series = await page.evaluate(
          (grain) =>
            window.__TAURI_INTERNALS__.invoke("read_dashboard", {
              filter: { grain },
            }),
          grain,
        );
        for (const field of [
          "input",
          "cached",
          "output",
          "reasoning",
          "total",
        ]) {
          const sum = series.timeline.reduce(
            (n, g) => n + BigInt(g.totals[field]),
            0n,
          );
          if (sum.toString() !== series.totals[field])
            throw Error("Timeline mismatch " + grain + " " + field);
        }
        if (series.timeline.length > 720) throw Error("Series unbounded");
      }
      await page.getByRole("button", { name: "总览", exact: true }).click();
      await expect(page.getByText("上次同步", { exact: false })).toBeVisible();
      await page.locator(".quota-help summary").click();
      await expect(
        page.getByText("额度来自服务端，可能跨客户端与设备共享。", {
          exact: false,
        }),
      ).toBeVisible();
      await page.locator(".quota-help summary").click();
      await page.keyboard.press("Alt+4");
      await expect(
        page.getByRole("heading", { name: "消耗与额度预警", exact: true }),
      ).toBeVisible();
      await page.keyboard.press("Alt+5");
      await page
        .getByRole("button", { name: "设置与数据源", exact: true })
        .click();
      await expect(
        page.getByRole("heading", { name: "Provider 能力" }),
      ).toBeVisible();
      await page
        .getByRole("checkbox", { name: "每日自动更新参考汇率" })
        .uncheck();
      await expect(
        page.getByRole("checkbox", { name: "每日自动更新参考汇率" }),
      ).not.toBeChecked();
      await page.getByRole("button", { name: /^总览/ }).click();
      await expect(page.locator(".cost-total")).toHaveText("—");
      await page
        .getByRole("button", { name: "设置与数据源", exact: true })
        .click();
      await page
        .getByRole("checkbox", { name: "每日自动更新参考汇率" })
        .check();
      await expect(
        page.getByRole("checkbox", { name: "每日自动更新参考汇率" }),
      ).toBeChecked();
      console.log(
        "real data, exact IPC display, model filter, modal keyboard, timeline, quota, settings: passed",
      );
    }
    if (errors.length) throw Error(errors.join("\n"));
    console.log(mode, "runtime errors:", errors.length);
  } finally {
    if (mode === "real" && page)
      await page
        .evaluate(() =>
          window.__TAURI_INTERNALS__.invoke("set_fx_enabled", {
            enabled: true,
          }),
        )
        .catch(() => {});
    if (browser) await browser.close();
    child.kill();
    if (child.exitCode === null && child.signalCode === null)
      await new Promise((r) => child.once("exit", r));
  }
}
if (process.argv.includes("--real")) await test("real", 9223);
await test("empty", 9224);

await test("recovery", 9225);

await test("blocked", 9226);
