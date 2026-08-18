import { useEffect, useState } from "react";
import { api, RECORD_CHANGED_EVENT } from "../api";
import type { DayRecord, RecordEntry, Structured } from "../types";
import FlowDiagram from "./FlowDiagram";

interface Props {
  date: string;
  onClose: () => void;
}

const inputStyle: React.CSSProperties = {
  width: "100%",
  padding: "6px 8px",
  fontSize: 13,
  borderRadius: 6,
  border: "1px solid var(--border, #d8dee6)",
  boxSizing: "border-box",
  fontFamily: "inherit",
};
const taStyle: React.CSSProperties = {
  ...inputStyle,
  minHeight: 60,
  resize: "vertical",
};

export default function DetailModal({ date, onClose }: Props) {
  const [record, setRecord] = useState<DayRecord | null>(null);
  const [loading, setLoading] = useState(true);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [confirmingId, setConfirmingId] = useState<string | null>(null);
  const [editSummary, setEditSummary] = useState("");
  const [editTags, setEditTags] = useState("");
  const [editTasks, setEditTasks] = useState("");
  const [editOutputs, setEditOutputs] = useState("");
  const [editTodos, setEditTodos] = useState("");
  const [error, setError] = useState("");

  const reload = () => {
    setLoading(true);
    api
      .getDayRecord(date)
      .then((r) => setRecord(r))
      .catch(() => setRecord(null))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    reload();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [date]);

  const startEdit = (e: RecordEntry) => {
    setEditingId(e.id);
    setEditSummary(e.structured.summary || "");
    setEditTags(e.structured.tags.join(", "));
    setEditTasks(e.structured.tasks.join("\n"));
    setEditOutputs(e.structured.outputs.join("\n"));
    setEditTodos(e.structured.todos.join("\n"));
    setError("");
  };

  const cancelEdit = () => {
    setEditingId(null);
    setError("");
  };

  const splitLines = (s: string) =>
    s
      .split("\n")
      .map((x) => x.trim())
      .filter(Boolean);
  const splitTags = (s: string) =>
    s
      .split(/[,，]/)
      .map((x) => x.trim())
      .filter(Boolean);

  const saveEdit = async (e: RecordEntry) => {
    const structured: Structured = {
      summary: editSummary.trim(),
      tasks: splitLines(editTasks),
      tags: splitTags(editTags),
      outputs: splitLines(editOutputs),
      flowcharts: e.structured.flowcharts,
      todos: splitLines(editTodos),
    };
    try {
      await api.updateEntry(date, e.id, structured);
      setEditingId(null);
      setError("");
      reload();
      window.dispatchEvent(new Event(RECORD_CHANGED_EVENT));
    } catch (err) {
      setError(String(err));
    }
  };

  const removeEntry = async (e: RecordEntry) => {
    try {
      await api.deleteEntry(date, e.id);
      setError("");
      setConfirmingId(null);
      reload();
      window.dispatchEvent(new Event(RECORD_CHANGED_EVENT));
    } catch (err) {
      setError(String(err));
    }
  };

  return (
    <div className="modal-mask" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-head">
          <div>
            <h2>{date}</h2>
            <div className="date-sub">
              {record ? `${record.entries.length} 条记录` : ""}
            </div>
          </div>
          <button className="btn btn-sm" onClick={onClose}>
            关闭
          </button>
        </div>

        {loading && <div className="empty">加载中…</div>}
        {!loading && (!record || record.entries.length === 0) && (
          <div className="empty">当天还没有记录</div>
        )}
        {error && (
          <div className="status-line" style={{ marginBottom: 8, color: "var(--red)" }}>
            {error}
          </div>
        )}

        {record?.entries.map((e) =>
          editingId === e.id ? (
            <div key={e.id} className="entry">
              <div className="weak" style={{ marginBottom: 8 }}>
                {e.kind === "todo" ? "编辑待办" : "编辑记录"}
              </div>
              <label className="weak" style={{ fontSize: 12 }}>摘要</label>
              <input
                style={inputStyle}
                value={editSummary}
                onChange={(ev) => setEditSummary(ev.target.value)}
                placeholder="一句话摘要"
              />
              {e.kind !== "todo" && (
                <>
                  <label className="weak" style={{ fontSize: 12, display: "block", marginTop: 8 }}>标签（逗号分隔）</label>
                  <input
                    style={inputStyle}
                    value={editTags}
                    onChange={(ev) => setEditTags(ev.target.value)}
                    placeholder="如：开发, 修复"
                  />
                </>
              )}
              <label className="weak" style={{ fontSize: 12, display: "block", marginTop: 8 }}>
                {e.kind === "todo" ? "待办列表（每行一项）" : "任务（每行一项）"}
              </label>
              <textarea
                style={taStyle}
                value={editTasks}
                onChange={(ev) => setEditTasks(ev.target.value)}
              />
              {e.kind !== "todo" && (
                <>
                  <label className="weak" style={{ fontSize: 12, display: "block", marginTop: 8 }}>产出（每行一项）</label>
                  <textarea
                    style={taStyle}
                    value={editOutputs}
                    onChange={(ev) => setEditOutputs(ev.target.value)}
                  />
                  <label className="weak" style={{ fontSize: 12, display: "block", marginTop: 8 }}>待办（每行一项）</label>
                  <textarea
                    style={taStyle}
                    value={editTodos}
                    onChange={(ev) => setEditTodos(ev.target.value)}
                  />
                </>
              )}
              <div style={{ marginTop: 12, display: "flex", gap: 8 }}>
                <button className="btn btn-primary" onClick={() => saveEdit(e)}>保存</button>
                <button className="btn" onClick={cancelEdit}>取消</button>
              </div>
            </div>
          ) : (
            <div key={e.id} className="entry">
              {e.kind === "todo" && <span className="entry-kind">待办</span>}
              <div className="sum">{e.structured.summary || (e.kind === "todo" ? "（待办）" : "（无摘要）")}</div>
              {e.kind !== "todo" && e.structured.tags.length > 0 && (
                <div className="tag-row">
                  {e.structured.tags.map((t, i) => (
                    <span key={i} className="tag">
                      {t}
                    </span>
                  ))}
                </div>
              )}
              {e.structured.tasks.map((t, i) => (
                <div key={i} className={e.kind === "todo" ? "task todo-item" : "task"}>
                  {e.kind === "todo" && <span className="todo-check">☐</span>}
                  {t}
                </div>
              ))}
              {e.structured.outputs.length > 0 && (
                <div className="weak" style={{ fontSize: 12, marginTop: 4 }}>
                  产出：{e.structured.outputs.join("、")}
                </div>
              )}
              {e.structured.flowcharts.map((f, i) => (
                <div key={i} className="flow">
                  <div className="flow-label">流程图 · {f.title}</div>
                  <FlowDiagram code={f.mermaid} />
                </div>
              ))}
              {e.structured.todos.length > 0 && (
                <div className="weak" style={{ fontSize: 12, marginTop: 4 }}>
                  待办：{e.structured.todos.join("、")}
                </div>
              )}
              <div style={{ marginTop: 10, display: "flex", gap: 8 }}>
                {confirmingId === e.id ? (
                  <>
                    <button className="btn btn-sm btn-danger" onClick={() => removeEntry(e)}>
                      确认删除
                    </button>
                    <button className="btn btn-sm" onClick={() => setConfirmingId(null)}>
                      取消
                    </button>
                  </>
                ) : (
                  <>
                    <button className="btn btn-sm" onClick={() => startEdit(e)}>编辑</button>
                    <button className="btn btn-sm" onClick={() => setConfirmingId(e.id)}>删除</button>
                  </>
                )}
              </div>
            </div>
          )
        )}
      </div>
    </div>
  );
}
