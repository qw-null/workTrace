use serde_json::{json, Value};

use crate::settings;
use crate::storage::Structured;

const SCHEMA_DESC: &str = r#"{
  "summary": "一句话摘要（字符串）",
  "tasks": ["字符串数组，每项为动宾短语"],
  "tags": ["字符串数组，分类标签"],
  "outputs": ["字符串数组，交付产出物"],
  "flowcharts": [{"title": "字符串", "mermaid": "Mermaid 代码字符串"}],
  "todos": ["字符串数组，后续待办"]
}"#;

const SYSTEM_PROMPT: &str = "你是工作日志结构化助手。把用户的工作记录（可能包含文字、从 Word/PDF 提取的文本、图片内容说明）转化为结构化 JSON。\n只输出一个合法 JSON 对象，不要输出任何其他文字、注释或 Markdown 代码块。\n\n输出 Schema（严格遵守，字段名与类型必须完全一致）：\n{\n  \"summary\": \"一句话摘要（字符串）\",\n  \"tasks\": [\"字符串数组，每项为动宾短语\"],\n  \"tags\": [\"字符串数组，分类标签\"],\n  \"outputs\": [\"字符串数组，交付产出物\"],\n  \"flowcharts\": [{\"title\": \"字符串\", \"mermaid\": \"Mermaid 代码字符串\"}],\n  \"todos\": [\"字符串数组，后续待办\"]\n}\n\n硬性要求：\n1. tasks、tags、outputs、todos 都必须是「字符串数组」——数组里每个元素都是字符串，禁止使用对象或嵌套结构。\n2. flowcharts 数组的每个元素是含 title、mermaid 两个字符串字段的对象。\n3. 忠实原文，不编造、不夸大。\n4. 只有内容存在清晰流程/逻辑关系时才生成 flowcharts，最多 2 个，节点文字简短。\n5. 无法判断的字段用空数组或空字符串。";

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

/// 把原始输入交给指定模型转化为结构化记录
#[tauri::command]
pub async fn transform_record(input: String, model_id: String) -> Result<Structured, String> {
    let content = chat_completion(&model_id, SYSTEM_PROMPT, &input).await?;
    match parse_structured(&content) {
        Ok(s) => Ok(s),
        Err(first_err) => {
            // 首次解析失败（常见于推理模型不严格遵循 Schema），反馈错误让模型修正一次
            let fix_prompt = format!(
                "你上一次输出的内容不符合要求的 JSON Schema，无法解析，错误信息：{first_err}\n\n请严格按照下面的 Schema 重新输出，tasks/tags/outputs/todos 必须是字符串数组，不要输出任何解释文字或 Markdown 代码块：\n{SCHEMA_DESC}"
            );
            let fixed = chat_completion(&model_id, &fix_prompt, &input).await?;
            parse_structured(&fixed).map_err(|e| {
                format!("模型输出格式不正确：{first_err}（自动修正后仍失败：{e}）")
            })
        }
    }
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
