import { useEffect, useId, useRef, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { Download } from "lucide-react";
import type { Filter, Totals } from "../lib/types";
import { exact, localTime } from "../lib/format";
import { Button } from "./ui/button";
interface Report {
  directory: string;
  rows: number;
  uncertain: number;
  totals: Totals;
  exported_at: string;
}
export function ExportUsage({
  filter,
  projectLabel,
}: {
  filter: () => Filter;
  projectLabel?: string;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const titleId = useId();
  const descriptionId = useId();
  const inFlight = useRef(false);
  const [pending, setPending] = useState<{
    filter: Filter;
    project: string;
  } | null>(null);
  useEffect(() => {
    if (pending && !dialog.current?.open) dialog.current?.showModal();
    else if (!pending && dialog.current?.open) dialog.current.close();
  }, [pending]);
  const [busy, setBusy] = useState(false),
    [report, setReport] = useState<Report | null>(null),
    [error, setError] = useState("");
  const run = async (snapshot: Filter) => {
    if (inFlight.current) return;
    inFlight.current = true;
    setPending(null);
    setBusy(true);
    setError("");
    setReport(null);
    try {
      setReport(await invoke<Report>("export_usage", { filter: snapshot }));
    } catch (e) {
      setError(typeof e === "string" ? e : "导出失败，请重试");
    } finally {
      setBusy(false);
      inFlight.current = false;
    }
  };
  return (
    <section className="export-panel" aria-label="本地 CSV 导出">
      <div className="between">
        <div>
          <strong>导出当前筛选的计量事件</strong>
          <p>完整范围 · UTC 时间 · 未知增量留空 · CSV + 口径说明 JSON</p>
        </div>
        <Button
          variant="outline"
          disabled={busy || !isTauri()}
          aria-busy={busy}
          onClick={() => {
            const snapshot = { ...filter() };
            setPending({
              filter: snapshot,
              project: projectLabel || snapshot.project || "全部项目",
            });
          }}
        >
          <Download size={15} />
          {busy ? "正在导出…" : "导出 CSV"}
        </Button>
      </div>
      <dialog
        ref={dialog}
        className="panel export-confirm"
        aria-labelledby={titleId}
        aria-describedby={descriptionId}
        onCancel={() => setPending(null)}
        onKeyDown={(e) => {
          if (e.key !== "Tab") return;
          const buttons = Array.from(
            e.currentTarget.querySelectorAll<HTMLButtonElement>(
              "button:not(:disabled)",
            ),
          );
          const first = buttons[0],
            last = buttons[buttons.length - 1];
          if (e.shiftKey && document.activeElement === first) {
            e.preventDefault();
            last?.focus();
          } else if (!e.shiftKey && document.activeElement === last) {
            e.preventDefault();
            first?.focus();
          }
        }}
        onClick={(e) => {
          if (e.target !== e.currentTarget) return;
          const bounds = e.currentTarget.getBoundingClientRect();
          if (
            e.clientX < bounds.left ||
            e.clientX > bounds.right ||
            e.clientY < bounds.top ||
            e.clientY > bounds.bottom
          )
            setPending(null);
        }}
      >
        {pending && (
          <>
            <div className="panel-head">
              <h2 id={titleId}>确认导出 CSV</h2>
            </div>
            <div className="detail-body">
              <p id={descriptionId}>
                将导出以下筛选范围内的全部计量事件，包括未显示在表格中的记录。
              </p>
              <dl className="export-scope">
                <dt>项目</dt>
                <dd>{pending.project}</dd>
                <dt>模型</dt>
                <dd>{pending.filter.model || "全部模型"}</dd>
                <dt>开始时间</dt>
                <dd>
                  {pending.filter.from
                    ? localTime(pending.filter.from)
                    : "不限"}
                </dd>
                <dt>结束时间</dt>
                <dd>
                  {pending.filter.to
                    ? localTime(pending.filter.to)
                    : "不限（以导出快照为准）"}
                </dd>
                {pending.filter.session && (
                  <>
                    <dt>会话</dt>
                    <dd>{pending.filter.session}</dd>
                  </>
                )}
              </dl>
              <p>
                文件保存至本机应用数据目录下的 exports 独立目录，包含 usage.csv
                和 metadata.json。CSV 时间使用 UTC，未知增量留空；Token
                列导入电子表格时请设为文本。
              </p>
              <div className="export-confirm-actions">
                <Button
                  variant="outline"
                  autoFocus
                  onClick={() => setPending(null)}
                >
                  取消
                </Button>
                <Button onClick={() => void run(pending.filter)}>
                  确认导出
                </Button>
              </div>
            </div>
          </>
        )}
      </dialog>
      {error && (
        <p role="alert" className="warning">
          {error}
        </p>
      )}
      {report && (
        <div role="status" className="export-receipt">
          <p>
            已导出 {report.rows} 条记录 · 已确认 {exact(report.totals.total)}{" "}
            Token · {report.uncertain} 条未知增量。导出时刻：
            {localTime(report.exported_at)}
          </p>
          <p className="export-path">{report.directory}</p>
          <p>
            电子表格导入时请将 Token
            列设为文本，以保留完整整数。未分配历史单独列出，不计入已确认总量。
          </p>
          <Button
            variant="outline"
            onClick={() => {
              void invoke("open_exports").catch(() =>
                setError("无法打开目录，请复制上方路径"),
              );
            }}
          >
            打开导出目录
          </Button>
        </div>
      )}
    </section>
  );
}
