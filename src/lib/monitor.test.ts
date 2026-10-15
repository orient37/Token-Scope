import { describe, expect, it } from "vitest";
import { collectorLabel, intervalLabel } from "./monitor";
import type { Health } from "./types";
describe("collector freshness and interval labels", () => {
  const now = Date.parse("2026-10-09T00:00:00Z");
  const collector: Health["collector"] = {
    status: "synced",
    last_attempt: new Date(now).toISOString(),
    last_success: new Date(now).toISOString(),
    next_retry: new Date(now + 3000).toISOString(),
    consecutive_failures: 0,
    recoveries: 0,
  };
  it("expires a previously connected collector when heartbeats stop", () => {
    expect(collectorLabel(collector, now)).toBe("本地采集已连接");
    expect(collectorLabel(collector, now + 19000)).toBe("采集状态已过期");
  });
  it("respects backoff and reports failures instead of healthy stale readings", () => {
    expect(
      collectorLabel(
        {
          ...collector,
          status: "error",
          next_retry: new Date(now + 30000).toISOString(),
        },
        now + 20000,
      ),
    ).toBe("采集异常 · 将重试");
    expect(collectorLabel({ ...collector, status: "degraded" }, now)).toContain(
      "部分采集失败",
    );
  });
  it("describes actual adaptive widths", () => {
    expect(intervalLabel(60)).toBe("1 分钟");
    expect(intervalLabel(120)).toBe("2 分钟");
    expect(intervalLabel(3600)).toBe("1 小时");
    expect(intervalLabel(172800)).toBe("2 天");
  });
});
