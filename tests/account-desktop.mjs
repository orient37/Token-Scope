// TEST ONLY: synthetic stdio server exercises Rust projection and the native GUI.
import { chromium, expect } from "@playwright/test";
import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
const dir = path.resolve(".local/ACCOUNT_TEST_ONLY_" + Date.now());
fs.mkdirSync(dir, { recursive: true });
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
      TOKENSCOPE_DATA_DIR: dir,
      CODEX_HOME: path.join(dir, "missing"),
      TOKENSCOPE_CODEX_BIN: path.join(dir, "account-fixture.exe"),
      TOKENSCOPE_ACCOUNT_FIXTURE_FILE: path.join(dir, "profile.json"),
      WEBVIEW2_USER_DATA_FOLDER: path.join(dir, "webview"),
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:
        "--remote-debugging-port=9261 --remote-debugging-address=127.0.0.1",
    },
  },
);
let browser;
try {
  for (let n = 0; n < 40; n++) {
    try {
      browser = await chromium.connectOverCDP("http://127.0.0.1:9261");
      break;
    } catch {
      await new Promise((r) => setTimeout(r, 500));
    }
  }
  if (!browser) throw Error("WebView unavailable");
  await expect
    .poll(() => browser.contexts().flatMap((c) => c.pages()).length)
    .toBeGreaterThan(0);
  const page = browser.contexts().flatMap((c) => c.pages())[0];
  await expect(page.locator(".workspace .provider-mark.openai")).toBeVisible();
  await expect(page.locator("svg.lucide-terminal")).toHaveCount(0);
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  const avatar = page.getByRole("button", { name: "查看 Codex 账号资料" });
  await avatar.hover();
  const panel = page.getByRole("dialog", { name: "Codex 账号资料" });
  await expect(panel).toBeVisible();
  await expect(panel).toContainText("账号资料不可用", { timeout: 25000 });
  await expect(panel).toContainText("未找到 Codex");
  await page.mouse.move(400, 400);
  await expect(panel).not.toBeVisible();
  await avatar.focus();
  await expect(panel).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(panel).not.toBeVisible();
  await avatar.click();
  await expect(panel).toBeVisible();
  const compilation = spawnSync(
    "rustc",
    [
      "tests/fixtures/account-server.rs",
      "-o",
      path.join(dir, "account-fixture.exe"),
    ],
    { encoding: "utf8", windowsHide: true },
  );
  if (compilation.status !== 0) throw Error(compilation.stderr);
  const refresh = async (profile) => {
    const account =
      profile.status === "signed_out"
        ? null
        : {
            type: profile.auth_type,
            email: profile.email,
            planType: profile.plan,
          };
    fs.writeFileSync(
      path.join(dir, "profile.json"),
      JSON.stringify({ id: 2, result: { account, requiresOpenaiAuth: true } }),
    );
    await panel.getByRole("button", { name: "刷新账号" }).click();
    await expect(panel.getByRole("button", { name: "刷新账号" })).toBeEnabled();
  };
  const fixture = {
    status: "synced",
    auth_type: "chatgpt",
    email: "person@example.test",
    plan: "plus",
    message: null,
  };
  await refresh(fixture);
  await expect(panel.locator(".account-email")).toHaveText(fixture.email);
  await expect(panel).toContainText("默认头像");
  await expect(panel.locator("dt")).toHaveText(["登录方式", "邮箱", "套餐"]);
  await panel.screenshot({
    path: path.join(dir, "profile-light-test-only.png"),
  });
  await page.evaluate(() => (document.documentElement.dataset.theme = "dark"));
  await panel.screenshot({
    path: path.join(dir, "profile-dark-test-only.png"),
  });
  await refresh({ ...fixture, email: "second@example.test" });
  await expect(panel.locator(".account-email")).toHaveText(
    "second@example.test",
  );
  await expect(panel).not.toContainText("person@example.test");
  await refresh({ ...fixture, auth_type: "apiKey", email: null, plan: null });
  await expect(panel).toContainText("API Key");
  await expect(panel.locator(".account-email")).toHaveText("未提供");
  await refresh({
    status: "signed_out",
    auth_type: null,
    email: null,
    plan: null,
    message: null,
  });
  await expect(panel).toContainText("未登录");
  await expect(panel.locator(".account-email")).toHaveCount(0);
  fs.writeFileSync(
    path.join(dir, "profile.json"),
    JSON.stringify({ id: 2, error: { code: -1, message: "TEST_ONLY_SECRET" } }),
  );
  await panel.getByRole("button", { name: "刷新账号" }).click();
  await expect(panel).toContainText("账号资料读取失败");
  await expect(panel).not.toContainText("TEST_ONLY_SECRET");
  await expect(panel).not.toContainText("second@example.test");
  await refresh({
    ...fixture,
    email: "long-address-for-layout-verification@example.test",
  });
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("Emulation.setDeviceMetricsOverride", {
    width: 860,
    height: 640,
    deviceScaleFactor: 1,
    mobile: false,
  });
  const box = await panel.boundingBox();
  if (!box || box.x < 0 || box.x + box.width > 860)
    throw Error("Popover exceeds viewport");
  const storage = await page.evaluate(() =>
    JSON.stringify({ ...localStorage }),
  );
  if (storage.includes("example.test"))
    throw Error("Profile persisted to browser storage");
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "设置与数据源", exact: true }).click();
  await expect(page.locator(".provider .provider-mark.openai")).toHaveCount(1);
  const openaiLogo = page.locator(".provider .provider-mark.openai svg");
  if (await openaiLogo.evaluate((el) => getComputedStyle(el).color) !==
      await page.locator(".provider .provider-mark.openai").evaluate((el) => getComputedStyle(el).color)) {
    throw Error("OpenAI provider logo overrides the neutral brand color");
  }
  await expect(page.locator(".provider .provider-mark.unknown")).toHaveCount(0);
  for (const provider of ["claude-code", "deepseek", "glm", "kimi"]) {
    const row = page.locator(".provider").filter({
      has: page.locator(`.provider-mark.${provider}`),
    });
    await expect(row).toContainText("未支持");
    const loaded = await row.locator("img:visible").evaluate(
      (img) => img.complete && img.naturalWidth > 0,
    );
    if (!loaded) throw Error(`${provider} icon did not load`);
  }
  await page.locator(".provider").first().locator("..").screenshot({
    path: path.join(dir, "providers-dark-test-only.png"),
  });
  await page.evaluate(() => (document.documentElement.dataset.theme = "light"));
  await expect(page.locator(".provider-logo-light")).toBeVisible();
  await expect(page.locator(".provider-logo-dark")).not.toBeVisible();
  await page.locator(".provider").first().locator("..").screenshot({
    path: path.join(dir, "providers-light-test-only.png"),
  });
  await expect(page.locator("svg.lucide-terminal")).toHaveCount(0);
  if (errors.length) throw Error(errors.join("\n"));
  console.log(
    JSON.stringify({
      test_only: true,
      missing_cli: true,
      hover_keyboard_escape: true,
      synthetic_stdio_profile_states: true,
      account_switch_clears_old: true,
      no_browser_persistence: true,
      narrow_window: true,
      frontend_errors: 0,
    }),
  );
} finally {
  if (browser) await browser.close();
  child.kill();
  if (child.exitCode === null && child.signalCode === null)
    await new Promise((r) => child.once("exit", r));
}
