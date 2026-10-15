// TEST ONLY: streaming CSV integrity and alert pagination in native WebView.
import { chromium, expect } from "@playwright/test";
import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
const dir = path.resolve(".local/REPORTS_TEST_ONLY_" + Date.now());
fs.mkdirSync(dir, { recursive: true });
const db = path.join(dir, "tokenscope.sqlite3");
function python(code, args = []) {
  const r = spawnSync("python", ["-c", code, ...args], { encoding: "utf8" });
  if (r.status !== 0) throw Error(r.stderr);
  return r.stdout.trim();
}
python(
  `import sqlite3,pathlib,sys,json,datetime
c=sqlite3.connect(sys.argv[1])
for p in sorted(pathlib.Path('src-tauri/migrations').glob('*.sql')): c.executescript(p.read_text(encoding='utf8'))
c.execute("UPDATE app_settings SET value='false' WHERE key='fx_enabled'")
c.executescript("INSERT INTO accounts VALUES('TEST_ONLY','codex'); INSERT INTO projects VALUES('P','=TEST_ONLY_PROJECT'); INSERT INTO sessions(id,source_id,account_id,project_id) VALUES('S','REPORTS_TEST_ONLY','TEST_ONLY','P');")
for i in range(606):
 n=18446744073709551615 if i==0 else 1
 d=json.dumps(dict(input=n,cached=0,output=0,reasoning=0,total=n))
 stamp=(datetime.datetime(2026,10,9,tzinfo=datetime.timezone.utc)+datetime.timedelta(seconds=i)).strftime('%Y-%m-%dT%H:%M:%S.000000000Z')
 c.execute("INSERT INTO usage_events(id,session_id,event_at,collected_at,model,raw_total,delta,quality,normalization_version) VALUES(?,'S',?,?,?,'{}',?,'confirmed',3)",(f'TEST_ONLY_{i:04}',stamp,stamp,'=TEST_MODEL_A' if i%2==0 else 'TEST_MODEL_B',d))
c.execute("INSERT INTO usage_events(id,session_id,event_at,collected_at,model,raw_total,quality,normalization_version,carried_baseline) VALUES('UNKNOWN','S','2026-10-09T00:12:00.000000000Z','2026-10-09T00:12:00.000000000Z','=TEST_MODEL_A','{}','unknown',3,?)",(json.dumps(dict(input=7,cached=0,output=0,reasoning=0,total=7)),))
stamp=(datetime.datetime.now(datetime.timezone.utc)-datetime.timedelta(minutes=1)).strftime('%Y-%m-%dT%H:%M:%S.000000000Z')
for i in range(123):
 c.execute('INSERT INTO alerts VALUES(?,?,?,?,?,?,?,?)',(f'TEST_ONLY_{i:03}','usage_high' if i%3==0 else 'quota_risk','critical' if i%2==0 else 'warning',f'TEST_ONLY alert {i:03}','fixture',stamp,stamp,stamp if i%4==0 else None))
c.commit()`,
  [db],
);
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
      TOKENSCOPE_CODEX_BIN: path.join(dir, "missing.exe"),
      WEBVIEW2_USER_DATA_FOLDER: path.join(dir, "webview-profile"),
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:
        "--remote-debugging-port=9234 --remote-debugging-address=127.0.0.1",
    },
  },
);
let browser;
try {
  for (let i = 0; i < 30; i++) {
    try {
      browser = await chromium.connectOverCDP("http://127.0.0.1:9234");
      break;
    } catch {
      await new Promise((r) => setTimeout(r, 1000));
    }
  }
  if (!browser) throw Error("Native WebView unavailable");
  const page = browser.contexts().flatMap((c) => c.pages())[0],
    errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await expect(page.locator(".metric strong").first()).toHaveAttribute(
    "title",
    (18446744073709551615n + 605n).toLocaleString("zh-CN"),
  );
  await expect(page.getByText("开发验证模式", { exact: false })).toBeVisible();
  await expect(
    page.getByRole("button", { name: "导出 CSV", exact: true }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "会话", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "导出 CSV", exact: true }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "时间线", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "导出 CSV", exact: true }),
  ).toHaveCount(1);
  await page.getByRole("button", { name: "导出 CSV", exact: true }).click();
  const confirmation = page.getByRole("dialog", { name: "确认导出 CSV" });
  await expect(confirmation).toBeVisible();
  await expect(confirmation).toContainText("全部项目");
  await expect(
    confirmation.getByRole("button", { name: "取消", exact: true }),
  ).toBeFocused();
  await page.keyboard.press("Shift+Tab");
  await expect(
    confirmation.getByRole("button", { name: "确认导出", exact: true }),
  ).toBeFocused();
  await page.keyboard.press("Alt+1");
  await expect(
    page.getByRole("heading", { name: "消耗时间线", exact: true }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(confirmation).not.toBeVisible();
  await expect(
    page.getByRole("button", { name: "导出 CSV", exact: true }),
  ).toBeFocused();
  if (fs.existsSync(path.join(dir, "exports")))
    throw Error("Escape created export files");
  await page.getByRole("button", { name: "导出 CSV", exact: true }).click();
  await confirmation.getByRole("button", { name: "取消", exact: true }).click();
  await expect(confirmation).not.toBeVisible();
  if (fs.existsSync(path.join(dir, "exports")))
    throw Error("Cancel created export files");
  await page.getByRole("button", { name: "导出 CSV", exact: true }).click();
  await page.screenshot({
    path: path.join(dir, "export-confirmation-test-only.png"),
  });
  await confirmation
    .getByRole("button", { name: "确认导出", exact: true })
    .click();
  await expect(page.locator(".export-receipt")).toContainText("607 条记录");
  const exportsRoot = path.join(dir, "exports");
  const first = path.join(exportsRoot, fs.readdirSync(exportsRoot)[0]);
  const validation = JSON.parse(
    python(
      `import csv,json,pathlib,sys
root=pathlib.Path(sys.argv[1]); m=json.loads((root/'metadata.json').read_text(encoding='utf8'))
with (root/'usage.csv').open(encoding='utf-8-sig',newline='') as f: rows=list(csv.DictReader(f))
assert len(rows)==607 and m['rows']==607
assert sum(int(r['total_tokens_decimal']) for r in rows if r['confirmed']=='true')==18446744073709551615+605
assert m['totals']['total']==str(18446744073709551615+605)
u=[r for r in rows if r['confirmed']=='false']; assert len(u)==1 and u[0]['total_tokens_decimal']=='' and u[0]['carried_total_tokens_decimal']=='7'
assert rows[0]['project']=="'=TEST_ONLY_PROJECT" and rows[0]['model']=="'=TEST_MODEL_A"
assert m['carried_baseline']['total']=='7'
assert all('prompt' not in k and 'account' not in k for k in rows[0])
print(json.dumps(dict(rows=len(rows),exact=True,unknown_blank=True,carried_separate=True,formula_protected=True)))`,
      [first],
    ),
  );
  await page
    .getByRole("combobox", { name: "模型筛选" })
    .selectOption("TEST_MODEL_B");
  await page.getByRole("combobox", { name: "项目筛选" }).selectOption("P");
  await page.getByRole("button", { name: "导出 CSV", exact: true }).click();
  await expect(confirmation).toContainText("TEST_MODEL_B");
  await expect(confirmation).toContainText("=TEST_ONLY_PROJECT");
  await confirmation
    .getByRole("button", { name: "确认导出", exact: true })
    .click();
  await expect(page.locator(".export-receipt")).toContainText("303 条记录");
  if (fs.readdirSync(exportsRoot).length !== 2)
    throw Error("Export overwrote report");
  const second = path.join(
    exportsRoot,
    fs.readdirSync(exportsRoot).sort().at(-1),
  );
  const metadata = JSON.parse(
    fs.readFileSync(path.join(second, "metadata.json"), "utf8"),
  );
  if (
    metadata.filter.model !== "TEST_MODEL_B" ||
    metadata.filter.project !== "P" ||
    metadata.totals.total !== "303"
  )
    throw Error("Export filter mismatch");
  await page.getByRole("button", { name: /^预警/ }).click();
  await expect(page.locator(".alert-pagination")).toContainText("123 条匹配");
  await expect(page.locator(".alert-item")).toHaveCount(50);
  const firstPage = await page.locator(".alert-item h3").allTextContents();
  await page.getByRole("button", { name: "下一页", exact: true }).click();
  await expect(page.locator(".alert-pagination")).toContainText("第 2 / 3 页");
  const secondPage = await page.locator(".alert-item h3").allTextContents();
  if (firstPage.some((title) => secondPage.includes(title)))
    throw Error("Repeated page items");
  await page.getByRole("button", { name: "下一页", exact: true }).click();
  await expect(page.locator(".alert-item")).toHaveCount(23);
  await expect(
    page.getByRole("button", { name: "下一页", exact: true }),
  ).toBeDisabled();
  await page
    .getByRole("combobox", { name: "预警类型" })
    .selectOption("usage_high");
  await expect(page.locator(".alert-pagination")).toContainText("41 条匹配");
  await page
    .getByRole("combobox", { name: "预警级别" })
    .selectOption("critical");
  await expect(page.locator(".alert-pagination")).toContainText("21 条匹配");
  await page
    .getByRole("combobox", { name: "预警已读状态" })
    .selectOption("unread");
  await expect(page.locator(".alert-pagination")).toContainText("10 条匹配");
  await page
    .getByRole("button", { name: "标为已读", exact: true })
    .first()
    .click();
  await expect(page.locator(".alert-pagination")).toContainText("9 条匹配");
  await page.screenshot({
    path: path.join(dir, "pagination-test-only.png"),
    fullPage: true,
  });
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("Emulation.setDeviceMetricsOverride", {
    width: 860,
    height: 640,
    deviceScaleFactor: 1,
    mobile: false,
  });
  if (
    await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)
  )
    throw Error("Alert filters overflow narrow window");
  await page
    .getByRole("textbox", { name: "预警开始时间" })
    .fill("2099-01-01T00:00");
  await expect(page.locator(".alert-pagination")).toContainText("0 条匹配");
  if (errors.length) throw Error(errors.join("\n"));
  const report = {
    test_only: true,
    csv: validation,
    filtered_export: true,
    no_overwrite: true,
    confirmation_cancel_escape_keyboard: true,
    pagination: true,
    filters: true,
    ack_filtered: true,
    narrow_window: true,
    frontend_errors: errors.length,
  };
  fs.writeFileSync(
    path.join(dir, "report.json"),
    JSON.stringify(report, null, 2),
  );
  console.log(JSON.stringify(report));
} finally {
  if (browser) await browser.close();
  child.kill();
  if (child.exitCode === null && child.signalCode === null)
    await new Promise((r) => child.once("exit", r));
}
