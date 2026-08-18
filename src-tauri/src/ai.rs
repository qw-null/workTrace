use serde_json::{json, Value};

use crate::settings;
use crate::storage::{RecordField, TodoField};

const RECORD_SYSTEM_PROMPT: &str = "你是一个专业的职场助理。请将我输入的文本解析并拆分为多条独立的工作记录。\n如果文本中包含多项工作内容，请务必自动拆分，并使用\"### 记录 1\"、\"### 记录 2\"等标题区分。\n每条记录必须严格按照以下格式输出，不要添加任何多余的说明或问候语：\n\n### 记录 [序号]\n时间：[日期/时间段]\n工作内容：[具体做了什么]\n进度/结果：[完成状态、产出或直接结果]\n相关人员：[涉及的对内/对外人员或部门，未提及填\"无\"]\n备注/下一步：[需要留意的问题或后续计划，未提及填\"无\"]";

const TODO_SYSTEM_PROMPT: &str = "你是一个专业的日程秘书。请将我输入的文本解析并拆分为多条独立的待办事项。\n如果文本中只包含一件事，则只输出一组；如果包含多件事，请务必自动拆分，并使用\"### 待办 1\"、\"### 待办 2\"等标题进行区分。\n每条待办事项必须严格按照以下格式输出，不要添加任何多余的说明、问候语或总结：\n\n### 待办 [序号]\n时间地点：[提取的时间和地点，如果原文未提及请填\"无\"]\n事项：[核心待办事项]\n注意点：[需要留意的事项、物品或特别提醒，如果原文未提及请填\"无\"]";

/// 通用 OpenAI 兼容对话调用，返回助手回复文本
pub async fn chat_completion(model_id: &str, system: &str, user: &str) -> Result<String, String> {
    let model = settings::get_model(model_id)?;
    if model.api_key.is_empty() {
        return Err("该模型未配置 API Key".to_string());
    }

    let base = model.base_url.trim_end_matches('/');
    let url = format!("{}/chat/completions", base);

    let client = reqwest::Client::new();
    let auth = format!("Bearer {}", model.api_key);
    let model_name = model.model.clone();
    let messages = json!([
        { "role": "system", "content": system },
        { "role": "user", "content": user }
    ]);

    // 先带 temperature=0.3（多数模型输出更稳定）；部分推理模型（如 kimi-k3）
    // 只允许 temperature=1，若返回 400 且提示 temperature 非法，则去掉 temperature 重试一次。
    let mut resp = client
        .post(&url)
        .header("Authorization", &auth)
        .json(&json!({
            "model": model_name.clone(),
            "messages": messages.clone(),
            "temperature": 0.3
        }))
        .send()
        .await
        .map_err(|e| format!("请求模型失败: {e}"))?;

    let mut status = resp.status();
    let mut body = resp.text().await.map_err(|e| e.to_string())?;

    if status == reqwest::StatusCode::BAD_REQUEST && body.contains("temperature") {
        resp = client
            .post(&url)
            .header("Authorization", &auth)
            .json(&json!({
                "model": model_name,
                "messages": messages,
            }))
            .send()
            .await
            .map_err(|e| format!("请求模型失败: {e}"))?;
        status = resp.status();
        body = resp.text().await.map_err(|e| e.to_string())?;
    }

    if !status.is_success() {
        return Err(format!("模型返回错误 {status}: {}", truncate(&body, 200)));
    }

    let v: Value = serde_json::from_str(&body).map_err(|e| format!("解析响应失败: {e}"))?;
    v["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "模型响应缺少 content".to_string())
}

/// 把原始输入交给指定模型转化为多条工作记录
#[tauri::command]
pub async fn transform_record(input: String, model_id: String) -> Result<Vec<RecordField>, String> {
    let content = chat_completion(&model_id, RECORD_SYSTEM_PROMPT, &input).await?;
    let records = parse_record_text(&content)?;
    if records.is_empty() {
        return Err("模型未解析出有效记录".to_string());
    }
    Ok(records)
}

/// 把待办输入交给指定模型转化为多条待办
#[tauri::command]
pub async fn transform_todo(input: String, model_id: String) -> Result<Vec<TodoField>, String> {
    let content = chat_completion(&model_id, TODO_SYSTEM_PROMPT, &input).await?;
    let todos = parse_todo_text(&content)?;
    if todos.is_empty() {
        return Err("模型未解析出有效待办".to_string());
    }
    Ok(todos)
}

/// 解析记录文本：按「### 记录 N」分块，提取每块的字段行
fn parse_record_text(content: &str) -> Result<Vec<RecordField>, String> {
    let blocks = split_blocks(content, "记录");
    let mut records: Vec<RecordField> = vec![];
    for block in blocks {
        let mut f = RecordField::default();
        let mut has_content = false;
        for line in block.lines() {
            let line = line.trim();
            if let Some((k, v)) = split_kv(line) {
                match k.as_str() {
                    "时间" => f.time = v,
                    "工作内容" | "内容" => {
                        f.content = v;
                        has_content = true;
                    }
                    "进度/结果" | "进度" | "结果" => f.progress = v,
                    "相关人员" | "人员" => f.people = v,
                    "备注/下一步" | "备注" | "下一步" => f.next = v,
                    _ => {}
                }
            }
        }
        if has_content || !f.time.is_empty() {
            records.push(f);
        }
    }
    Ok(records)
}

/// 解析待办文本：按「### 待办 N」分块，提取每块的字段行
fn parse_todo_text(content: &str) -> Result<Vec<TodoField>, String> {
    let blocks = split_blocks(content, "待办");
    let mut todos: Vec<TodoField> = vec![];
    for block in blocks {
        let mut f = TodoField::default();
        let mut has_item = false;
        for line in block.lines() {
            let line = line.trim();
            if let Some((k, v)) = split_kv(line) {
                match k.as_str() {
                    "时间地点" | "时间" | "地点" => f.time_location = v,
                    "事项" | "内容" => {
                        f.item = v;
                        has_item = true;
                    }
                    "注意点" | "注意" | "提醒" => f.note = v,
                    _ => {}
                }
            }
        }
        if has_item || !f.item.is_empty() {
            todos.push(f);
        }
    }
    Ok(todos)
}

/// 把输出按「### 标题 N」标题行切分成块（去掉标题行本身）
fn split_blocks(content: &str, title: &str) -> Vec<String> {
    let mut blocks: Vec<String> = vec![];
    let mut current = String::new();
    for line in content.lines() {
        let t = line.trim();
        if t.starts_with("###") && t.contains(title) {
            if !current.trim().is_empty() {
                blocks.push(std::mem::take(&mut current));
            }
        } else {
            current.push_str(line);
            current.push('\n');
        }
    }
    if !current.trim().is_empty() {
        blocks.push(current);
    }
    blocks
}

/// 从一行「字段：值」中拆出字段名和值（兼容中英文冒号）
fn split_kv(line: &str) -> Option<(String, String)> {
    // 优先中文冒号，否则英文冒号；用 char_indices 拿到准确字节偏移与字符长度
    let (idx, ch) = line
        .char_indices()
        .find(|(_, c)| *c == '：' || *c == ':')?;
    let key = line[..idx].trim().trim_matches(|c| c == '*' || c == '-').trim();
    let val = line[idx + ch.len_utf8()..].trim();
    if key.is_empty() {
        return None;
    }
    Some((key.to_string(), val.to_string()))
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() > n {
        let mut out: String = s.chars().take(n).collect();
        out.push('…');
        out
    } else {
        s.to_string()
    }
}

/// 测试模型连通性（发送最小请求）
#[tauri::command]
pub async fn test_model(model: crate::settings::ModelConfig) -> Result<String, String> {
    let mut m = model;
    // api_key 为空时，尝试从已保存配置（Keychain/JSON）回填
    if m.api_key.is_empty() && !m.id.is_empty() {
        if let Ok(saved) = crate::settings::get_model(&m.id) {
            m.api_key = saved.api_key;
        }
    }
    if m.api_key.is_empty() {
        return Err("未填写 API Key".to_string());
    }
    if m.base_url.trim().is_empty() || m.model.trim().is_empty() {
        return Err("请先填写 Base URL 和模型名".to_string());
    }

    let base = m.base_url.trim_end_matches('/');
    let url = format!("{}/chat/completions", base);
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", m.api_key))
        .json(&json!({
            "model": m.model,
            "messages": [{ "role": "user", "content": "ping" }],
            "max_tokens": 5
        }))
        .send()
        .await
        .map_err(|e| format!("连接失败: {e}"))?;

    let status = resp.status();
    if status.is_success() {
        Ok("连通成功".to_string())
    } else {
        let body = resp.text().await.unwrap_or_default();
        Err(format!("连通失败 {status}: {}", truncate(&body, 120)))
    }
}
