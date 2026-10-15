import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
} from "react";
import { createPortal } from "react-dom";
import type { Group } from "../lib/types";
import { distribution } from "../lib/distribution";
import { compact, exact } from "../lib/format";

const colors = [
  "#d56b42",
  "#168b80",
  "#427bd0",
  "#9561ca",
  "#c79724",
  "#cb5687",
  "#269db6",
  "#6d963d",
];
export function ProjectBubbles({ groups = [] }: { groups?: Group[] }) {
  const container = useRef<HTMLDivElement>(null);
  // Keep a project's color while live totals reorder the cluster.
  const projectColors = useRef(new Map<string, string>());
  const [width, setWidth] = useState(420);
  const [active, setActive] = useState<{
    name: string;
    value: string;
    x: number;
    y: number;
  } | null>(null);
  useEffect(() => {
    if (!container.current) return;
    const observer = new ResizeObserver(([entry]) => {
      setWidth(Math.max(240, entry.contentRect.width));
    });
    observer.observe(container.current);
    const dismiss = () => setActive(null);
    window.addEventListener("scroll", dismiss, true);
    window.addEventListener("resize", dismiss);
    return () => {
      observer.disconnect();
      window.removeEventListener("scroll", dismiss, true);
      window.removeEventListener("resize", dismiss);
    };
  }, []);
  const { rows, total } = useMemo(() => distribution(groups), [groups]);
  for (const row of rows) {
    if (!projectColors.current.has(row.name)) {
      projectColors.current.set(
        row.name,
        colors[projectColors.current.size % colors.length],
      );
    }
  }
  const layout = useMemo(() => {
    const placed: { x: number; y: number; r: number }[] = [];
    for (const row of rows) {
      const r = 29 + 47 * Math.sqrt(row.width / 100);
      let selected: { x: number; y: number; r: number } | undefined;
      for (let attempt = 0; attempt < 1600; attempt++) {
        const angle = attempt * 2.399963;
        const distance = 7 * Math.sqrt(attempt);
        const x = width / 2 + Math.cos(angle) * distance;
        const y = 185 + Math.sin(angle) * distance;
        if (x < r + 12 || x > width - r - 12 || y < r + 16) continue;
        if (
          placed.every((p) => Math.hypot(x - p.x, y - p.y) >= (r + p.r) * 0.87)
        ) {
          selected = { x, y, r };
          break;
        }
      }
      // Keep every project reachable when the compact cluster fills up.
      placed.push(
        selected ?? {
          x: width / 2,
          y:
            placed.reduce((bottom, p) => Math.max(bottom, p.y + p.r), 0) +
            r +
            12,
          r,
        },
      );
    }
    return {
      nodes: placed,
      height: placed.reduce(
        (bottom, p) => Math.max(bottom, p.y + p.r + 24),
        360,
      ),
    };
  }, [rows, width]);
  const show = (element: HTMLButtonElement, row: (typeof rows)[number]) => {
    const box = element.getBoundingClientRect();
    setActive({
      name: row.name,
      value: row.totals.total,
      x: Math.max(
        12,
        Math.min(window.innerWidth - 292, box.x + box.width / 2 - 140),
      ),
      y: Math.max(12, Math.min(window.innerHeight - 112, box.bottom + 10)),
    });
  };
  const current = active && rows.find((row) => row.name === active.name);
  return (
    <section
      className={`panel project-bubble-panel ${current ? "has-active-bubble" : ""}`}
      aria-label="项目消耗"
    >
      <div className="panel-head">
        <div>
          <h2>项目消耗</h2>
          <p>每个球代表一个项目 · 悬停查看用量</p>
        </div>
        <span className="tag">{rows.length} 个项目</span>
      </div>
      <div
        ref={container}
        className="project-bubble-cloud"
        style={{ height: rows.length && total !== "0" ? layout.height : 220 }}
      >
        {rows.length && total !== "0" ? (
          rows.map((row, i) => {
            const node = layout.nodes[i];
            return (
              <button
                key={row.name}
                type="button"
                className={`project-bubble ${current?.name === row.name ? "is-active" : ""}`}
                style={
                  {
                    left: node.x - node.r,
                    top: node.y - node.r,
                    width: node.r * 2,
                    height: node.r * 2,
                    "--bubble-color": projectColors.current.get(row.name),
                    "--float-delay": `${-i * 1.37}s`,
                    "--float-duration": `${9 + (i % 5)}s`,
                    "--breath-duration": `${5.5 + (i % 4) * 0.8}s`,
                  } as CSSProperties
                }
                aria-label={`${row.name}，${exact(row.totals.total)} Token，占比 ${row.share.toFixed(2)}%`}
                aria-describedby={
                  current?.name === row.name
                    ? "project-bubble-detail"
                    : undefined
                }
                onMouseEnter={(e) => show(e.currentTarget, row)}
                onMouseLeave={() => setActive(null)}
                onFocus={(e) => show(e.currentTarget, row)}
                onBlur={() => setActive(null)}
                onClick={(e) => show(e.currentTarget, row)}
                onKeyDown={(e) => {
                  if (e.key === "Escape") setActive(null);
                }}
              >
                <span className="bubble-name">
                  {row.name.replace(/ · [a-f0-9]+$/, "")}
                </span>
                <strong>{compact(row.totals.total)}</strong>
              </button>
            );
          })
        ) : (
          <div className="empty">当前筛选暂无已确认用量</div>
        )}
      </div>
      <div className="panel-foot">
        {exact(total)} Token · 当前筛选 · 球越大消耗越高，小球设最小尺寸以便操作
      </div>
      {current &&
        active &&
        createPortal(
          <div
            role="tooltip"
            id="project-bubble-detail"
            className="project-bubble-tooltip"
            style={{ left: active.x, top: active.y }}
          >
            <strong>{current.name}</strong>
            <span>{exact(current.totals.total)} Token</span>
            <small>占当前筛选 {current.share.toFixed(2)}%</small>
          </div>,
          document.body,
        )}
    </section>
  );
}
