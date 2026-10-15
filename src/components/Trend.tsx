import { useEffect, useMemo, useRef, useState } from "react";
import * as echarts from "echarts/core";
import { LineChart, BarChart, CandlestickChart } from "echarts/charts";
import {
  GridComponent,
  TooltipComponent,
  MarkPointComponent,
} from "echarts/components";
import { CanvasRenderer } from "echarts/renderers";
echarts.use([
  LineChart,
  BarChart,
  CandlestickChart,
  GridComponent,
  TooltipComponent,
  MarkPointComponent,
  CanvasRenderer,
]);
import type { Group, Candle } from "../lib/types";
import { compact, exact, localTime } from "../lib/format";
import { trendPeak, type TrendMode } from "../lib/trendPeak";
import { intervalLabel } from "../lib/monitor";
export function Trend({
  groups,
  interval = 86400,
  candles = [],
}: {
  groups: Group[];
  interval?: number;
  candles?: (Candle | null)[];
}) {
  const [mode, setMode] = useState<TrendMode>("line");
  const peak = useMemo(
    () => trendPeak(groups, candles, mode),
    [groups, candles, mode],
  );
  const peakKind = mode === "candlestick" ? "单条增量峰值" : "桶总量峰值";
  const peakDescription = peak
    ? `${peakKind} ${exact(peak.value)} Token · ${localTime(peak.at)} 至 ${localTime(new Date(Date.parse(peak.at) + interval * 1000).toISOString())}（不含结束）`
    : "暂无非零峰值";
  const node = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!node.current || !groups.length) return;
    const chart = echarts.init(node.current);
    chart.setOption({
      backgroundColor: "transparent",
      grid: { left: 52, right: 22, top: 50, bottom: 32 },
      tooltip: {
        trigger: "axis",
        backgroundColor: "#17222c",
        borderColor: "#31434f",
        textStyle: { color: "#e9f1f7" },
        formatter: (p: unknown) => {
          const a = p as { dataIndex: number }[];
          const index = a[0]?.dataIndex;
          const g = groups[index];
          if (!g) return "";
          const c = candles[index];
          return (
            localTime(g.name) +
            " 至 " +
            localTime(
              new Date(Date.parse(g.name) + interval * 1000).toISOString(),
            ) +
            "（不含结束）<br/>Token " +
            exact(g.totals.total) +
            (mode === "candlestick" && c
              ? `<br/>计量增量：首 ${exact(c.open)} · 末 ${exact(c.close)}<br/>最低 ${exact(c.low)} · 最高 ${exact(c.high)}<br/>${c.samples} 条已确认增量（非请求数）`
              : "")
          );
        },
      },
      xAxis: {
        type: "category",
        boundaryGap: mode !== "line",
        data: groups.map((g) => {
          const at = new Date(g.name);
          return at.toLocaleString("zh-CN", {
            month: "2-digit",
            day: "2-digit",
            ...(interval < 86400
              ? { hour: "2-digit", minute: "2-digit", hour12: false }
              : {}),
          });
        }),
        axisLine: { lineStyle: { color: "#2a3742" } },
        axisLabel: { color: "#8494a5" },
      },
      yAxis: {
        type: "value",
        axisLabel: {
          color: "#8494a5",
          formatter: (n: number) =>
            n >= 1e6
              ? (n / 1e6).toFixed(1) + "M"
              : n >= 1000
                ? (n / 1000).toFixed(0) + "K"
                : String(n),
        },
        splitLine: { lineStyle: { color: "#24313b", type: "dashed" } },
      },
      series: [
        {
          type: mode,
          smooth: false,
          symbolSize: 5,
          showSymbol: groups.length < 80,
          barMaxWidth: 22,
          data:
            mode === "candlestick"
              ? groups.map((_, i) => {
                  const c = candles[i];
                  return c
                    ? [
                        Number(c.open),
                        Number(c.close),
                        Number(c.low),
                        Number(c.high),
                      ]
                    : ["-", "-", "-", "-"];
                })
              : groups.map((g) => Number(g.totals.total)),
          lineStyle: { width: 2 },
          markPoint: {
            symbol: "circle",
            symbolSize: 10,
            data: peak
              ? [
                  {
                    name: peakKind,
                    coord: [peak.index, Number(peak.value)],
                    value: peak.value,
                  },
                ]
              : [],
            label: {
              show: true,
              position: "top",
              distance: 10,
              align: peak && peak.index > groups.length / 2 ? "right" : "left",
              formatter: peak ? `峰值 ${compact(peak.value)} Token` : "",
              fontSize: 11,
              fontWeight: "bold",
              padding: [5, 7],
              borderRadius: 5,
            },
            tooltip: { trigger: "item", formatter: peakDescription },
          },
        },
      ],
    });
    const restyle = () => {
      const css = getComputedStyle(document.documentElement);
      const token = (name: string) => css.getPropertyValue(name).trim();
      const accent = token("--accent");
      chart.setOption({
        animation: !matchMedia("(prefers-reduced-motion: reduce)").matches,
        tooltip: {
          backgroundColor: token("--surface"),
          borderColor: token("--border"),
          textStyle: { color: token("--text") },
        },
        xAxis: {
          axisLine: { lineStyle: { color: token("--border") } },
          axisLabel: { color: token("--muted") },
        },
        yAxis: {
          axisLabel: { color: token("--muted") },
          splitLine: { lineStyle: { color: token("--border") } },
        },
        series: [
          {
            lineStyle: { color: accent },
            markPoint: {
              itemStyle: {
                color: accent,
                borderColor: token("--surface"),
                borderWidth: 2,
              },
              label: {
                color: token("--text"),
                backgroundColor: token("--surface"),
                borderColor: accent,
                borderWidth: 1,
              },
            },
            itemStyle: {
              color: accent,
              color0: token("--success"),
              borderColor: accent,
              borderColor0: token("--success"),
              ...(mode === "bar" ? { borderRadius: [4, 4, 0, 0] } : {}),
            },
            areaStyle: {
              color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
                { offset: 0, color: accent + "30" },
                { offset: 1, color: accent + "00" },
              ]),
            },
          },
        ],
      });
    };
    restyle();
    const themeObserver = new MutationObserver(restyle);
    themeObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-theme"],
    });
    const observer = new ResizeObserver(() => chart.resize());
    observer.observe(node.current);
    return () => {
      observer.disconnect();
      themeObserver.disconnect();
      chart.dispose();
    };
  }, [groups, interval, candles, mode, peak, peakKind, peakDescription]);
  return (
    <div className="trend-wrap">
      <div className="trend-tools">
        <span>
          {mode === "candlestick"
            ? "单条增量分布 · 首 / 末 / 低 / 高"
            : "时间桶内已确认 Token 总量"}
        </span>
        <div className="chart-switch" role="group" aria-label="趋势图类型">
          {(
            [
              ["line", "折线"],
              ["bar", "柱状"],
              ["candlestick", "K 线"],
            ] as const
          ).map(([value, label]) => (
            <button
              key={value}
              aria-pressed={mode === value}
              onClick={() => setMode(value)}
            >
              {label}
            </button>
          ))}
        </div>
      </div>
      {groups.length > 0 && (
        <p
          className="trend-peak"
          aria-label="趋势峰值"
          title={peakDescription}
          data-peak-index={peak?.index}
          data-peak-value={peak?.value}
        >
          {peak
            ? `${peakKind} ${compact(peak.value)} Token · ${localTime(peak.at)}`
            : "暂无非零峰值"}
        </p>
      )}
      {groups.length ? (
        <div
          ref={node}
          className="trend"
          role="img"
          aria-label={
            (mode === "line"
              ? "折线图"
              : mode === "bar"
                ? "柱状图"
                : "K 线图") +
            "，按 UTC 对齐的 " +
            intervalLabel(interval) +
            "间隔聚合，本地时区显示，精确数据见悬浮提示"
          }
        />
      ) : (
        <div className="empty">此时间范围没有可确认的用量</div>
      )}
      {mode === "candlestick" && (
        <p className="chart-note">
          每根 K
          线为时间桶内单条计量增量的首、末、最低、最高值；空桶留空。与折线 /
          柱状的桶总量口径不同。
        </p>
      )}
    </div>
  );
}
