import { useEffect, useState } from "react";
import { api } from "../api";
import type { DayRecord } from "../types";
import FlowDiagram from "./FlowDiagram";

interface Props {
  date: string;
  onClose: () => void;
}

export default function DetailModal({ date, onClose }: Props) {
  const [record, setRecord] = useState<DayRecord | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setLoading(true);
    api
      .getDayRecord(date)
      .then((r) => setRecord(r))
      .catch(() => setRecord(null))
      .finally(() => setLoading(false));
  }, [date]);

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

        {record?.entries.map((e) => (
          <div key={e.id} className="entry">
            <div className="sum">{e.structured.summary || "（无摘要）"}</div>
            {e.structured.tags.length > 0 && (
              <div className="tag-row">
                {e.structured.tags.map((t, i) => (
                  <span key={i} className="tag">
                    {t}
                  </span>
                ))}
              </div>
            )}
            {e.structured.tasks.map((t, i) => (
              <div key={i} className="task">
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
          </div>
        ))}
      </div>
    </div>
  );
}
