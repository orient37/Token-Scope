import { useState, type CSSProperties } from "react";
import { ChartNoAxesCombined, ChevronDown, ChevronUp } from "lucide-react";
import type { Group } from "../lib/types";
import { compact, exact } from "../lib/format";
import { distribution } from "../lib/distribution";
import { ProviderMark } from "./ProviderMark";
import { Button } from "./ui/button";

const colors = [
  "#b86b50",
  "#739188",
  "#8798b5",
  "#ad91aa",
  "#bda16e",
  "#8d9672",
  "#8f8b88",
];
export function UsageDistribution({
  groups = [],
  model = false,
}: {
  groups?: Group[];
  model?: boolean;
}) {
  const [expanded, setExpanded] = useState(false);
  const { rows, total } = distribution(groups);
  let cursor = 0;
  const stops = rows.map((row, index) => {
    const start = cursor;
    cursor += row.share;
    return `${colors[index % colors.length]} ${start}% ${cursor}%`;
  });
  const title = model ? "模型分布" : "项目消耗";
  return (
    <section className="panel distribution-panel" aria-label={title}>
      <div className="panel-head">
        <div>
          <h2>{title}</h2>
          <p>
            {model ? "已确认 Token 的模型占比" : "按已确认 Token 排序"} ·
            当前筛选
          </p>
        </div>
        <ChartNoAxesCombined size={18} className="muted" />
      </div>
      {rows.length && total !== "0" ? (
        <>
          {model && (
            <div className="distribution-summary">
              <div
                className="distribution-donut"
                role="img"
                aria-label={`模型占比，共 ${exact(total)} Token，详细数值见下方列表`}
                style={{ background: `conic-gradient(${stops.join(",")})` }}
              >
                <div>
                  <strong title={exact(total)}>{compact(total)}</strong>
                  <small>Token 总量</small>
                </div>
              </div>
              <div className="distribution-leader">
                <span>主要模型</span>
                <strong title={rows[0].name}>{rows[0].name}</strong>
                <b>
                  {rows[0].share.toFixed(1)}
                  <small>%</small>
                </b>
                <span>{rows.length} 个模型参与消耗</span>
              </div>
            </div>
          )}
          <div
            className={`distribution-list ${expanded ? "expanded" : ""}`}
            role="list"
          >
            {(expanded ? rows : rows.slice(0, 8)).map((row, index) => (
              <div
                className="distribution-row"
                role="listitem"
                key={row.name}
                tabIndex={0}
                title={`${row.name} · ${exact(row.totals.total)} Token · 占比 ${row.share.toFixed(2)}%`}
                style={
                  {
                    "--bar-color": colors[index % colors.length],
                  } as CSSProperties
                }
              >
                <div className="distribution-row-label">
                  <span>
                    {model ? (
                      <ProviderMark model={row.name} size={16} />
                    ) : (
                      <small className="distribution-rank">
                        {String(index + 1).padStart(2, "0")}
                      </small>
                    )}
                    <span className="distribution-name">{row.name}</span>
                  </span>
                  <strong title={exact(row.totals.total)}>
                    {compact(row.totals.total)}
                    <small>{row.share.toFixed(1)}%</small>
                  </strong>
                </div>
                <div className="distribution-track" aria-hidden="true">
                  <div style={{ width: `${row.width}%` }} />
                </div>
              </div>
            ))}
          </div>
          {rows.length > 8 && (
            <div className="distribution-more">
              <Button
                variant="outline"
                onClick={() => setExpanded(!expanded)}
                aria-expanded={expanded}
              >
                {expanded ? <ChevronUp size={14} /> : <ChevronDown size={14} />}{" "}
                {expanded ? "收起排行" : `查看全部 ${rows.length} 项`}
              </Button>
            </div>
          )}
        </>
      ) : (
        <div className="empty">当前筛选暂无已确认用量</div>
      )}
    </section>
  );
}
