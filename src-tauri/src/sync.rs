use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use chrono::DateTime;
use reqwest::header::{AUTHORIZATION, WWW_AUTHENTICATE};
use reqwest::Method;

use crate::settings::{fill_webdav_password, get_webdav_config_internal, WebdavConfig};

/// 同步范围：结构化记录 + 周报（不含附件、不含含密钥的 settings）
const SYNC_DIRS: [&str; 2] = ["records", "reports"];

struct Webdav {
    client: reqwest::Client,
    cfg: WebdavConfig,
    base: String,
    cnonce: String,
}

impl Webdav {
    fn new(cfg: WebdavConfig) -> Self {
        let base = format!("{}/worktrace", cfg.url.trim_end_matches('/'));
        let cnonce = uuid::Uuid::new_v4().simple().to_string();
        // 设置超时，避免网络异常时请求无限期挂起导致应用卡死
        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            cfg,
            base,
            cnonce,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.base, path.trim_start_matches('/'))
    }

    /// 发送请求：先尝试 Basic 认证（坚果云等）；
    /// 若服务端返回 401 且要求 Digest（WWW-Authenticate: Digest，如中国科技云），
    /// 则自动改用 Digest 认证重试。
    async fn send(
        &self,
        method: Method,
        url: &str,
        body: Option<Vec<u8>>,
        extra_headers: &[(&str, &str)],
    ) -> Result<reqwest::Response, String> {
        let mut req = self
            .client
            .request(method.clone(), url)
            .basic_auth(&self.cfg.account, Some(&self.cfg.password));
        for &(k, v) in extra_headers {
            req = req.header(k, v);
        }
        if let Some(b) = &body {
            req = req.body(b.clone());
        }
        let mut resp = req.send().await.map_err(|e| format!("请求失败: {e}"))?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            let challenge = resp
                .headers()
                .get(WWW_AUTHENTICATE)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            if let Some(ch) = challenge {
                if ch.trim_start().to_ascii_lowercase().starts_with("digest") {
                    let uri = digest_uri(url)?;
                    let auth = self.digest_header(&ch, method.as_str(), &uri)?;
                    let mut req2 = self.client.request(method, url).header(AUTHORIZATION, auth);
                    for &(k, v) in extra_headers {
                        req2 = req2.header(k, v);
                    }
                    if let Some(b) = &body {
                        req2 = req2.body(b.clone());
                    }
                    resp = req2
                        .send()
                        .await
                        .map_err(|e| format!("Digest 请求失败: {e}"))?;
                }
            }
        }
        Ok(resp)
    }

    /// 计算 Digest 认证的 Authorization 头（MD5 + qop=auth）
    /// 每次 401 挑战都伴随全新 nonce，故 nonce 计数 nc 固定为 1。
    fn digest_header(&self, challenge: &str, method: &str, uri: &str) -> Result<String, String> {
        let p = parse_digest_params(challenge);
        let realm = p.get("realm").cloned().unwrap_or_default();
        let nonce = p.get("nonce").cloned().ok_or("Digest 响应缺少 nonce")?;
        let opaque = p.get("opaque").cloned();
        let algorithm = p.get("algorithm").cloned().unwrap_or_else(|| "MD5".to_string());
        let qop = p.get("qop").cloned();

        let nc = "00000001".to_string();

        let ha1 = md5_hex(&format!("{}:{}:{}", self.cfg.account, realm, self.cfg.password));
        let ha2 = md5_hex(&format!("{}:{}", method, uri));

        let response = match qop.as_deref() {
            Some(q) if q == "auth" || q == "auth-int" => {
                md5_hex(&format!("{}:{}:{}:{}:{}:{}", ha1, nonce, nc, self.cnonce, q, ha2))
            }
            _ => md5_hex(&format!("{}:{}:{}", ha1, nonce, ha2)),
        };

        let mut parts = vec![
            format!("username=\"{}\"", self.cfg.account),
            format!("realm=\"{}\"", realm),
            format!("nonce=\"{}\"", nonce),
            format!("uri=\"{}\"", uri),
            format!("algorithm={}", algorithm),
            format!("response=\"{}\"", response),
        ];
        if let Some(q) = &qop {
            parts.push(format!("qop={}", q));
            parts.push(format!("nc={}", nc));
            parts.push(format!("cnonce=\"{}\"", self.cnonce));
        }
        if let Some(o) = opaque {
            parts.push(format!("opaque=\"{}\"", o));
        }
        Ok(format!("Digest {}", parts.join(", ")))
    }

    /// PROPFIND Depth: infinity 列出远端所有文件，返回 {相对路径: 最后修改时间(秒)}
    async fn list_all(&self) -> Result<HashMap<String, i64>, String> {
        let url = format!("{}/", self.base);
        let resp = self
            .send(
                Method::from_bytes(b"PROPFIND").unwrap(),
                &url,
                None,
                &[("Depth", "infinity")],
            )
            .await?;

        let status = resp.status();

        // 首次同步：远端 worktrace 目录尚不存在（404）→ 自动创建并视为空
        if status == reqwest::StatusCode::NOT_FOUND {
            self.ensure_base().await?;
            return Ok(HashMap::new());
        }

        let body = resp.text().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            let snippet: String = body.chars().take(200).collect();
            let hint = if status == reqwest::StatusCode::UNAUTHORIZED {
                "（认证失败：请确认账号为邮箱格式、密码为 WebDAV 专用应用密码）"
            } else {
                ""
            };
            return Err(format!("列出远端文件返回错误 {status}{hint}: {snippet}"));
        }
        parse_propfind(&body)
    }

    /// 确保远端 base 目录（及其父目录）存在，用于首次同步自动建目录
    async fn ensure_base(&self) -> Result<(), String> {
        let u = reqwest::Url::parse(&self.base).map_err(|e| e.to_string())?;
        let mut origin = format!("{}://{}", u.scheme(), u.host_str().unwrap_or(""));
        if let Some(port) = u.port() {
            origin = format!("{origin}:{port}");
        }
        let path = u.path().trim_matches('/');
        let mut cur = origin;
        for seg in path.split('/').filter(|s| !s.is_empty()) {
            cur = format!("{cur}/{seg}");
            let _ = self
                .send(Method::from_bytes(b"MKCOL").unwrap(), &cur, None, &[])
                .await;
        }
        Ok(())
    }

    async fn upload(&self, rel_path: &str, content: Vec<u8>) -> Result<(), String> {
        self.mkcol_parents(rel_path).await?;
        let resp = self
            .send(Method::PUT, &self.url(rel_path), Some(content), &[])
            .await?;
        if !resp.status().is_success() {
            return Err(format!("上传 {rel_path} 失败: {}", resp.status()));
        }
        Ok(())
    }

    /// 逐级创建父目录（已存在返回 405，忽略）
    async fn mkcol_parents(&self, rel_path: &str) -> Result<(), String> {
        let parts: Vec<&str> = rel_path.split('/').collect();
        let mut cur = self.base.clone();
        for p in &parts[..parts.len() - 1] {
            cur = format!("{cur}/{p}");
            let _ = self
                .send(Method::from_bytes(b"MKCOL").unwrap(), &cur, None, &[])
                .await;
        }
        Ok(())
    }

    async fn download(&self, rel_path: &str) -> Result<Vec<u8>, String> {
        let resp = self.send(Method::GET, &self.url(rel_path), None, &[]).await?;
        if !resp.status().is_success() {
            return Err(format!("下载 {rel_path} 失败: {}", resp.status()));
        }
        resp.bytes().await.map(|b| b.to_vec()).map_err(|e| e.to_string())
    }
}

fn md5_hex(s: &str) -> String {
    format!("{:x}", md5::compute(s.as_bytes()))
}

/// 提取 URL 的 path + query，作为 Digest 的 uri 字段（path 为百分号编码形式）
fn digest_uri(url: &str) -> Result<String, String> {
    let u = reqwest::Url::parse(url).map_err(|e| e.to_string())?;
    let mut uri = u.path().to_string();
    if let Some(q) = u.query() {
        uri.push('?');
        uri.push_str(q);
    }
    Ok(uri)
}

/// 解析 WWW-Authenticate: Digest 的键值参数（值可能带引号）
fn parse_digest_params(header: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let body = header
        .trim()
        .split_once(' ')
        .map(|(_, r)| r)
        .unwrap_or(header)
        .trim();
    for part in body.split(',') {
        let part = part.trim();
        if let Some(eq) = part.find('=') {
            let key = part[..eq].trim().to_lowercase();
            let mut val = part[eq + 1..].trim().to_string();
            if val.len() >= 2 && val.starts_with('"') && val.ends_with('"') {
                val = val[1..val.len() - 1].to_string();
            }
            map.insert(key, val);
        }
    }
    map
}

/// 执行一次增量同步（内部，供命令与定时任务复用）
pub async fn sync_now_internal() -> Result<String, String> {
    let cfg = get_webdav_config_internal()?.ok_or("未配置备份，请先在设置中配置 WebDAV")?;
    let wd = Webdav::new(cfg);

    let local = enumerate_local()?;
    let remote = wd.list_all().await?;

    let mut uploaded = 0usize;
    let mut downloaded = 0usize;

    // 上传本地新增或更新的文件
    for (rel, mtime) in &local {
        let should_upload = match remote.get(rel) {
            None => true,
            Some(rtime) => *mtime > *rtime,
        };
        if should_upload {
            let path = crate::data_dir()?.join(rel);
            let content = fs::read(&path).map_err(|e| e.to_string())?;
            wd.upload(rel, content).await?;
            uploaded += 1;
        }
    }

    // 下载远端新增或更新的文件
    for (rel, rtime) in &remote {
        let should_download = match local.iter().find(|(p, _)| p == rel) {
            None => true,
            Some((_, mtime)) => *rtime > *mtime,
        };
        if should_download {
            let content = wd.download(rel).await?;
            let path = crate::data_dir()?.join(rel);
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            }
            fs::write(&path, content).map_err(|e| e.to_string())?;
            downloaded += 1;
        }
    }

    Ok(format!("同步完成：上传 {uploaded} 个，下载 {downloaded} 个"))
}

/// 手动同步命令
#[tauri::command]
pub async fn sync_now() -> Result<String, String> {
    sync_now_internal().await
}

/// 测试备份账号连通性（PROPFIND 远端目录）
#[tauri::command]
pub async fn test_webdav(cfg: WebdavConfig) -> Result<String, String> {
    let mut c = cfg;
    fill_webdav_password(&mut c);
    let wd = Webdav::new(c);
    let url = format!("{}/", wd.base);
    let resp = wd
        .send(
            Method::from_bytes(b"PROPFIND").unwrap(),
            &url,
            None,
            &[("Depth", "0")],
        )
        .await?;
    match resp.status().as_u16() {
        200..=299 => Ok("连接成功".to_string()),
        401 => Err("认证失败：账号或密码错误".to_string()),
        403 => Err("无访问权限（403）".to_string()),
        404 => Ok("连接成功（远端目录将在首次同步时自动创建）".to_string()),
        s => {
            let body = resp.text().await.unwrap_or_default();
            let snippet: String = body.chars().take(150).collect();
            Err(format!("连接失败（HTTP {s}）: {snippet}"))
        }
    }
}

/// 枚举本地待同步文件，返回 (相对路径, 修改时间秒)
fn enumerate_local() -> Result<Vec<(String, i64)>, String> {
    let root = crate::data_dir()?;
    let mut files = vec![];
    for dir in SYNC_DIRS {
        let d = root.join(dir);
        if d.exists() {
            walk(&d, &root, &mut files)?;
        }
    }
    Ok(files)
}

fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, i64)>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            walk(&path, root, out)?;
        } else {
            let rel = path
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            let mtime = fs::metadata(&path)
                .map_err(|e| e.to_string())?
                .modified()
                .map_err(|e| e.to_string())?
                .duration_since(UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_secs() as i64;
            out.push((rel, mtime));
        }
    }
    Ok(())
}

/// 解析 PROPFIND 响应，提取每个「文件」的 href 与 getlastmodified（跳过目录）
fn parse_propfind(xml: &str) -> Result<HashMap<String, i64>, String> {
    let mut map = HashMap::new();
    let mut reader = quick_xml::Reader::from_str(xml);

    // 当前 <d:response> 的解析状态
    let mut cur_href: Option<String> = None;
    let mut cur_lm: Option<i64> = None;
    let mut cur_is_collection = false;

    let mut in_href = false;
    let mut in_lm = false;
    let mut href_buf = String::new();
    let mut lm_buf = String::new();

    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Start(ref e)) => match e.local_name().as_ref() {
                b"response" => {
                    cur_href = None;
                    cur_lm = None;
                    cur_is_collection = false;
                }
                b"href" => {
                    in_href = true;
                    href_buf.clear();
                }
                b"getlastmodified" => {
                    in_lm = true;
                    lm_buf.clear();
                }
                _ => {}
            },
            Ok(quick_xml::events::Event::Empty(ref e)) => {
                // 自闭合 <d:collection/> 表示目录
                if e.local_name().as_ref() == b"collection" {
                    cur_is_collection = true;
                }
            }
            Ok(quick_xml::events::Event::Text(ref e)) => {
                if in_href {
                    if let Ok(t) = e.unescape() {
                        href_buf.push_str(&t);
                    }
                } else if in_lm {
                    if let Ok(t) = e.unescape() {
                        lm_buf.push_str(&t);
                    }
                }
            }
            Ok(quick_xml::events::Event::End(ref e)) => match e.local_name().as_ref() {
                b"href" => {
                    in_href = false;
                    cur_href = Some(href_buf.trim().to_string());
                }
                b"getlastmodified" => {
                    in_lm = false;
                    cur_lm = parse_http_date(lm_buf.trim()).ok();
                }
                b"response" => {
                    // 仅文件（非目录）且路径有效时加入清单
                    if !cur_is_collection {
                        if let (Some(href), Some(ts)) = (&cur_href, cur_lm) {
                            if let Some(rel) = href_to_rel(href) {
                                map.insert(rel, ts);
                            }
                        }
                    }
                }
                _ => {}
            },
            Ok(quick_xml::events::Event::Eof) => break,
            Err(e) => return Err(format!("解析远端清单失败: {e}")),
            _ => {}
        }
    }
    Ok(map)
}

/// href 形如 .../worktrace/records/...，取 worktrace/ 之后的相对路径
fn href_to_rel(href: &str) -> Option<String> {
    let idx = href.find("/worktrace/")?;
    let rel = &href[idx + "/worktrace/".len()..];
    if rel.is_empty() {
        None
    } else {
        Some(rel.trim_end_matches('/').to_string())
    }
}

fn parse_http_date(s: &str) -> Result<i64, String> {
    DateTime::parse_from_rfc2822(s)
        .map(|dt| dt.timestamp())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gmt_date() {
        let ts = parse_http_date("Sun, 16 Aug 2026 09:53:41 GMT").unwrap();
        assert!(ts > 0);
        eprintln!("parsed timestamp: {}", ts);
    }

    #[test]
    fn test_parse_propfind_jianguoyun() {
        // 贴近坚果云真实响应：getlastmodified 在 resourcetype 之前；目录带 <d:collection/>
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?><d:multistatus xmlns:d="DAV:" xmlns:s="http://ns.jianguoyun.com"><d:response><d:href>/dav/worktrace/</d:href><d:propstat><d:prop><d:getlastmodified>Sun, 16 Aug 2026 09:53:41 GMT</d:getlastmodified><d:resourcetype><d:collection/></d:resourcetype></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response><d:response><d:href>/dav/worktrace/records</d:href><d:propstat><d:prop><d:getlastmodified>Sun, 16 Aug 2026 10:00:00 GMT</d:getlastmodified><d:resourcetype><d:collection/></d:resourcetype></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response><d:response><d:href>/dav/worktrace/records/2026-08-16.json</d:href><d:propstat><d:prop><d:getlastmodified>Sun, 16 Aug 2026 10:30:00 GMT</d:getlastmodified></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response></d:multistatus>"#;
        let map = parse_propfind(xml).unwrap();
        eprintln!("parsed map: {:?}", map);
        assert_eq!(map.len(), 1);
        assert!(map.contains_key("records/2026-08-16.json"));
        assert!(!map.contains_key("records"));
    }
}
