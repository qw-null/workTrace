use std::io::Read;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

/// 解析附件内容为文本：docx/pdf 本地提取，图片走多模态视觉模型识别
#[tauri::command]
pub async fn parse_attachment(
    filename: String,
    base64_data: String,
    model_id: Option<String>,
) -> Result<String, String> {
    let data = STANDARD
        .decode(&base64_data)
        .map_err(|e| format!("解码文件失败: {e}"))?;
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();

    match ext.as_str() {
        "docx" | "doc" => parse_docx(&data),
        "pdf" => parse_pdf(&data),
        "png" | "jpg" | "jpeg" | "gif" | "webp" => {
            parse_image(&data, &ext, model_id.as_deref()).await
        }
        _ => Err(format!("暂不支持的文件类型: {ext}")),
    }
}

/// 解包 docx（zip）并提取 word/document.xml 中的 <w:t> 文本
fn parse_docx(data: &[u8]) -> Result<String, String> {
    let reader = std::io::Cursor::new(data);
    let mut zip = zip::ZipArchive::new(reader).map_err(|e| format!("打开 docx 失败: {e}"))?;
    let mut xml = String::new();
    zip.by_name("word/document.xml")
        .map_err(|e| format!("读取 document.xml 失败: {e}"))?
        .read_to_string(&mut xml)
        .map_err(|e| e.to_string())?;

    let mut reader = quick_xml::Reader::from_str(&xml);
    let mut texts: Vec<String> = vec![];
    let mut in_t = false;
    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Start(ref e)) => {
                let name = e.name();
                if name.as_ref() == b"w:t" {
                    in_t = true;
                } else if name.as_ref() == b"w:p" {
                    texts.push("\n".to_string());
                }
            }
            Ok(quick_xml::events::Event::End(ref e)) => {
                if e.name().as_ref() == b"w:t" {
                    in_t = false;
                }
            }
            Ok(quick_xml::events::Event::Text(ref e)) => {
                if in_t {
                    if let Ok(t) = e.unescape() {
                        texts.push(t.into_owned());
                    }
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(e) => return Err(format!("解析 docx XML 失败: {e}")),
            _ => {}
        }
    }

    let text = texts.join("");
    if text.trim().is_empty() {
        Err("docx 未提取到文本".to_string())
    } else {
        Ok(text)
    }
}

/// 写临时文件后用 pdf-extract 提取文本
fn parse_pdf(data: &[u8]) -> Result<String, String> {
    let mut tmp = std::env::temp_dir();
    tmp.push(format!("worktrace-{}.pdf", uuid::Uuid::new_v4()));
    std::fs::write(&tmp, data).map_err(|e| e.to_string())?;

    let result = pdf_extract::extract_text(&tmp).map_err(|e| format!("提取 PDF 文本失败: {e}"));
    let _ = std::fs::remove_file(&tmp);

    let text = result?;
    if text.trim().is_empty() {
        Err("PDF 未提取到文本（可能为扫描件，需走图片识别）".to_string())
    } else {
        Ok(text)
    }
}

// 内置 PaddleOCR 图片识别（讯飞免费 OCR 服务，开箱即用，无需用户配置）
const OCR_BASE_URL: &str = "https://maas-api.cn-huabei-1.xf-yun.com/v2";
const OCR_MODEL: &str = "xoppaddleocrv16";
const OCR_API_KEY: &str = "ea3c6ea556d43a6c02a44eed60ab04b2:NzYyYmU2OGQwYjE0MmVmYTFmMzRkMGVh";

/// 图片识别入口：使用内置 PaddleOCR，无需用户配置
async fn parse_image(data: &[u8], ext: &str, _model_id: Option<&str>) -> Result<String, String> {
    let _ = _model_id;
    let base = OCR_BASE_URL.trim_end_matches('/');
    let url = format!("{}/chat/completions", base);
    let mime_ext = if ext == "jpg" { "jpeg" } else { ext };
    let data_url = format!("data:image/{mime_ext};base64,{}", STANDARD.encode(data));

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", OCR_API_KEY))
        .json(&serde_json::json!({
            "model": OCR_MODEL,
            "messages": [{
                "role": "user",
                "content": [
                    { "type": "text", "text": "请识别这张图片中的文字，直接输出识别结果，不要多余说明。" },
                    { "type": "image_url", "image_url": { "url": data_url } }
                ]
            }]
        }))
        .send()
        .await
        .map_err(|e| format!("请求 OCR 服务失败: {e}"))?;

    let status = resp.status();
    let body = resp.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        let snippet: String = body.chars().take(200).collect();
        return Err(format!("OCR 服务返回错误 {status}: {snippet}"));
    }

    let v: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("解析 OCR 响应失败: {e}"))?;
    v["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "识别结果为空".to_string())
}
