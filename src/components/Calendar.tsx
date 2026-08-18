import { useEffect, useState } from "react";
import ReactCalendar from "react-calendar";
import "react-calendar/dist/Calendar.css";
import { api, RECORD_CHANGED_EVENT } from "../api";
import DetailModal from "./DetailModal";

const WEEKDAY_LABELS = ["日", "一", "二", "三", "四", "五", "六"];

function dateKey(date: Date): string {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, "0");
  const d = String(date.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

export default function Calendar() {
  const now = new Date();
  const [ym, setYm] = useState({ y: now.getFullYear(), m: now.getMonth() + 1 });
  const [counts, setCounts] = useState<Record<string, number>>({});
  const [todoCounts, setTodoCounts] = useState<Record<string, number>>({});
  const [selected, setSelected] = useState<string | null>(null);
  const [refresh, setRefresh] = useState(0);

  useEffect(() => {
    const m = String(ym.m).padStart(2, "0");
    api
      .getMonthActive(`${ym.y}-${m}`)
      .then((list) => {
        const map: Record<string, number> = {};
        const todoMap: Record<string, number> = {};
        list.forEach((d) => {
          map[d.date] = d.count;
          todoMap[d.date] = d.todoCount;
        });
        setCounts(map);
        setTodoCounts(todoMap);
      })
      .catch(() => {});
  }, [ym, refresh]);

  // 监听入库事件，实时刷新当前月的记录数
  useEffect(() => {
    const onRecordChanged = () => setRefresh((r) => r + 1);
    window.addEventListener(RECORD_CHANGED_EVENT, onRecordChanged);
    return () => window.removeEventListener(RECORD_CHANGED_EVENT, onRecordChanged);
  }, []);

  const open = (date: Date) => setSelected(dateKey(date));

  return (
    <>
      <div className="card" style={{ flex: 1, minHeight: 0, overflow: "auto" }}>
        <div className="cal-head">
          <h3>每日任务</h3>
        </div>
        <ReactCalendar
          locale="zh-CN"
          onClickDay={open}
          onActiveStartDateChange={({ activeStartDate }) => {
            if (activeStartDate) {
              const y = activeStartDate.getFullYear();
              const m = activeStartDate.getMonth() + 1;
              setYm((prev) => (prev.y === y && prev.m === m ? prev : { y, m }));
            }
          }}
          tileContent={({ date, view }) => {
            if (view !== "month") return null;
            const cnt = counts[dateKey(date)] || 0;
            const todoCnt = todoCounts[dateKey(date)] || 0;
            if (cnt === 0 && todoCnt === 0) return null;
            return (
              <span className="cal-badges">
                {cnt > 0 && <span className="cal-badge">{cnt}</span>}
                {todoCnt > 0 && <span className="cal-badge cal-badge-todo">{todoCnt}</span>}
              </span>
            );
          }}
          formatMonthYear={(_, date) => `${date.getFullYear()} 年 ${date.getMonth() + 1} 月`}
          formatShortWeekday={(_, date) => WEEKDAY_LABELS[date.getDay()]}
        />
      </div>
      {selected && <DetailModal date={selected} onClose={() => setSelected(null)} />}
    </>
  );
}
