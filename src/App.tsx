import { useEffect, useState } from "react";
import Dashboard from "./views/Dashboard";
import Report from "./views/Report";
import Settings from "./views/Settings";
import { api, BACKUP_CHANGED_EVENT, CHECK_UPDATE_EVENT, UPDATE_STATUS_EVENT } from "./api";
import type { UpdateInfo, UpdateStatus } from "./api";
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
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [updating, setUpdating] = useState(false);
  const [updateProgress, setUpdateProgress] = useState(0);
  const [updateError, setUpdateError] = useState("");

  const refreshBackup = async () => {
    try {
      const accounts = await api.listWebdavConfigs();
      const def = accounts.find((a) => a.isDefault) || accounts[0];
      setBackupName(def ? def.name : "");
    } catch {
      /* 后端不可用时忽略 */
    }
  };

  const doCheckUpdate = async () => {
    // 广播「检查中」状态
    window.dispatchEvent(
      new CustomEvent<UpdateStatus>(UPDATE_STATUS_EVENT, {
        detail: { status: "checking" },
      })
    );
    try {
      const info = await api.checkUpdate();
      setUpdateInfo(info);
      window.dispatchEvent(
        new CustomEvent<UpdateStatus>(UPDATE_STATUS_EVENT, {
          detail: info
            ? { status: "available", version: info.version }
            : { status: "uptodate", version: undefined },
        })
      );
    } catch {
      // 检查失败（无网络 / 开发环境无更新配置）
      setUpdateInfo(null);
      window.dispatchEvent(
        new CustomEvent<UpdateStatus>(UPDATE_STATUS_EVENT, {
          detail: { status: "error", message: "检查更新失败，请确认网络或更新服务器是否可用" },
        })
      );
    }
  };

  useEffect(() => {
    refreshBackup();
    const onBackupChanged = () => {
      void refreshBackup();
    };
    window.addEventListener(BACKUP_CHANGED_EVENT, onBackupChanged);

    // 启动后延迟自动检查更新
    const timer = setTimeout(() => {
      void doCheckUpdate();
    }, 5000);
    const onCheckUpdate = () => {
      void doCheckUpdate();
    };
    window.addEventListener(CHECK_UPDATE_EVENT, onCheckUpdate);

    return () => {
      window.removeEventListener(BACKUP_CHANGED_EVENT, onBackupChanged);
      window.removeEventListener(CHECK_UPDATE_EVENT, onCheckUpdate);
      clearTimeout(timer);
    };
  }, []);

  const doInstall = async () => {
    setUpdating(true);
    setUpdateError("");
    try {
      await api.installUpdate((p) => setUpdateProgress(p));
    } catch (e) {
      setUpdateError(String(e));
      setUpdating(false);
    }
  };

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
        {updateInfo && (
          <div className="update-banner">
            <div className="update-banner-text">
              <strong>发现新版本 v{updateInfo.version}</strong>
              {updateInfo.notes && (
                <span className="weak" style={{ marginLeft: 10, fontSize: 12 }}>
                  {updateInfo.notes}
                </span>
              )}
            </div>
            <div className="update-banner-actions">
              {updateError && (
                <span className="weak" style={{ color: "var(--red)", fontSize: 12 }}>
                  {updateError}
                </span>
              )}
              <button
                className="btn btn-sm btn-primary"
                onClick={doInstall}
                disabled={updating}
              >
                {updating ? `更新中 ${updateProgress}%` : "立即更新"}
              </button>
              <button
                className="btn btn-sm"
                onClick={() => setUpdateInfo(null)}
                disabled={updating}
              >
                稍后
              </button>
            </div>
          </div>
        )}

        {page === "dash" && <Dashboard />}
        {page === "report" && <Report />}
        {page === "settings" && <Settings />}
      </main>
    </div>
  );
}
