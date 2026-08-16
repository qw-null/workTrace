import { useEffect, useState } from "react";
import { api, RECORD_CHANGED_EVENT } from "../api";

const LEVELS = ["--heat0", "--heat1", "--heat2", "--heat3", "--heat4"];

function fmt(d: Date): string {
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${m}-${day}`;
}

function level(count: number): number {
  if (count <= 0) return 0;
  if (count === 1) return 1;
  if (count === 2) return 2;
  if (count <= 4) return 3;
  return 4;
}

export default function Heatmap() {
  const [year, setYear] = useState(new Date().getFullYear());
  const [counts, setCounts] = useState<Map<string, number>>(new Map());
  const [refresh, setRefresh] = useState(0);

  useEffect(() => {
    api
      .getYearActive(String(year))
      .then((list) => {
        const m = new Map<string, number>();
        list.forEach((d) => m.set(d.date, d.count));
        setCounts(m);
      })
      .catch(() => {});
  }, [year, refresh]);

  // 监听入库事件，实时刷新当前年的活跃度
  useEffect(() => {
    const onRecordChanged = () => setRefresh((r) => r + 1);
    window.addEventListener(RECORD_CHANGED_EVENT, onRecordChanged);
    return () => window.removeEventListener(RECORD_CHANGED_EVENT, onRecordChanged);
  }, []);

  // 生成日期序列：从年初所在周的周日，到年末所在周的周六
  const first = new Date(year, 0, 1);
  const start = new Date(year, 0, 1 - first.getDay());
  const last = new Date(year, 11, 31);
  const end = new Date(year, 11, 31 + (6 - last.getDay()));

  const cells: { date: string; lvl: number; inYear: boolean }[] = [];
  const cursor = new Date(start);
  while (cursor <= end) {
    const inYear = cursor.getFullYear() === year;
    const dateStr = fmt(cursor);
    cells.push({
      date: dateStr,
      lvl: inYear ? level(counts.get(dateStr) || 0) : 0,
      inYear,
    });
    cursor.setDate(cursor.getDate() + 1);
  }

  return (
    <div className="card">
      <div className="heat-head">
        <h3>工作活跃度</h3>
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <button className="btn btn-sm" onClick={() => setYear(year - 1)}>
            ‹
          </button>
          <span className="weak">{year} 年</span>
          <button className="btn btn-sm" onClick={() => setYear(year + 1)}>
            ›
          </button>
        </div>
      </div>

      <div className="heat-grid">
        {cells.map((c, i) => (
          <span
            key={i}
            title={c.inYear ? `${c.date} · ${counts.get(c.date) || 0} 条记录` : ""}
            style={{ background: c.inYear ? `var(${LEVELS[c.lvl]})` : "transparent" }}
          />
        ))}
      </div>

      <div className="legend">
        少
        <i style={{ background: "var(--heat0)" }} />
        <i style={{ background: "var(--heat1)" }} />
        <i style={{ background: "var(--heat2)" }} />
        <i style={{ background: "var(--heat3)" }} />
        <i style={{ background: "var(--heat4)" }} />
        多
      </div>

      {counts.size === 0 && (
        <div className="weak" style={{ marginTop: 8, fontSize: 12 }}>
          {year} 年还没有记录，从今天开始记录吧
        </div>
      )}
    </div>
  );
}
