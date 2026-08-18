use chrono::{Datelike, Duration, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::ai::chat_completion;
use crate::storage;

const REPORT_SYSTEM: &str = "你是工作周报生成助手。根据本周的工作记录，生成一份周报，Markdown 格式，包含以下四个板块：\n\n## 本周完成事项\n## 关键成果 / 亮点\n## 问题与风险\n## 下周计划\n\n要求：\n1. 忠实记录内容，不编造、不夸大。\n2. 用简洁的要点列表（- 开头）。\n3. 标记为【待办】的条目是尚未完成的待办事项，应归入「下周计划」板块，不要当作已完成事项。\n4. 若某板块没有对应内容，写「无」。\n5. 只输出 Markdown 正文，不要额外解释。";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub week_start: String,
    pub week_end: String,
    pub content: String,
    pub generated_at: String,
    pub model_used: String,
}

/// 汇总本周记录并调用 AI 生成周报，保存到 reports/ 目录后返回
#[tauri::command]
pub async fn generate_report(week_start: String, model_id: String) -> Result<Report, String> {
    let start = NaiveDate::parse_from_str(&week_start, "%Y-%m-%d")
        .map_err(|e| format!("周起始日期格式错误: {e}"))?;
    let end = start + Duration::days(6);
    let week_end = end.format("%Y-%m-%d").to_string();

    // 汇总本周 7 天的记录
    let mut summary = String::new();
    let mut total = 0usize;
    for i in 0..7 {
        let date = (start + Duration::days(i)).format("%Y-%m-%d").to_string();
        if let Some(day) = storage::read_day(&date)? {
            if day.entries.is_empty() {
                continue;
            }
            summary.push_str(&format!("【{date}】\n"));
            for e in &day.entries {
                let s = &e.structured;
                if e.kind == "todo" {
                    // 待办条目：明确标记，供 AI 归入「下周计划」
                    if !s.tasks.is_empty() {
                        summary.push_str(&format!("- 【待办】{}\n", s.tasks.join("；")));
                    } else {
                        summary.push_str(&format!("- 【待办】{}\n", s.summary));
                    }
                } else {
                    summary.push_str(&format!("- 摘要：{}\n", s.summary));
                    if !s.tasks.is_empty() {
                        summary.push_str(&format!("  任务：{}\n", s.tasks.join("；")));
                    }
                    if !s.outputs.is_empty() {
                        summary.push_str(&format!("  产出：{}\n", s.outputs.join("；")));
                    }
                    if !s.todos.is_empty() {
                        summary.push_str(&format!("  待办：{}\n", s.todos.join("；")));
                    }
                }
            }
            total += day.entries.len();
        }
    }

    if total == 0 {
        return Err("本周还没有记录，先在工作台记录一些内容吧".to_string());
    }

    let content = chat_completion(&model_id, REPORT_SYSTEM, &summary).await?;

    // 保存 Markdown 到数据目录 reports/YYYY-Www.md
    let iso = start.iso_week();
    let filename = format!("reports/{}-W{:02}.md", iso.year(), iso.week());
    let path = crate::data_dir()?.join(&filename);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, &content).map_err(|e| e.to_string())?;

    Ok(Report {
        week_start,
        week_end,
        content,
        generated_at: chrono::Local::now().to_rfc3339(),
        model_used: model_id,
    })
}

/// 读取某周已保存的周报（存在则返回，否则 None）
#[tauri::command]
pub fn get_report(week_start: String) -> Result<Option<Report>, String> {
    let start = NaiveDate::parse_from_str(&week_start, "%Y-%m-%d")
        .map_err(|e| format!("周起始日期格式错误: {e}"))?;
    let iso = start.iso_week();
    let filename = format!("reports/{}-W{:02}.md", iso.year(), iso.week());
    let path = crate::data_dir()?.join(&filename);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let generated_at = std::fs::metadata(&path)
        .and_then(|m| m.modified())
        .map(|t| {
            let dt: chrono::DateTime<chrono::Utc> = t.into();
            dt.to_rfc3339()
        })
        .unwrap_or_default();
    let end = start + Duration::days(6);
    Ok(Some(Report {
        week_start,
        week_end: end.format("%Y-%m-%d").to_string(),
        content,
        generated_at,
        model_used: String::new(),
    }))
}

/// 导出周报：word 生成 .doc（HTML 内容，Word/WPS 可打开），pdf 生成 .html（可打印），返回保存路径
#[tauri::command]
pub fn export_report(content: String, week_start: String, format: String) -> Result<String, String> {
    let html = markdown_to_html(&content);
    let filename = match format.as_str() {
        "word" => format!("report-{week_start}.doc"),
        "pdf" => format!("report-{week_start}.html"),
        _ => return Err("未知导出格式".to_string()),
    };
    let path = crate::data_dir()?.join("reports").join(&filename);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, &html).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

fn markdown_to_html(md: &str) -> String {
    let mut html = String::from(
        "<html xmlns:o=\"urn:schemas-microsoft-com:office:office\" xmlns:w=\"urn:schemas-microsoft-com:office:word\" xmlns=\"http://www.w3.org/TR/REC-html40\"><head><meta charset=\"utf-8\"><style>body{font-family:'PingFang SC','Microsoft YaHei',sans-serif;line-height:1.9;max-width:800px;margin:40px auto;padding:0 20px;color:#333;}h2{color:#1d6fc4;margin:24px 0 10px;font-size:17px;}li{margin:5px 0;}p{margin:8px 0;}</style></head><body>",
    );
    let mut in_list = false;
    for line in md.lines() {
        if line.starts_with("## ") {
            if in_list {
                html.push_str("</ul>");
                in_list = false;
            }
            html.push_str(&format!("<h2>{}</h2>", escape_html(&line[3..])));
        } else if line.starts_with("- ") {
            if !in_list {
                html.push_str("<ul>");
                in_list = true;
            }
            html.push_str(&format!("<li>{}</li>", escape_html(&line[2..])));
        } else if line.trim().is_empty() {
            if in_list {
                html.push_str("</ul>");
                in_list = false;
            }
        } else {
            if in_list {
                html.push_str("</ul>");
                in_list = false;
            }
            html.push_str(&format!("<p>{}</p>", escape_html(line)));
        }
    }
    if in_list {
        html.push_str("</ul>");
    }
    html.push_str("</body></html>");
    html
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
