import { useEffect, useState } from "react";
import Dashboard from "./views/Dashboard";
import Report from "./views/Report";
import Settings from "./views/Settings";
import { api, BACKUP_CHANGED_EVENT } from "./api";
import logoUrl from "./assets/logo.svg";

type Page = "dash" | "report" | "settings";

const NAV: { key: Page; label: string }[] = [
  { key: "dash", label: "工作台" },
  { key: "report", label: "周报" },
  { key: "settings", label: "设置" },
];

export default function App() {
  const [page, setPage] = useState<Page>("dash");
  const [backupName, setBackupName] = useState("");

  const refreshBackup = async () => {
    try {
      const accounts = await api.listWebdavConfigs();
      const def = accounts.find((a) => a.isDefault) || accounts[0];
      setBackupName(def ? def.name : "");
    } catch {
      /* 后端不可用时忽略 */
    }
  };

  useEffect(() => {
    refreshBackup();
    const onBackupChanged = () => {
      void refreshBackup();
    };
    window.addEventListener(BACKUP_CHANGED_EVENT, onBackupChanged);
    return () => window.removeEventListener(BACKUP_CHANGED_EVENT, onBackupChanged);
  }, []);

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="logo">
          <img className="logo-mark" src={logoUrl} alt="工作日迹" />
          <span className="logo-name">工作日迹</span>
        </div>
        <nav className="nav">
          {NAV.map((n) => (
            <button
              key={n.key}
              className={"nav-item" + (page === n.key ? " active" : "")}
              onClick={() => setPage(n.key)}
            >
              {n.label}
            </button>
          ))}
        </nav>
        <div className="side-foot">
          <span className="sync-dot" /> 备份：{backupName || "未配置"}
        </div>
      </aside>

      <main className="content">
        {page === "dash" && <Dashboard />}
        {page === "report" && <Report />}
        {page === "settings" && <Settings />}
      </main>
    </div>
  );
}
