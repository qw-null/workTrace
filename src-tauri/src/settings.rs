use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

/// 大模型配置（api_key 存 Keychain，JSON 中不落明文）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelConfig {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub api_key: String,
    pub role: String,
    pub is_default: bool,
}

/// WebDAV 备份账号（password 存 Keychain，JSON 中不落明文）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebdavConfig {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub url: String,
    pub account: String,
    pub password: String,
    pub is_default: bool,
}

/// 全局备份设置（自动同步频率、附件临时副本保留天数）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSettings {
    pub sync_interval_min: u32,
    pub attachment_retention_days: u32,
}

/// 旧版单账号 WebDAV 配置（仅用于数据迁移）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyWebdavConfig {
    provider: String,
    url: String,
    account: String,
    password: String,
    sync_interval_min: u32,
    attachment_retention_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct AppSettings {
    #[serde(default)]
    models: Vec<ModelConfig>,
    #[serde(default)]
    webdav_accounts: Vec<WebdavConfig>,
    #[serde(default)]
    backup: Option<BackupSettings>,
    /// 旧版字段，仅反序列化兼容用，加载后即迁移清空
    #[serde(default)]
    webdav: Option<LegacyWebdavConfig>,
}

/// 密码本地文件存储（不用钥匙串，避免未签名应用反复弹授权框）
fn secrets_path() -> Result<PathBuf, String> {
    Ok(crate::data_dir()?.join("settings").join("secrets.json"))
}

fn load_secrets() -> HashMap<String, String> {
    secrets_path()
        .ok()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_secrets(map: &HashMap<String, String>) -> Result<(), String> {
    let path = secrets_path()?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(map).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

/// 密码内存缓存：进程内缓存，避免每次同步/测试都读文件
static SECRET_CACHE: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

fn cache_get(key: &str) -> Option<String> {
    SECRET_CACHE
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref()?.get(key).cloned())
}

fn cache_set(key: &str, value: &str) {
    if let Ok(mut guard) = SECRET_CACHE.lock() {
        guard
            .get_or_insert_with(HashMap::new)
            .insert(key.to_string(), value.to_string());
    }
}

fn cache_remove(key: &str) {
    if let Ok(mut guard) = SECRET_CACHE.lock() {
        if let Some(map) = guard.as_mut() {
            map.remove(key);
        }
    }
}

fn set_secret(key: &str, value: &str) -> Result<(), String> {
    let mut map = load_secrets();
    map.insert(key.to_string(), value.to_string());
    save_secrets(&map)?;
    cache_set(key, value);
    Ok(())
}

fn get_secret(key: &str) -> Option<String> {
    // 先查内存缓存，命中则不再读文件
    if let Some(v) = cache_get(key) {
        return Some(v);
    }
    let val = load_secrets().get(key).cloned();
    if let Some(v) = &val {
        cache_set(key, v);
    }
    val
}

fn delete_secret(key: &str) {
    let mut map = load_secrets();
    map.remove(key);
    let _ = save_secrets(&map);
    cache_remove(key);
}

fn settings_path() -> Result<PathBuf, String> {
    Ok(crate::data_dir()?.join("settings").join("app.json"))
}

fn default_backup_settings() -> BackupSettings {
    BackupSettings {
        sync_interval_min: 5,
        attachment_retention_days: 7,
    }
}

fn load() -> AppSettings {
    let mut s: AppSettings = settings_path()
        .ok()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();

    // 迁移旧版单账号配置（webdav 字段）→ 多账号 + 全局设置
    if let Some(legacy) = s.webdav.take() {
        if s.webdav_accounts.is_empty() {
            let mut password = legacy.password;
            if password.is_empty() {
                password = get_secret("webdav").unwrap_or_default();
            }
            s.webdav_accounts.push(WebdavConfig {
                id: uuid::Uuid::new_v4().to_string(),
                name: default_name_for_provider(&legacy.provider),
                provider: legacy.provider,
                url: legacy.url,
                account: legacy.account,
                password,
                is_default: true,
            });
        }
        if s.backup.is_none() {
            s.backup = Some(BackupSettings {
                sync_interval_min: legacy.sync_interval_min,
                attachment_retention_days: legacy.attachment_retention_days,
            });
        }
        let _ = save(&s);
    }
    s
}

fn save(s: &AppSettings) -> Result<(), String> {
    let path = settings_path()?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

fn default_name_for_provider(provider: &str) -> String {
    match provider {
        "jianguoyun" => "坚果云".to_string(),
        "infinicloud" => "InfiniCLOUD".to_string(),
        "other" => "WebDAV".to_string(),
        other => other.to_string(),
    }
}

/// 内部查询单个模型配置（含从 Keychain 读取的 api_key）
pub fn get_model(id: &str) -> Result<ModelConfig, String> {
    let mut m = load()
        .models
        .into_iter()
        .find(|m| m.id == id)
        .ok_or_else(|| format!("模型 {} 不存在", id))?;
    if m.api_key.is_empty() {
        if let Some(key) = get_secret(&format!("model:{id}")) {
            m.api_key = key;
        }
    }
    Ok(m)
}

/// 回填账号密码（密码为空时从 Keychain 读取）
pub(crate) fn fill_webdav_password(cfg: &mut WebdavConfig) {
    if cfg.password.is_empty() && !cfg.id.is_empty() {
        if let Some(pw) = get_secret(&format!("webdav:{}", cfg.id)) {
            cfg.password = pw;
        }
    }
}

/// 内部读取当前激活的备份账号（含从 Keychain 读取的密码）
pub(crate) fn get_webdav_config_internal() -> Result<Option<WebdavConfig>, String> {
    let mut accounts = load().webdav_accounts;
    if accounts.is_empty() {
        return Ok(None);
    }
    let idx = accounts.iter().position(|a| a.is_default).unwrap_or(0);
    let mut cfg = accounts.remove(idx);
    fill_webdav_password(&mut cfg);
    Ok(Some(cfg))
}

/// 读取自动同步间隔（分钟），未配置或仅手动返回 None
pub(crate) fn get_sync_interval_minutes() -> Option<u32> {
    load().backup.as_ref().map(|b| b.sync_interval_min)
}

#[tauri::command]
pub fn list_models() -> Result<Vec<ModelConfig>, String> {
    Ok(load().models)
}

#[tauri::command]
pub fn save_model(model: ModelConfig) -> Result<(), String> {
    let mut m = model;
    if m.id.is_empty() {
        m.id = uuid::Uuid::new_v4().to_string();
    }
    // 尝试存 Keychain；失败则降级为明文存 JSON（仍保证功能可用）
    if !m.api_key.is_empty() {
        if set_secret(&format!("model:{}", m.id), &m.api_key).is_ok() {
            m.api_key = String::new();
        }
    }
    let mut s = load();
    if let Some(existing) = s.models.iter_mut().find(|x| x.id == m.id) {
        *existing = m;
    } else {
        s.models.push(m);
    }
    save(&s)
}

#[tauri::command]
pub fn delete_model(id: String) -> Result<(), String> {
    delete_secret(&format!("model:{id}"));
    let mut s = load();
    s.models.retain(|m| m.id != id);
    save(&s)
}

#[tauri::command]
pub fn list_webdav_configs() -> Result<Vec<WebdavConfig>, String> {
    Ok(load().webdav_accounts)
}

#[tauri::command]
pub fn save_webdav_config(cfg: WebdavConfig) -> Result<(), String> {
    let mut c = cfg;
    if c.id.is_empty() {
        c.id = uuid::Uuid::new_v4().to_string();
    }
    // 尝试存 Keychain；失败则降级为明文存 JSON
    if !c.password.is_empty() {
        if set_secret(&format!("webdav:{}", c.id), &c.password).is_ok() {
            c.password = String::new();
        }
    }
    let mut s = load();
    // 设为默认时清掉其它账号的默认标记
    if c.is_default {
        for a in s.webdav_accounts.iter_mut() {
            a.is_default = false;
        }
    }
    if let Some(existing) = s.webdav_accounts.iter_mut().find(|x| x.id == c.id) {
        // 编辑时密码留空：保留旧密码（可能明文或 Keychain）
        if c.password.is_empty() && !existing.password.is_empty() {
            c.password = existing.password.clone();
        }
        *existing = c;
    } else {
        s.webdav_accounts.push(c);
    }
    // 保证始终存在一个默认账号
    if !s.webdav_accounts.iter().any(|a| a.is_default) {
        if let Some(first) = s.webdav_accounts.first_mut() {
            first.is_default = true;
        }
    }
    save(&s)
}

#[tauri::command]
pub fn delete_webdav_config(id: String) -> Result<(), String> {
    delete_secret(&format!("webdav:{id}"));
    let mut s = load();
    s.webdav_accounts.retain(|a| a.id != id);
    // 若删掉的是默认账号，且还有其它账号，则把第一个设为默认
    if !s.webdav_accounts.is_empty() && !s.webdav_accounts.iter().any(|a| a.is_default) {
        if let Some(first) = s.webdav_accounts.first_mut() {
            first.is_default = true;
        }
    }
    save(&s)
}

#[tauri::command]
pub fn get_backup_settings() -> Result<BackupSettings, String> {
    Ok(load().backup.unwrap_or_else(default_backup_settings))
}

#[tauri::command]
pub fn save_backup_settings(bs: BackupSettings) -> Result<(), String> {
    let mut s = load();
    s.backup = Some(bs);
    save(&s)
}
