use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// 结构化记录中的流程图
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flowchart {
    pub title: String,
    pub mermaid: String,
}

/// AI 转化后的结构化字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Structured {
    pub summary: String,
    pub tasks: Vec<String>,
    pub tags: Vec<String>,
    pub outputs: Vec<String>,
    pub flowcharts: Vec<Flowchart>,
    pub todos: Vec<String>,
}

/// 单条工作记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordEntry {
    pub id: String,
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
pub struct DayActive {
    pub date: String,
    pub count: usize,
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

/// 把一条 AI 转化结果确认入库（追加到当日记录）
#[tauri::command]
pub fn confirm_record(
    date: String,
    raw_text: String,
    model_id: String,
    structured: Structured,
) -> Result<(), String> {
    let now = chrono::Local::now().to_rfc3339();
    let entry = RecordEntry {
        id: uuid::Uuid::new_v4().to_string(),
        created_at: now.clone(),
        updated_at: now,
        raw_text,
        source_attachments: vec![],
        model_used: model_id,
        status: "confirmed".to_string(),
        structured,
    };
    let mut day = read_day(&date)?.unwrap_or(DayRecord {
        version: 1,
        date: date.clone(),
        entries: vec![],
    });
    day.entries.push(entry);
    write_day(&day)
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
                    result.push(DayActive {
                        date: day.date,
                        count: day.entries.len(),
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
                            result.push(DayActive {
                                date: day.date,
                                count: day.entries.len(),
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
