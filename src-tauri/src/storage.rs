use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// 结构化记录中的流程图
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flowchart {
    pub title: String,
    pub mermaid: String,
}

/// 一条工作记录（记录模式，日程秘书字段）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RecordField {
    #[serde(default)]
    pub time: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub progress: String,
    #[serde(default)]
    pub people: String,
    #[serde(default)]
    pub next: String,
}

/// 一条待办事项（待办模式，日程秘书字段）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TodoField {
    #[serde(default)]
    pub time_location: String,
    #[serde(default)]
    pub item: String,
    #[serde(default)]
    pub note: String,
}

/// AI 转化后的结构化字段（旧字段保留用于历史数据兼容）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Structured {
    // —— 旧字段（历史数据，向后兼容）——
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub tasks: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub outputs: Vec<String>,
    #[serde(default)]
    pub flowcharts: Vec<Flowchart>,
    #[serde(default)]
    pub todos: Vec<String>,
    // —— 新字段（新提示词产出）——
    #[serde(default)]
    pub records: Vec<RecordField>,
    #[serde(default)]
    pub todo_items: Vec<TodoField>,
}

fn default_kind() -> String {
    "record".to_string()
}

/// 单条记录（工作记录或待办，kind 区分）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordEntry {
    pub id: String,
    /// "record" = 工作记录，"todo" = 待办
    #[serde(default = "default_kind")]
    pub kind: String,
    pub created_at: String,
    pub updated_at: String,
    pub raw_text: String,
    pub source_attachments: Vec<String>,
    pub model_used: String,
    pub status: String,
    pub structured: Structured,
}

/// 每日记录文件内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayRecord {
    pub version: u32,
    pub date: String,
    pub entries: Vec<RecordEntry>,
}

/// 某天的记录条数（供日历/热力图使用）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DayActive {
    pub date: String,
    pub count: usize,
    /// 待办条数（kind = todo）
    #[serde(default)]
    pub todo_count: usize,
}

fn record_path(date: &str) -> Result<PathBuf, String> {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return Err("日期格式应为 YYYY-MM-DD".to_string());
    }
    Ok(crate::data_dir()?
        .join("records")
        .join(parts[0])
        .join(parts[1])
        .join(format!("{}.json", date)))
}

pub(crate) fn read_day(date: &str) -> Result<Option<DayRecord>, String> {
    let path = record_path(date)?;
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let rec: DayRecord = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    Ok(Some(rec))
}

fn write_day(record: &DayRecord) -> Result<(), String> {
    let path = record_path(&record.date)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(record).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_day_record(date: String) -> Result<Option<DayRecord>, String> {
    read_day(&date)
}

#[tauri::command]
pub fn save_day_record(record: DayRecord) -> Result<(), String> {
    write_day(&record)
}

/// 把 AI 转化出的多条工作记录确认入库（每条 record 存为新字段 records）
#[tauri::command]
pub fn confirm_record(
    date: String,
    raw_text: String,
    model_id: String,
    records: Vec<RecordField>,
) -> Result<(), String> {
    let now = chrono::Local::now().to_rfc3339();
    let entry = RecordEntry {
        id: uuid::Uuid::new_v4().to_string(),
        kind: "record".to_string(),
        created_at: now.clone(),
        updated_at: now,
        raw_text,
        source_attachments: vec![],
        model_used: model_id,
        status: "confirmed".to_string(),
        structured: Structured {
            records,
            ..Default::default()
        },
    };
    let mut day = read_day(&date)?.unwrap_or(DayRecord {
        version: 1,
        date: date.clone(),
        entries: vec![],
    });
    day.entries.push(entry);
    write_day(&day)
}

/// 添加待办：把 AI 转化出的多条待办入库；若为空则兜底按行拆分
#[tauri::command]
pub fn confirm_todo(
    date: String,
    raw_text: String,
    model_id: String,
    todo_items: Vec<TodoField>,
) -> Result<(), String> {
    let mut items = todo_items;
    // 兜底：模型未提取出待办项时，按行拆分原文
    if items.is_empty() {
        let lines: Vec<String> = raw_text
            .split('\n')
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect();
        if lines.is_empty() {
            return Err("待办内容不能为空".to_string());
        }
        items = lines
            .into_iter()
            .map(|s| TodoField {
                item: s,
                ..Default::default()
            })
            .collect();
    }
    let now = chrono::Local::now().to_rfc3339();
    let entry = RecordEntry {
        id: uuid::Uuid::new_v4().to_string(),
        kind: "todo".to_string(),
        created_at: now.clone(),
        updated_at: now,
        raw_text,
        source_attachments: vec![],
        model_used: model_id,
        status: "confirmed".to_string(),
        structured: Structured {
            todo_items: items,
            ..Default::default()
        },
    };
    let mut day = read_day(&date)?.unwrap_or(DayRecord {
        version: 1,
        date: date.clone(),
        entries: vec![],
    });
    day.entries.push(entry);
    write_day(&day)
}

/// 删除某天的某条记录
#[tauri::command]
pub fn delete_entry(date: String, entry_id: String) -> Result<(), String> {
    let mut day = read_day(&date)?.ok_or("该日期没有记录")?;
    let before = day.entries.len();
    day.entries.retain(|e| e.id != entry_id);
    if day.entries.len() == before {
        return Err("未找到该记录".to_string());
    }
    write_day(&day)
}

/// 更新某天的某条记录的结构化内容（保留原流程图）
#[tauri::command]
pub fn update_entry(date: String, entry_id: String, structured: Structured) -> Result<(), String> {
    let mut day = read_day(&date)?.ok_or("该日期没有记录")?;
    let entry = day
        .entries
        .iter_mut()
        .find(|e| e.id == entry_id)
        .ok_or("未找到该记录")?;
    entry.structured = structured;
    entry.updated_at = chrono::Local::now().to_rfc3339();
    write_day(&day)
}

/// 统计记录数和待办数（kind = todo 计为待办，其余计为记录）
fn count_kinds(entries: &[RecordEntry]) -> (usize, usize) {
    entries
        .iter()
        .fold((0, 0), |(r, t), e| if e.kind == "todo" { (r, t + 1) } else { (r + 1, t) })
}

/// 返回某月每天的记录条数（month 形如 "2026-08"）
#[tauri::command]
pub fn get_month_active(month: String) -> Result<Vec<DayActive>, String> {
    let parts: Vec<&str> = month.split('-').collect();
    if parts.len() != 2 {
        return Err("月份格式应为 YYYY-MM".to_string());
    }
    let dir = crate::data_dir()?.join("records").join(parts[0]).join(parts[1]);
    let mut result: Vec<DayActive> = vec![];
    if dir.exists() {
        let mut files: Vec<PathBuf> = fs::read_dir(&dir)
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("json"))
            .collect();
        files.sort();
        for f in files {
            if let Ok(content) = fs::read_to_string(&f) {
                if let Ok(day) = serde_json::from_str::<DayRecord>(&content) {
                    let (count, todo_count) =
                        count_kinds(&day.entries);
                    result.push(DayActive {
                        date: day.date,
                        count,
                        todo_count,
                    });
                }
            }
        }
    }
    Ok(result)
}

/// 返回某年每天的记录条数（year 形如 "2026"）
#[tauri::command]
pub fn get_year_active(year: String) -> Result<Vec<DayActive>, String> {
    let dir = crate::data_dir()?.join("records").join(&year);
    let mut result: Vec<DayActive> = vec![];
    if dir.exists() {
        for month_entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
            let month_entry = month_entry.map_err(|e| e.to_string())?;
            let month_dir = month_entry.path();
            if !month_dir.is_dir() {
                continue;
            }
            if let Ok(files) = fs::read_dir(&month_dir) {
                let mut day_files: Vec<PathBuf> = files
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("json"))
                    .collect();
                day_files.sort();
                for f in day_files {
                    if let Ok(content) = fs::read_to_string(&f) {
                        if let Ok(day) = serde_json::from_str::<DayRecord>(&content) {
                            let (count, todo_count) =
                                count_kinds(&day.entries);
                            result.push(DayActive {
                                date: day.date,
                                count,
                                todo_count,
                            });
                        }
                    }
                }
            }
        }
    }
    result.sort_by(|a, b| a.date.cmp(&b.date));
    Ok(result)
}
