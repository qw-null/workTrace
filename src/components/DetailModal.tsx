import { useEffect, useState } from "react";
import { api, RECORD_CHANGED_EVENT } from "../api";
import type { DayRecord, RecordEntry, RecordField, Structured, TodoField } from "../types";

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

export default function DetailModal({ date, onClose }: Props) {
  const [record, setRecord] = useState<DayRecord | null>(null);
  const [loading, setLoading] = useState(true);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [confirmingId, setConfirmingId] = useState<string | null>(null);
  const [editRecords, setEditRecords] = useState<RecordField[]>([]);
  const [editTodos, setEditTodos] = useState<TodoField[]>([]);
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
    if (e.kind === "todo") {
      setEditTodos(e.structured.todoItems.length > 0 ? e.structured.todoItems : []);
      setEditRecords([]);
    } else {
      setEditRecords(e.structured.records.length > 0 ? e.structured.records : []);
      setEditTodos([]);
    }
    setError("");
  };

  const cancelEdit = () => {
    setEditingId(null);
    setError("");
  };

  // 更新某条记录字段
  const setRecordField = (i: number, key: keyof RecordField, val: string) => {
    setEditRecords((prev) => prev.map((r, idx) => (idx === i ? { ...r, [key]: val } : r)));
  };
  const addRecord = () => {
    setEditRecords((prev) => [...prev, { time: "", content: "", progress: "", people: "", next: "" }]);
  };
  const removeRecord = (i: number) => {
    setEditRecords((prev) => prev.filter((_, idx) => idx !== i));
  };

  // 更新某条待办字段
  const setTodoField = (i: number, key: keyof TodoField, val: string) => {
    setEditTodos((prev) => prev.map((t, idx) => (idx === i ? { ...t, [key]: val } : t)));
  };
  const addTodo = () => {
    setEditTodos((prev) => [...prev, { timeLocation: "", item: "", note: "" }]);
  };
  const removeTodo = (i: number) => {
    setEditTodos((prev) => prev.filter((_, idx) => idx !== i));
  };

  const saveEdit = async (e: RecordEntry) => {
    let structured: Structured;
    if (e.kind === "todo") {
      const items = editTodos.filter((t) => t.item.trim());
      structured = { ...e.structured, todoItems: items };
    } else {
      const items = editRecords.filter((r) => r.content.trim());
      structured = { ...e.structured, records: items };
    }
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
              {e.kind === "todo" ? (
                <>
                  {editTodos.map((t, i) => (
                    <div key={i} className="edit-block">
                      <div className="weak" style={{ fontSize: 12, marginBottom: 4 }}>
                        待办 {i + 1}
                        {editTodos.length > 1 && (
                          <button
                            className="btn btn-sm"
                            style={{ marginLeft: 8 }}
                            onClick={() => removeTodo(i)}
                          >
                            删除
                          </button>
                        )}
                      </div>
                      <input
                        style={inputStyle}
                        value={t.item}
                        onChange={(ev) => setTodoField(i, "item", ev.target.value)}
                        placeholder="事项"
                      />
                      <input
                        style={{ ...inputStyle, marginTop: 6 }}
                        value={t.timeLocation}
                        onChange={(ev) => setTodoField(i, "timeLocation", ev.target.value)}
                        placeholder="时间地点（无则留空）"
                      />
                      <input
                        style={{ ...inputStyle, marginTop: 6 }}
                        value={t.note}
                        onChange={(ev) => setTodoField(i, "note", ev.target.value)}
                        placeholder="注意点（无则留空）"
                      />
                    </div>
                  ))}
                  <button className="btn btn-sm" style={{ marginTop: 6 }} onClick={addTodo}>
                    + 添加待办
                  </button>
                </>
              ) : (
                <>
                  {editRecords.map((r, i) => (
                    <div key={i} className="edit-block">
                      <div className="weak" style={{ fontSize: 12, marginBottom: 4 }}>
                        记录 {i + 1}
                        {editRecords.length > 1 && (
                          <button
                            className="btn btn-sm"
                            style={{ marginLeft: 8 }}
                            onClick={() => removeRecord(i)}
                          >
                            删除
                          </button>
                        )}
                      </div>
                      <input
                        style={inputStyle}
                        value={r.content}
                        onChange={(ev) => setRecordField(i, "content", ev.target.value)}
                        placeholder="工作内容"
                      />
                      <input
                        style={{ ...inputStyle, marginTop: 6 }}
                        value={r.time}
                        onChange={(ev) => setRecordField(i, "time", ev.target.value)}
                        placeholder="时间（无则留空）"
                      />
                      <input
                        style={{ ...inputStyle, marginTop: 6 }}
                        value={r.progress}
                        onChange={(ev) => setRecordField(i, "progress", ev.target.value)}
                        placeholder="进度/结果（无则留空）"
                      />
                      <input
                        style={{ ...inputStyle, marginTop: 6 }}
                        value={r.people}
                        onChange={(ev) => setRecordField(i, "people", ev.target.value)}
                        placeholder="相关人员（无则留空）"
                      />
                      <input
                        style={{ ...inputStyle, marginTop: 6 }}
                        value={r.next}
                        onChange={(ev) => setRecordField(i, "next", ev.target.value)}
                        placeholder="备注/下一步（无则留空）"
                      />
                    </div>
                  ))}
                  <button className="btn btn-sm" style={{ marginTop: 6 }} onClick={addRecord}>
                    + 添加记录
                  </button>
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
              {e.kind === "todo" ? (
                e.structured.todoItems.length > 0 ? (
                  e.structured.todoItems.map((t, i) => (
                    <div key={i} className="task todo-item">
                      <span className="todo-check">☐</span>
                      <div>
                        <div>{t.item || "（无内容）"}</div>
                        {(t.timeLocation && t.timeLocation !== "无") || (t.note && t.note !== "无") ? (
                          <div className="weak" style={{ fontSize: 12, marginTop: 4 }}>
                            {t.timeLocation && t.timeLocation !== "无" && <>时间地点：{t.timeLocation}</>}
                            {t.timeLocation && t.timeLocation !== "无" && t.note && t.note !== "无" && "；"}
                            {t.note && t.note !== "无" && <>注意点：{t.note}</>}
                          </div>
                        ) : null}
                      </div>
                    </div>
                  ))
                ) : (
                  // 兼容旧数据
                  <>
                    <div className="sum">{e.structured.summary || "（待办）"}</div>
                    {e.structured.tasks.map((t, i) => (
                      <div key={i} className="task todo-item">
                        <span className="todo-check">☐</span>
                        {t}
                      </div>
                    ))}
                  </>
                )
              ) : e.structured.records.length > 0 ? (
                e.structured.records.map((r, i) => (
                  <div key={i} className="entry-result-item">
                    {e.structured.records.length > 1 && (
                      <div className="weak" style={{ fontSize: 12, marginBottom: 4 }}>
                        记录 {i + 1}
                      </div>
                    )}
                    <div className="sum">{r.content || "（无内容）"}</div>
                    {r.time && r.time !== "无" && (
                      <div className="weak" style={{ fontSize: 12, marginTop: 4 }}>时间：{r.time}</div>
                    )}
                    {r.progress && r.progress !== "无" && (
                      <div className="weak" style={{ fontSize: 12, marginTop: 4 }}>进度/结果：{r.progress}</div>
                    )}
                    {r.people && r.people !== "无" && (
                      <div className="weak" style={{ fontSize: 12, marginTop: 4 }}>相关人员：{r.people}</div>
                    )}
                    {r.next && r.next !== "无" && (
                      <div className="weak" style={{ fontSize: 12, marginTop: 4 }}>备注/下一步：{r.next}</div>
                    )}
                  </div>
                ))
              ) : (
                // 兼容旧数据
                <>
                  <div className="sum">{e.structured.summary || "（无摘要）"}</div>
                  {e.structured.tags.length > 0 && (
                    <div className="tag-row">
                      {e.structured.tags.map((t, i) => (
                        <span key={i} className="tag">{t}</span>
                      ))}
                    </div>
                  )}
                  {e.structured.tasks.map((t, i) => (
                    <div key={i} className="task">{t}</div>
                  ))}
                  {e.structured.outputs.length > 0 && (
                    <div className="weak" style={{ fontSize: 12, marginTop: 4 }}>
                      产出：{e.structured.outputs.join("、")}
                    </div>
                  )}
                </>
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
