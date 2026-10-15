// LIVE READ-ONLY QUOTA: shell launch with an empty override and no Codex PATH.
// SQLite/WebView data are isolated; credentials are managed only by Codex.
import { chromium, expect } from "@playwright/test";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
const directory = path.resolve(".local/QUOTA_SHELL_TEST_ONLY_" + Date.now());
fs.mkdirSync(directory, { recursive: true });
const executable = path.resolve(
  process.env.TOKENSCOPE_TEST_EXE ?? "src-tauri/target/release/tokenscope.exe",
);
const shell = process.env.TOKENSCOPE_TEST_POWERSHELL ?? "pwsh.exe";
const env = {
  ...process.env,
  TOKENSCOPE_TEST_EXE: executable,
  TOKENSCOPE_TEST_MODE: "1",
  TOKENSCOPE_DATA_DIR: directory,
  TOKENSCOPE_CODEX_BIN: "",
  WEBVIEW2_USER_DATA_FOLDER: path.join(directory, "webview"),
  WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:
    "--remote-debugging-port=9254 --remote-debugging-address=127.0.0.1",
};
for (const key of Object.keys(env)) {
  if (key.toLowerCase() === "path")
    env[key] = env[key]
      .split(path.delimiter)
      .filter((p) => !/OpenAI[\\/]Codex[\\/]bin/i.test(p))
      .join(path.delimiter);
}
const launched = spawnSync(
  shell,
  [
    "-NoProfile",
    "-NonInteractive",
    "-Command",
    "[Environment]::SetEnvironmentVariable('TOKENSCOPE_CODEX_BIN','','Process'); $p=Start-Process -FilePath $env:TOKENSCOPE_TEST_EXE -WindowStyle Hidden -PassThru; $p.Id",
  ],
  { env, encoding: "utf8", windowsHide: true },
);
if (launched.status !== 0)
  throw Error(launched.stderr || "Shell launch failed");
const pid = Number(launched.stdout.trim());
if (!Number.isSafeInteger(pid) || pid <= 0) throw Error("No launched PID");
let browser;
try {
  for (let n = 0; n < 30; n++) {
    try {
      browser = await chromium.connectOverCDP("http://127.0.0.1:9254");
      break;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
  }
  if (!browser) throw Error("WebView unavailable");
  await expect
    .poll(() => browser.contexts().flatMap((c) => c.pages()).length, {
      timeout: 15000,
    })
    .toBeGreaterThan(0);
  const page = browser.contexts().flatMap((c) => c.pages())[0];
  await expect(
    page.getByRole("heading", { name: "用量总览", exact: true }),
  ).toBeVisible();
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  const quota = await page.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke("refresh_quota"),
  );
  if (
    quota.status !== "synced" ||
    !quota.collected_at ||
    !quota.data?.buckets.length
  )
    throw Error(quota.message || "No live quota windows");
  await expect(page.getByText("上次同步", { exact: false })).toBeVisible();
  await page
    .locator("#quota-panel")
    .screenshot({ path: path.join(directory, "live-quota.png") });
  const profile = await page.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke("read_account"),
  );
  if (
    profile.status !== "synced" ||
    profile.auth_type !== "chatgpt" ||
    !profile.email
  )
    throw Error("Live account unavailable");
  await page.getByRole("button", { name: "查看 Codex 账号资料" }).hover();
  const accountPanel = page.getByRole("dialog", { name: "Codex 账号资料" });
  await expect
    .poll(
      async () =>
        (await accountPanel.locator(".account-email").textContent()) ===
        profile.email,
      { timeout: 25000 },
    )
    .toBe(true);
  await expect(accountPanel).toContainText("默认头像");
  await page.mouse.move(400, 400);
  // Never log or screenshot the real email/profile.
  if (errors.length) throw Error(errors.join("\n"));
  console.log(
    JSON.stringify({
      shell_launch: true,
      live_account_email_matches: true,
      empty_override: true,
      no_codex_path: true,
      quota: "synced",
      pools: quota.data.buckets.length,
      frontend_errors: 0,
    }),
  );
} finally {
  if (browser) await browser.close();
  const stopped = spawnSync(
    shell,
    [
      "-NoProfile",
      "-NonInteractive",
      "-Command",
      "$p=Get-Process -Id ([int]$env:TOKENSCOPE_TEST_PID) -ErrorAction SilentlyContinue; if($p){if($p.Path -ne $env:TOKENSCOPE_TEST_EXE){throw 'Process path mismatch'}; Stop-Process -Id $p.Id}",
    ],
    {
      env: { ...env, TOKENSCOPE_TEST_PID: String(pid) },
      encoding: "utf8",
      windowsHide: true,
    },
  );
  if (stopped.status !== 0)
    throw Error(stopped.stderr || "Test process cleanup failed");
}
