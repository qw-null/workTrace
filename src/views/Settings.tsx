import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { api, BACKUP_CHANGED_EVENT, CHECK_UPDATE_EVENT, UPDATE_STATUS_EVENT } from "../api";
import type { UpdateStatus } from "../api";
import type { BackupSettings, ModelConfig, WebdavConfig } from "../types";

type Tab = "model" | "backup" | "about";

const ROLES: { value: string; label: string }[] = [
  { value: "record", label: "记录转化" },
  { value: "report", label: "周报生成" },
  { value: "vision", label: "图片 / 流程图识别" },
];

const PROVIDERS: { value: string; label: string; url: string }[] = [
  { value: "jianguoyun", label: "坚果云（默认）", url: "https://dav.jianguoyun.com/dav/" },
  { value: "infinicloud", label: "InfiniCLOUD", url: "https://dav.infini-cloud.net/dav/" },
  { value: "other", label: "其他 WebDAV", url: "" },
];

const emptyModel = (): ModelConfig => ({
  id: "",
  name: "",
  baseUrl: "",
  model: "",
  apiKey: "",
  role: "record",
  isDefault: false,
});

const emptyWebdav = (): WebdavConfig => ({
  id: "",
  name: "",
  provider: "jianguoyun",
  url: "https://dav.jianguoyun.com/dav/",
  account: "",
  password: "",
  isDefault: false,
});

export default function Settings() {
  const [tab, setTab] = useState<Tab>("model");
  const [version, setVersion] = useState("");

  // 检查更新状态
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus | null>(null);

  useEffect(() => {
    getVersion()
      .then((v) => setVersion(v))
      .catch(() => setVersion(""));
  }, []);

  useEffect(() => {
    const onStatus = (e: Event) => {
      const detail = (e as CustomEvent<UpdateStatus>).detail;
      setUpdateStatus(detail);
    };
    window.addEventListener(UPDATE_STATUS_EVENT, onStatus);
    return () => window.removeEventListener(UPDATE_STATUS_EVENT, onStatus);
  }, []);

  // 大模型
  const [models, setModels] = useState<ModelConfig[]>([]);
  const [showForm, setShowForm] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [form, setForm] = useState<ModelConfig>(emptyModel());
  const [msg, setMsg] = useState("");

  // 备份
  const [waccounts, setWaccounts] = useState<WebdavConfig[]>([]);
  const [backupSettings, setBackupSettings] = useState<BackupSettings>({
    syncIntervalMin: 5,
    attachmentRetentionDays: 7,
  });
  const [showWForm, setShowWForm] = useState(false);
  const [editingWId, setEditingWId] = useState<string | null>(null);
  const [wform, setWform] = useState<WebdavConfig>(emptyWebdav());
  const [syncMsg, setSyncMsg] = useState("");
  const [syncing, setSyncing] = useState(false);

  const [testingId, setTestingId] = useState<string | null>(null);
  const [testResult, setTestResult] = useState("");
  const [testOk, setTestOk] = useState(false);

  const [wTestingId, setWTestingId] = useState<string | null>(null);
  const [wTestResult, setWTestResult] = useState("");
  const [wTestOk, setWTestOk] = useState(false);

  const reload = async () => {
    try {
      setModels(await api.listModels());
    } catch {
      /* 浏览器预览环境无后端 */
    }
  };

  const reloadWebdav = async () => {
    try {
      setWaccounts(await api.listWebdavConfigs());
    } catch {
      /* 浏览器预览环境无后端 */
    }
  };

  useEffect(() => {
    reload();
    reloadWebdav();
    api
      .getBackupSettings()
      .then((bs) => {
        if (bs) setBackupSettings(bs);
      })
      .catch(() => {});
  }, []);

  // ===== 大模型 =====
  const openAdd = () => {
    setForm(emptyModel());
    setEditingId(null);
    setShowForm(true);
  };

  const openEdit = (m: ModelConfig) => {
    setForm(m);
    setEditingId(m.id);
    setShowForm(true);
  };

  const save = async () => {
    if (!form.name.trim() || !form.baseUrl.trim() || !form.model.trim()) {
      setMsg("请填写名称、Base URL 和模型名");
      return;
    }
    const m: ModelConfig = { ...form };
    try {
      await api.saveModel(m);
      setMsg("已保存");
      setShowForm(false);
      await reload();
    } catch (e) {
      setMsg(String(e));
    }
  };

  const remove = async (id: string) => {
    try {
      await api.deleteModel(id);
      setMsg("已删除");
      await reload();
    } catch (e) {
      setMsg(String(e));
    }
  };

  const set = (k: keyof ModelConfig, v: string | boolean) => setForm((f) => ({ ...f, [k]: v }));

  const testModel = async (m: ModelConfig) => {
    const key = m.id || "form";
    setTestingId(key);
    setTestResult("");
    try {
      setTestResult(await api.testModel(m));
      setTestOk(true);
    } catch (e) {
      setTestResult(String(e));
      setTestOk(false);
    } finally {
      setTestingId(null);
    }
  };

  // ===== 备份 =====
  const openAddW = () => {
    setWform(emptyWebdav());
    setEditingWId(null);
    setShowWForm(true);
  };

  const openEditW = (w: WebdavConfig) => {
    setWform(w);
    setEditingWId(w.id);
    setShowWForm(true);
  };

  const saveW = async () => {
    if (!wform.name.trim() || !wform.url.trim() || !wform.account.trim()) {
      setSyncMsg("请填写名称、WebDAV 地址和账号");
      return;
    }
    try {
      await api.saveWebdavConfig(wform);
      setSyncMsg("已保存");
      setShowWForm(false);
      await reloadWebdav();
      window.dispatchEvent(new Event(BACKUP_CHANGED_EVENT));
    } catch (e) {
      setSyncMsg(String(e));
    }
  };

  const removeW = async (id: string) => {
    try {
      await api.deleteWebdavConfig(id);
      setSyncMsg("已删除");
      await reloadWebdav();
      window.dispatchEvent(new Event(BACKUP_CHANGED_EVENT));
    } catch (e) {
      setSyncMsg(String(e));
    }
  };

  const setDefaultW = async (w: WebdavConfig) => {
    try {
      await api.saveWebdavConfig({ ...w, isDefault: true });
      setSyncMsg("已切换为当前备份账号");
      await reloadWebdav();
      window.dispatchEvent(new Event(BACKUP_CHANGED_EVENT));
    } catch (e) {
      setSyncMsg(String(e));
    }
  };

  const testWebdav = async (w: WebdavConfig) => {
    const key = w.id || "wform";
    setWTestingId(key);
    setWTestResult("");
    try {
      setWTestResult(await api.testWebdav(w));
      setWTestOk(true);
    } catch (e) {
      setWTestResult(String(e));
      setWTestOk(false);
    } finally {
      setWTestingId(null);
    }
  };

  const saveBackup = async () => {
    try {
      await api.saveBackupSettings(backupSettings);
      setSyncMsg("同步设置已保存");
    } catch (e) {
      setSyncMsg(String(e));
    }
  };

  const doSync = async () => {
    setSyncing(true);
    setSyncMsg("");
    try {
      setSyncMsg(await api.syncNow());
    } catch (e) {
      setSyncMsg(String(e));
    } finally {
      setSyncing(false);
    }
  };

  const setW = (k: keyof WebdavConfig, v: string | boolean) => setWform((f) => ({ ...f, [k]: v }));

  return (
    <div className="settings">
      <div className="set-tabs">
        <button className={"set-tab" + (tab === "model" ? " active" : "")} onClick={() => setTab("model")}>
          大模型
        </button>
        <button className={"set-tab" + (tab === "backup" ? " active" : "")} onClick={() => setTab("backup")}>
          备份
        </button>
        <button className={"set-tab" + (tab === "about" ? " active" : "")} onClick={() => setTab("about")}>
          关于
        </button>
      </div>

      {tab === "model" && (
        <div className="set-panel">
          <div className="card">
            <h3>大模型配置</h3>
            <div className="sub">支持 OpenAI 兼容协议与本地 Ollama，可配置多个模型并指定用途</div>

            {models.length === 0 && !showForm && (
              <div className="empty">还没有配置模型，点击下方「添加模型」开始</div>
            )}

            {models.map((m) => (
              <div key={m.id} className="model-card">
                <div className="m-ico">{m.name.slice(0, 2)}</div>
                <div className="m-info">
                  <div className="m-name">{m.name}</div>
                  <div className="m-sub">
                    {m.model} · {m.baseUrl} · {ROLES.find((r) => r.value === m.role)?.label || m.role}
                  </div>
                </div>
                {m.isDefault && <span className="badge green">默认</span>}
                <button className="btn btn-sm" onClick={() => testModel(m)} disabled={testingId === m.id}>
                  {testingId === m.id ? "测试中…" : "测试"}
                </button>
                <button className="btn btn-sm" onClick={() => openEdit(m)}>编辑</button>
                <button className="btn btn-sm" onClick={() => remove(m.id)}>删除</button>
              </div>
            ))}

            {showForm && (
              <div style={{ marginTop: 8 }}>
                <hr className="divider" />
                <div className="form-grid">
                  <div className="field">
                    <label>显示名称</label>
                    <input value={form.name} onChange={(e) => set("name", e.target.value)} placeholder="例如：DeepSeek-V3" />
                  </div>
                  <div className="field-row">
                    <div className="field">
                      <label>Base URL</label>
                      <input value={form.baseUrl} onChange={(e) => set("baseUrl", e.target.value)} placeholder="https://api.deepseek.com/v1" />
                    </div>
                    <div className="field">
                      <label>模型名</label>
                      <input value={form.model} onChange={(e) => set("model", e.target.value)} placeholder="deepseek-chat" />
                    </div>
                  </div>
                  <div className="field">
                    <label>API Key（存于 macOS Keychain）</label>
                    <input type="password" value={form.apiKey} onChange={(e) => set("apiKey", e.target.value)} placeholder="留空则不修改" />
                  </div>
                  <div className="field-row">
                    <div className="field">
                      <label>默认用途</label>
                      <select value={form.role} onChange={(e) => set("role", e.target.value)}>
                        {ROLES.map((r) => (
                          <option key={r.value} value={r.value}>{r.label}</option>
                        ))}
                      </select>
                    </div>
                    <div className="field">
                      <label>设为默认</label>
                      <select value={form.isDefault ? "1" : "0"} onChange={(e) => set("isDefault", e.target.value === "1")}>
                        <option value="0">否</option>
                        <option value="1">是</option>
                      </select>
                    </div>
                  </div>
                </div>
                <div style={{ display: "flex", gap: 10 }}>
                  <button className="btn btn-primary" onClick={save}>{editingId ? "保存修改" : "添加模型"}</button>
                  <button className="btn" onClick={() => testModel(form)} disabled={testingId === "form"}>
                    {testingId === "form" ? "测试中…" : "测试连通性"}
                  </button>
                  <button className="btn" onClick={() => setShowForm(false)}>取消</button>
                </div>
              </div>
            )}

            {!showForm && (
              <button className="btn btn-primary" style={{ margin: "8px 0 4px" }} onClick={openAdd}>
                + 添加模型
              </button>
            )}
            {testResult && (
              <div className="status-line" style={{ marginTop: 12 }}>
                <span className="sync-dot" style={{ background: testOk ? "var(--green)" : "var(--red)" }} />
                <span style={{ color: testOk ? undefined : "var(--red)" }}>{testResult}</span>
              </div>
            )}
            {msg && <div className="status-line" style={{ marginTop: 12 }}>{msg}</div>}
          </div>
        </div>
      )}

      {tab === "backup" && (
        <div className="set-panel">
          <div className="card">
            <h3>备份与同步（WebDAV）</h3>
            <div className="sub">可配置多个备份账号，支持连通性测试与切换；云端仅备份结构化记录与周报</div>

            {waccounts.length === 0 && !showWForm && (
              <div className="empty">还没有备份账号，点击下方「添加备份账号」开始</div>
            )}

            {waccounts.map((w) => (
              <div key={w.id} className="model-card">
                <div className="m-ico">{w.name.slice(0, 2) || "备"}</div>
                <div className="m-info">
                  <div className="m-name">{w.name}</div>
                  <div className="m-sub">{w.account} · {w.url}</div>
                </div>
                {w.isDefault && <span className="badge green">当前使用</span>}
                <button className="btn btn-sm" onClick={() => testWebdav(w)} disabled={wTestingId === w.id}>
                  {wTestingId === w.id ? "测试中…" : "测试"}
                </button>
                {!w.isDefault && (
                  <button className="btn btn-sm" onClick={() => setDefaultW(w)}>设为默认</button>
                )}
                <button className="btn btn-sm" onClick={() => openEditW(w)}>编辑</button>
                <button className="btn btn-sm" onClick={() => removeW(w.id)}>删除</button>
              </div>
            ))}

            {showWForm && (
              <div style={{ marginTop: 8 }}>
                <hr className="divider" />
                <div className="form-grid">
                  <div className="field">
                    <label>显示名称</label>
                    <input value={wform.name} onChange={(e) => setW("name", e.target.value)} placeholder="例如：坚果云" />
                  </div>
                  <div className="field">
                    <label>服务商</label>
                    <select
                      value={wform.provider}
                      onChange={(e) => {
                        const v = e.target.value;
                        const p = PROVIDERS.find((x) => x.value === v);
                        setWform({ ...wform, provider: v, url: p?.url ?? "" });
                      }}
                    >
                      {PROVIDERS.map((p) => (
                        <option key={p.value} value={p.value}>{p.label}</option>
                      ))}
                    </select>
                    {wform.provider === "infinicloud" && (
                      <div className="weak" style={{ marginTop: 6, fontSize: 12 }}>
                        InfiniCLOUD 的连接地址因账号而异，请到官网「My Page → Apps Connection」复制你的专属地址替换默认值。
                      </div>
                    )}
                  </div>
                  <div className="field">
                    <label>WebDAV 地址</label>
                    <input value={wform.url} onChange={(e) => setW("url", e.target.value)} placeholder="https://dav.jianguoyun.com/dav/" />
                  </div>
                  <div className="field-row">
                    <div className="field">
                      <label>账号</label>
                      <input value={wform.account} onChange={(e) => setW("account", e.target.value)} placeholder="邮箱" />
                    </div>
                    <div className="field">
                      <label>应用密码（存于 Keychain）</label>
                      <input type="password" value={wform.password} onChange={(e) => setW("password", e.target.value)} placeholder="留空则不修改" />
                    </div>
                  </div>
                  <div className="field">
                    <label>设为默认</label>
                    <select value={wform.isDefault ? "1" : "0"} onChange={(e) => setW("isDefault", e.target.value === "1")}>
                      <option value="0">否</option>
                      <option value="1">是（切换为当前使用）</option>
                    </select>
                  </div>
                </div>
                <div style={{ display: "flex", gap: 10 }}>
                  <button className="btn btn-primary" onClick={saveW}>{editingWId ? "保存修改" : "添加账号"}</button>
                  <button className="btn" onClick={() => testWebdav(wform)} disabled={wTestingId === "wform"}>
                    {wTestingId === "wform" ? "测试中…" : "测试连通性"}
                  </button>
                  <button className="btn" onClick={() => setShowWForm(false)}>取消</button>
                </div>
              </div>
            )}

            {!showWForm && (
              <button className="btn btn-primary" style={{ margin: "8px 0 4px" }} onClick={openAddW}>
                + 添加备份账号
              </button>
            )}

            {wTestResult && (
              <div className="status-line" style={{ marginTop: 12 }}>
                <span className="sync-dot" style={{ background: wTestOk ? "var(--green)" : "var(--red)" }} />
                <span style={{ color: wTestOk ? undefined : "var(--red)" }}>{wTestResult}</span>
              </div>
            )}

            <hr className="divider" />

            <h3>同步设置</h3>
            <div className="sub">自动同步频率与附件临时副本清理策略（对所有备份账号生效）</div>
            <div className="form-grid" style={{ marginTop: 8 }}>
              <div className="field-row">
                <div className="field">
                  <label>自动同步频率</label>
                  <select
                    value={String(backupSettings.syncIntervalMin)}
                    onChange={(e) => setBackupSettings({ ...backupSettings, syncIntervalMin: Number(e.target.value) })}
                  >
                    <option value="5">每 5 分钟</option>
                    <option value="15">每 15 分钟</option>
                    <option value="60">每小时</option>
                    <option value="0">仅手动</option>
                  </select>
                </div>
                <div className="field">
                  <label>附件临时副本</label>
                  <select
                    value={String(backupSettings.attachmentRetentionDays)}
                    onChange={(e) => setBackupSettings({ ...backupSettings, attachmentRetentionDays: Number(e.target.value) })}
                  >
                    <option value="7">保留 7 天后自动清理</option>
                    <option value="30">保留 30 天</option>
                    <option value="0">处理完立即清理</option>
                  </select>
                </div>
              </div>
            </div>
            <div style={{ display: "flex", gap: 10, marginTop: 12 }}>
              <button className="btn btn-primary" onClick={doSync} disabled={syncing}>
                {syncing ? "同步中…" : "立即同步"}
              </button>
              <button className="btn" onClick={saveBackup}>保存同步设置</button>
            </div>
            {syncMsg && <div className="status-line" style={{ marginTop: 12 }}>{syncMsg}</div>}
          </div>
        </div>
      )}

      {tab === "about" && (
        <div className="set-panel">
          <div className="card">
            <h3>关于</h3>
            <div className="sub">工作日迹 WorkTrace · 本地优先的 AI 工作日志与周报工具</div>
            <div style={{ marginTop: 14 }}>
              <div className="weak">当前版本：{version ? `v${version}` : "—"}</div>
              <div style={{ display: "flex", gap: 10, marginTop: 14, alignItems: "center" }}>
                <button
                  className="btn btn-primary"
                  onClick={() => window.dispatchEvent(new Event(CHECK_UPDATE_EVENT))}
                  disabled={updateStatus?.status === "checking"}
                >
                  {updateStatus?.status === "checking" ? "检查中…" : "检查更新"}
                </button>
                {updateStatus && (
                  <span
                    className="weak"
                    style={{
                      fontSize: 13,
                      color:
                        updateStatus.status === "available"
                          ? "var(--green)"
                          : updateStatus.status === "error"
                          ? "var(--red)"
                          : undefined,
                    }}
                  >
                    {updateStatus.status === "checking" && "正在检查最新版本…"}
                    {updateStatus.status === "uptodate" && "已是最新版本"}
                    {updateStatus.status === "available" &&
                      `发现新版本 v${updateStatus.version}，可在顶部横幅点击更新`}
                    {updateStatus.status === "error" && updateStatus.message}
                  </span>
                )}
              </div>
              <div className="weak" style={{ marginTop: 14, fontSize: 12, lineHeight: 1.8 }}>
                启动时会自动检查更新，也可在此手动检查；发现新版本后确认即可自动下载安装并重启。
                <br />
                项目仓库：github.com/qw-null/workTrace
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
