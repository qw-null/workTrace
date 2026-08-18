import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import { api } from "../api";
import type { ModelConfig, Report } from "../types";

function mondayOf(d: Date): Date {
  const date = new Date(d);
  const day = date.getDay();
  const diff = day === 0 ? -6 : 1 - day;
  date.setDate(date.getDate() + diff);
  date.setHours(0, 0, 0, 0);
  return date;
}

function fmtDate(d: Date): string {
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${m}-${day}`;
}

function addDays(d: Date, n: number): Date {
  const date = new Date(d);
  date.setDate(date.getDate() + n);
  return date;
}

// 极简 Markdown 渲染：## 标题、- 列表、段落
function renderMarkdown(md: string): ReactNode[] {
  const lines = md.split("\n");
  const nodes: ReactNode[] = [];
  let list: string[] = [];
  let key = 0;

  const flushList = () => {
    if (list.length) {
      nodes.push(
        <ul key={"ul" + key++}>
          {list.map((x, j) => (
            <li key={j}>{x}</li>
          ))}
        </ul>
      );
      list = [];
    }
  };

  lines.forEach((line, idx) => {
    if (line.startsWith("## ")) {
      flushList();
      nodes.push(<h2 key={idx}>{line.slice(3)}</h2>);
    } else if (line.startsWith("- ")) {
      list.push(line.slice(2));
    } else if (line.trim() === "") {
      flushList();
    } else {
      flushList();
      nodes.push(<p key={idx}>{line}</p>);
    }
  });
  flushList();
  return nodes;
}

export default function Report() {
  const [weekStart, setWeekStart] = useState<Date>(mondayOf(new Date()));
  const [models, setModels] = useState<ModelConfig[]>([]);
  const [modelId, setModelId] = useState("");
  const [report, setReport] = useState<Report | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [exportPath, setExportPath] = useState("");
  const [savedMsg, setSavedMsg] = useState("");
  const [editing, setEditing] = useState(false);
  const [editContent, setEditContent] = useState("");
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    api
      .listModels()
      .then((ms) => {
        setModels(ms);
        const rep = ms.find((m) => m.role === "report") || ms.find((m) => m.isDefault) || ms[0];
        if (rep) setModelId(rep.id);
      })
      .catch(() => {});
  }, []);

  // 切换周次时：清空当前内容，若该周已有保存的周报则加载显示，否则显示「无内容」
  useEffect(() => {
    setReport(null);
    setError("");
    setExportPath("");
    setSavedMsg("");
    setEditing(false);
    const ws = fmtDate(weekStart);
    api
      .getReport(ws)
      .then((r) => {
        if (r) setReport(r);
      })
      .catch(() => {});
  }, [weekStart]);

  const weekEnd = addDays(weekStart, 6);
  const weekLabel = `${fmtDate(weekStart)} – ${fmtDate(weekEnd)}`;

  const generate = async () => {
    if (!modelId) {
      setError("请先在「设置」配置模型");
      return;
    }
    setLoading(true);
    setError("");
    try {
      const r = await api.generateReport(fmtDate(weekStart), modelId);
      setReport(r);
      setSavedMsg("已保存到周报库，可自动同步备份");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleExport = async (format: string) => {
    if (!report) return;
    try {
      const path = await api.exportReport(report.content, report.weekStart, format);
      setExportPath(path);
    } catch (e) {
      setError(String(e));
    }
  };

  const startEdit = () => {
    if (!report) return;
    setEditContent(report.content);
    setEditing(true);
    setError("");
    setSavedMsg("");
  };

  const cancelEdit = () => {
    setEditing(false);
    setEditContent("");
  };

  const saveEdit = async () => {
    if (!report) return;
    setSaving(true);
    setError("");
    try {
      await api.saveReport(report.weekStart, editContent);
      setReport({ ...report, content: editContent, generatedAt: new Date().toISOString() });
      setEditing(false);
      setSavedMsg("修改已保存");
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div>
      <div className="report-toolbar">
        <button className="btn" onClick={() => setWeekStart(addDays(weekStart, -7))}>
          ‹ 上一周
        </button>
        <button className="btn" onClick={() => setWeekStart(addDays(weekStart, 7))}>
          下一周 ›
        </button>
        <span className="week-range">{weekLabel}</span>
        <span style={{ flex: 1 }} />
        {models.length > 0 && (
          <select
            value={modelId}
            onChange={(e) => setModelId(e.target.value)}
            style={{ padding: "6px 10px", borderRadius: 8, border: "1px solid var(--border-strong)", fontSize: 13 }}
          >
            {models.map((m) => (
              <option key={m.id} value={m.id}>
                {m.name}
              </option>
            ))}
          </select>
        )}
        <button className="btn btn-primary" onClick={generate} disabled={loading}>
          {loading ? "生成中…" : "生成周报"}
        </button>
        {report && !editing && (
          <button className="btn" onClick={startEdit}>
            编辑
          </button>
        )}
        <button className="btn" onClick={() => handleExport("word")} disabled={!report || editing}>
          导出 Word
        </button>
        <button className="btn" onClick={() => handleExport("pdf")} disabled={!report || editing}>
          导出 PDF
        </button>
      </div>

      {error && (
        <div className="status-line" style={{ marginBottom: 12, color: "var(--red)" }}>
          {error}
        </div>
      )}
      {exportPath && (
        <div className="status-line" style={{ marginBottom: 12 }}>
          <span className="sync-dot" /> 已导出：{exportPath}
        </div>
      )}
      {savedMsg && !error && (
        <div className="status-line" style={{ marginBottom: 12 }}>
          <span className="sync-dot" style={{ background: "var(--green)" }} /> {savedMsg}
        </div>
      )}

      {report ? (
        editing ? (
          <div className="report-body">
            <h1>工作周报（{report.weekStart} – {report.weekEnd}）</h1>
            <div className="weak" style={{ marginBottom: 10, fontSize: 12 }}>
              支持 Markdown：## 标题、- 列表
            </div>
            <textarea
              value={editContent}
              onChange={(e) => setEditContent(e.target.value)}
              style={{
                width: "100%",
                minHeight: 420,
                padding: 14,
                fontSize: 14,
                lineHeight: 1.8,
                borderRadius: 10,
                border: "1px solid var(--border-strong)",
                boxSizing: "border-box",
                fontFamily: "inherit",
                resize: "vertical",
              }}
            />
            <div style={{ marginTop: 12, display: "flex", gap: 10 }}>
              <button className="btn btn-primary" onClick={saveEdit} disabled={saving}>
                {saving ? "保存中…" : "保存"}
              </button>
              <button className="btn" onClick={cancelEdit}>取消</button>
            </div>
          </div>
        ) : (
          <div className="report-body">
            <h1>工作周报（{report.weekStart} – {report.weekEnd}）</h1>
            <div className="report-meta">
              生成于 {new Date(report.generatedAt).toLocaleString()} · 模型 {report.modelUsed}
            </div>
            {renderMarkdown(report.content)}
          </div>
        )
      ) : (
        !loading && (
          <div className="report-body">
            <div className="empty" style={{ padding: 60 }}>
              <div>无内容</div>
              <div className="weak" style={{ marginTop: 8 }}>
                该周还没有周报，点击「生成周报」汇总本周记录
              </div>
            </div>
          </div>
        )
      )}
    </div>
  );
}
