import { ProviderMark } from "./ProviderMark";
import type { Estimate } from "../lib/types";
import { money, coverage, type Currency } from "../lib/money";
import { exact, localTime } from "../lib/format";
import { Button } from "./ui/button";
export function Cost({
  estimate,
  currency,
  setCurrency,
  refresh,
  busy,
}: {
  estimate: Estimate | undefined;
  currency: Currency;
  setCurrency: (value: Currency) => void;
  refresh: () => void;
  busy: boolean;
}) {
  const value =
    estimate?.[currency.toLowerCase() as "usd" | "hkd" | "cny"] ?? null;
  return (
    <section className="panel explanation cost-panel">
      <div className="panel-head">
        <div>
          <h2>API 等价参考估算</h2>
          <p>标准服务 · 短上下文 · 不是套餐费用或实际账单</p>
        </div>
        <select
          aria-label="估算币别"
          value={currency}
          onChange={(e) => setCurrency(e.target.value as Currency)}
        >
          {["USD", "HKD", "CNY"].map((c) => (
            <option key={c}>{c}</option>
          ))}
        </select>
      </div>
      <strong
        className="cost-total"
        title={value ? currency + " " + value : "缺少可用价格或汇率"}
      >
        {money(value, currency)}
      </strong>
      <p>
        公开价格覆盖{" "}
        {estimate
          ? coverage(estimate.covered_tokens, estimate.uncovered_tokens)
          : "—"}{" "}
        已确认 Token；未覆盖 {estimate ? exact(estimate.uncovered_tokens) : "—"}{" "}
        Token。不推测内部模型价格。
      </p>
      <p>
        按 {estimate?.catalog.verified_at ?? "未知"}{" "}
        价格版本重估筛选范围用量；缓存为输入子集，推理为输出子集。未计工具、缓存写入差价、长上下文、加速档与地区溢价。
      </p>
      <details className="cost-details">
        <summary>查看模型估算明细</summary>
        {estimate?.models.map((m) => (
          <div className="group-row" key={m.model}>
            <span>
              <span className="model-label">
                <ProviderMark model={m.model} />
                {m.model}
              </span>
            </span>
            <strong title={m.source ?? "未核实公开单价"}>
              {m.usd === null ? "单价未知" : money(m.usd, "USD")}
            </strong>
          </div>
        ))}
      </details>
      <div className="panel-foot">
        <p>
          参考汇率：
          {estimate?.fx.status === "synced"
            ? "已同步"
            : estimate?.fx.status === "disabled"
              ? "自动更新已关闭，暂停换算"
              : "未同步或已过期，暂停换算"}
          ；每日更新，可在设置关闭。
        </p>
        <details className="cost-details">
          <summary>汇率来源与观测日期</summary>
          {estimate?.fx.rates.map((r) => (
            <p key={r.currency}>
              1 USD = {r.rate} {r.currency} · 观测日期 {r.date} ·{" "}
              {r.providers.join(", ") || "未提供贡献方明细"}
            </p>
          ))}
          {estimate?.fx.collected_at && (
            <p>采集时间 {localTime(estimate.fx.collected_at)}</p>
          )}
        </details>
        <p>{estimate?.fx.message}</p>
        <Button
          variant="outline"
          onClick={refresh}
          disabled={busy || !estimate?.fx.enabled}
          aria-busy={busy}
        >
          {busy ? "同步汇率中" : "同步参考汇率"}
        </Button>
      </div>
    </section>
  );
}
