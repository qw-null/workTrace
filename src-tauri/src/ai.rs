use serde_json::{json, Value};

use crate::settings;
use crate::storage::Structured;

const SYSTEM_PROMPT: &str = "你是工作日志结构化助手。把用户的工作记录（可能包含文字、从 Word/PDF 提取的文本、图片内容说明）转化为结构化 JSON。只输出一个合法 JSON 对象，不要输出任何其他文字、注释或 Markdown 代码块。\n\n输出 Schema：\n{\n  \"summary\": \"一句话摘要，说明做了什么，不超过 30 字\",\n  \"tasks\": [\"拆分出的具体事项，动词开头，每项一句话\"],\n  \"tags\": [\"分类标签\"],\n  \"outputs\": [\"交付的产出物或成果\"],\n  \"flowcharts\": [{\"title\": \"流程名\", \"mermaid\": \"Mermaid flowchart 代码\"}],\n  \"todos\": [\"后续待办或明日计划，可为空数组\"]\n}\n\n规则：\n1. 忠实原文，不编造、不夸大。\n2. 只有内容中存在清晰流程/逻辑关系时才生成 flowcharts，最多 2 个，节点文字简短。\n3. 无法判断的字段用空数组或空字符串。";

/// 通用 OpenAI 兼容对话调用，返回助手回复文本
pub async fn chat_completion(model_id: &str, system: &str, user: &str) -> Result<String, String> {
    let model = settings::get_model(model_id)?;
    if model.api_key.is_empty() {
        return Err("该模型未配置 API Key".to_string());
    }

    let base = model.base_url.trim_end_matches('/');
    let url = format!("{}/chat/completions", base);

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", model.api_key))
        .json(&json!({
            "model": model.model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user }
            ],
            "temperature": 0.3
        }))
        .send()
        .await
        .map_err(|e| format!("请求模型失败: {e}"))?;

    let status = resp.status();
    let body = resp.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("模型返回错误 {status}: {}", truncate(&body, 200)));
    }

    let v: Value = serde_json::from_str(&body).map_err(|e| format!("解析响应失败: {e}"))?;
    v["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "模型响应缺少 content".to_string())
}

/// 把原始输入交给指定模型转化为结构化记录
#[tauri::command]
pub async fn transform_record(input: String, model_id: String) -> Result<Structured, String> {
    let content = chat_completion(&model_id, SYSTEM_PROMPT, &input).await?;
    parse_structured(&content)
}

/// 从模型返回的 content 中提取并解析 Structured（兼容 ```json 代码块包裹）
fn parse_structured(content: &str) -> Result<Structured, String> {
    let mut s = content.trim();
    if let Some(stripped) = s.strip_prefix("```json").or_else(|| s.strip_prefix("```")) {
        s = stripped;
    }
    if let Some(stripped) = s.strip_suffix("```") {
        s = stripped;
    }
    let s = s.trim();

    let start = s.find('{').ok_or("返回内容不是 JSON")?;
    let end = s.rfind('}').ok_or("返回内容不是 JSON")?;
    serde_json::from_str(&s[start..=end]).map_err(|e| format!("解析结构化结果失败: {e}"))
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
