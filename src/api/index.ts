import { invoke } from "@tauri-apps/api/core";
import { check } from "@tauri-apps/plugin-updater";
import type { BackupSettings, DayActive, DayRecord, ModelConfig, Report, Structured, WebdavConfig } from "../types";

export interface UpdateInfo {
  version: string;
  notes: string;
}

// 统一封装 Tauri 后端命令；在浏览器（vite dev）环境下后端不可用时返回 mock。
export const api = {
  async listModels(): Promise<ModelConfig[]> {
    return invoke<ModelConfig[]>("list_models");
  },
  async saveModel(model: ModelConfig): Promise<void> {
    return invoke("save_model", { model });
  },
  async deleteModel(id: string): Promise<void> {
    return invoke("delete_model", { id });
  },
  async transformRecord(input: string, modelId: string): Promise<Structured> {
    return invoke<Structured>("transform_record", { input, modelId });
  },
  async testModel(model: ModelConfig): Promise<string> {
    return invoke<string>("test_model", { model });
  },
  async parseAttachment(
    filename: string,
    base64Data: string,
    modelId: string | null
  ): Promise<string> {
    return invoke<string>("parse_attachment", { filename, base64Data, modelId });
  },
  async getDayRecord(date: string): Promise<DayRecord | null> {
    return invoke<DayRecord | null>("get_day_record", { date });
  },
  async saveDayRecord(record: DayRecord): Promise<void> {
    return invoke("save_day_record", { record });
  },
  async confirmRecord(
    date: string,
    rawText: string,
    modelId: string,
    structured: Structured
  ): Promise<void> {
    return invoke("confirm_record", { date, rawText, modelId, structured });
  },
  async confirmTodo(date: string, text: string): Promise<void> {
    return invoke("confirm_todo", { date, text });
  },
  async deleteEntry(date: string, entryId: string): Promise<void> {
    return invoke("delete_entry", { date, entryId });
  },
  async updateEntry(date: string, entryId: string, structured: Structured): Promise<void> {
    return invoke("update_entry", { date, entryId, structured });
  },
  async getMonthActive(month: string): Promise<DayActive[]> {
    return invoke<DayActive[]>("get_month_active", { month });
  },
  async getYearActive(year: string): Promise<DayActive[]> {
    return invoke<DayActive[]>("get_year_active", { year });
  },
  async generateReport(weekStart: string, modelId: string): Promise<Report> {
    return invoke<Report>("generate_report", { weekStart, modelId });
  },
  async getReport(weekStart: string): Promise<Report | null> {
    return invoke<Report | null>("get_report", { weekStart });
  },
  async exportReport(content: string, weekStart: string, format: string): Promise<string> {
    return invoke<string>("export_report", { content, weekStart, format });
  },
  async listWebdavConfigs(): Promise<WebdavConfig[]> {
    return invoke<WebdavConfig[]>("list_webdav_configs");
  },
  async saveWebdavConfig(cfg: WebdavConfig): Promise<void> {
    return invoke("save_webdav_config", { cfg });
  },
  async deleteWebdavConfig(id: string): Promise<void> {
    return invoke("delete_webdav_config", { id });
  },
  async testWebdav(cfg: WebdavConfig): Promise<string> {
    return invoke<string>("test_webdav", { cfg });
  },
  async getBackupSettings(): Promise<BackupSettings> {
    return invoke<BackupSettings>("get_backup_settings");
  },
  async saveBackupSettings(bs: BackupSettings): Promise<void> {
    return invoke("save_backup_settings", { bs });
  },
  async syncNow(): Promise<string> {
    return invoke("sync_now");
  },
  async checkUpdate(): Promise<UpdateInfo | null> {
    const update = await check();
    return update ? { version: update.version, notes: update.body || "" } : null;
  },
  async installUpdate(onProgress?: (percent: number) => void): Promise<void> {
    const update = await check();
    if (!update) return;
    let total = 0;
    let downloaded = 0;
    await update.downloadAndInstall((event) => {
      if (event.event === "Started") {
        total = event.data.contentLength || 0;
      } else if (event.event === "Progress") {
        downloaded += event.data.chunkLength;
        if (onProgress && total > 0) {
          onProgress(Math.min(99, Math.round((downloaded / total) * 100)));
        }
      }
    });
    await invoke("restart_app");
  },
};

// 备份账号变更事件名：设置页切换默认账号后触发，侧边栏等监听刷新
export const BACKUP_CHANGED_EVENT = "worktrace:backup-changed";

// 记录变更事件名：确认入库后触发，日历/热力图等监听刷新
export const RECORD_CHANGED_EVENT = "worktrace:record-changed";

// 检查更新事件名：设置页「检查更新」按钮触发，App 监听执行
export const CHECK_UPDATE_EVENT = "worktrace:check-update";
