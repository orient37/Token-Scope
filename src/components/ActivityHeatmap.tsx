import { useRef, useState } from "react";
import type { ActivityData } from "../lib/types";
import { compact, exact } from "../lib/format";
import { CalendarDays } from "lucide-react";
export function ActivityHeatmap({ data }: { data?: ActivityData }) {
  const [selected, setSelected] = useState<string | null>(null);
  const days = data?.days ?? [];
  const grid = useRef<HTMLDivElement>(null);
  const focusIndex = Math.max(
    0,
    days.findIndex((d) => d.date === selected),
  );
  const offset = days[0]?.weekday ?? 0;
  const detail = days.find((d) => d.date === selected);
  return (
    <section className="panel activity-panel">
      <div className="panel-head">
        <div>
          <h2>
            <CalendarDays size={18} /> 活跃热力图
          </h2>
          <p>近 365 天 · 本机时区 · 遵循上方时间、项目与模型筛选</p>
        </div>
        <span className="activity-summary">
          {data ? `${data.active_days} 个活跃日` : "尚未同步"}
        </span>
      </div>
      {days.length ? (
        <>
          <div className="heatmap-scroll">
            <div className="heatmap-layout">
              <div className="heatmap-weekdays">
                {["", "一", "", "三", "", "五", ""].map((s, i) => (
                  <span key={i}>{s}</span>
                ))}
              </div>
              <div>
                <div
                  className="heatmap-months"
                  style={{
                    gridTemplateColumns: `repeat(${Math.ceil((offset + days.length) / 7)}, 1fr)`,
                  }}
                >
                  {days
                    .filter((d, i) => i === 0 || d.date.endsWith("-01"))
                    .map((d) => {
                      const i = days.indexOf(d);
                      return (
                        <span
                          key={d.date}
                          style={{
                            gridColumn: Math.floor((offset + i) / 7) + 1,
                          }}
                        >
                          {Number(d.date.slice(5, 7))}月
                        </span>
                      );
                    })}
                </div>
                <div className="heatmap-grid" ref={grid}>
                  {Array.from({ length: offset }, (_, i) => (
                    <span key={`pad-${i}`} />
                  ))}
                  {days.map((day) => (
                    <button
                      key={day.date}
                      className={`heatmap-cell level-${day.level}`}
                      title={`${day.date} · ${exact(day.total)} Token`}
                      aria-label={`${day.date}，已确认 ${exact(day.total)} Token`}
                      tabIndex={days[focusIndex]?.date === day.date ? 0 : -1}
                      onKeyDown={(e) => {
                        const i = days.indexOf(day);
                        const target = {
                          ArrowLeft: i - 7,
                          ArrowRight: i + 7,
                          ArrowUp: i - 1,
                          ArrowDown: i + 1,
                          Home: 0,
                          End: days.length - 1,
                        }[e.key];
                        if (target !== undefined) {
                          e.preventDefault();
                          const next = Math.max(
                            0,
                            Math.min(days.length - 1, target),
                          );
                          setSelected(days[next].date);
                          grid.current
                            ?.querySelectorAll<HTMLButtonElement>("button")
                            [next]?.focus();
                        }
                      }}
                      aria-pressed={selected === day.date}
                      onClick={() => setSelected(day.date)}
                    />
                  ))}
                </div>
              </div>
            </div>
          </div>
          <div className="heatmap-foot">
            <span>
              {detail
                ? `${detail.date} · ${exact(detail.total)} Token`
                : `${days[0].date} — ${days[days.length - 1].date} · ${compact(data!.total)} Token`}
            </span>
            <div className="heatmap-scale">
              <span>少</span>
              {[0, 1, 2, 3, 4].map((n) => (
                <i key={n} className={`heatmap-cell level-${n}`} />
              ))}
              <span>多</span>
            </div>
          </div>
          <p className="chart-note">
            空白表示当前筛选内没有已确认消耗，不代表采集覆盖完整。颜色相对当前可见最高日用量分级；点击或用方向键选择，Home
            / End 跳到首末日，查看精确值。
          </p>
        </>
      ) : (
        <div className="empty">尚无已连接的数据，无法生成活跃日历</div>
      )}
    </section>
  );
}
