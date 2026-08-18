import { useEffect, useRef, useState } from "react";
import { api, RECORD_CHANGED_EVENT } from "../api";
import type { ModelConfig, Structured } from "../types";
import FlowDiagram from "./FlowDiagram";

// 输入草稿的本地持久化键：切换页面/重启应用时恢复未提交的内容
const DRAFT_KEY = "worktrace:input-draft";

function todayStr(): string {
  const d = new Date();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${m}-${day}`;
}

function fileToBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve((reader.result as string).split(",")[1] || "");
    reader.onerror = reject;
    reader.readAsDataURL(file);
  });
}

// 粘贴的文件可能没有文件名/扩展名，根据 MIME 类型补全
function ensureFilename(file: File): string {
  if (file.name && file.name.includes(".")) return file.name;
  const ext = (file.type.split("/")[1] || "png").toLowerCase();
  const map: Record<string, string> = { jpeg: "jpg", "svg+xml": "svg" };
  return `粘贴图片.${map[ext] || ext}`;
}

export default function InputBox() {
  const [models, setModels] = useState<ModelConfig[]>([]);
  const [modelId, setModelId] = useState("");
  const [text, setText] = useState<string>(() => {
    try {
      return localStorage.getItem(DRAFT_KEY) || "";
    } catch {
      return "";
    }
  });
  const [loading, setLoading] = useState(false);
  const [parsing, setParsing] = useState(false);
  const [result, setResult] = useState<Structured | null>(null);
  const [error, setError] = useState("");
  const [saved, setSaved] = useState("");
  const [mode, setMode] = useState<"record" | "todo">("record");
  const fileRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    api
      .listModels()
      .then((ms) => {
        if (ms.length > 0) {
          setModels(ms);
          // 默认选中「记录转化」用途的模型（排除图片识别专用的 vision 模型）
          const recordModels = ms.filter((m) => m.role !== "vision");
          const def = recordModels.find((m) => m.isDefault) || recordModels[0];
          if (def) setModelId(def.id);
        }
      })
      .catch(() => {
        /* 浏览器预览环境无后端，保持 fallback */
      });
  }, []);

  // 输入内容实时持久化，切换页面/重启应用后自动恢复
  useEffect(() => {
    try {
      if (text) localStorage.setItem(DRAFT_KEY, text);
      else localStorage.removeItem(DRAFT_KEY);
    } catch {
      /* 忽略存储异常 */
    }
  }, [text]);

  const send = async () => {
    if (!text.trim()) return;
    if (!modelId) {
      setError("请先在「设置」配置模型");
      return;
    }
    setLoading(true);
    setError("");
    setSaved("");
    setResult(null);
    try {
      const structured = await api.transformRecord(text.trim(), modelId);
      setResult(structured);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const confirm = async () => {
    if (!result) return;
    try {
      await api.confirmRecord(todayStr(), text.trim(), modelId, result);
      setSaved("已入库");
      setResult(null);
      setText("");
      try {
        localStorage.removeItem(DRAFT_KEY);
      } catch {
        /* ignore */
      }
      window.dispatchEvent(new Event(RECORD_CHANGED_EVENT));
    } catch (e) {
      setError(String(e));
    }
  };

  // 待办模式：直接入库（不经过 AI 结构化）
  const addTodo = async () => {
    if (!text.trim()) return;
    try {
      await api.confirmTodo(todayStr(), text.trim());
      setSaved("待办已添加");
      setText("");
      try {
        localStorage.removeItem(DRAFT_KEY);
      } catch {
        /* ignore */
      }
      window.dispatchEvent(new Event(RECORD_CHANGED_EVENT));
    } catch (e) {
      setError(String(e));
    }
  };

  // 解析单个附件（上传或粘贴共用），识别结果以文本块追加到输入框
  const processFile = async (file: File) => {
    setError("");
    setParsing(true);
    try {
      const base64 = await fileToBase64(file);
      const name = ensureFilename(file);
      const visionId = models.find((m) => m.role === "vision")?.id || modelId;
      const content = await api.parseAttachment(name, base64, visionId);
      const block = `【附件：${name}】\n${content}`;
      setText((t) => (t ? `${t}\n\n${block}` : block));
    } catch (err) {
      setError(String(err));
    } finally {
      setParsing(false);
    }
  };

  const handleFile = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    e.target.value = "";
    await processFile(file);
  };

  // 支持直接粘贴图片/文件到输入框
  const handlePaste = async (e: React.ClipboardEvent<HTMLTextAreaElement>) => {
    const items = e.clipboardData?.items;
    if (!items) return;
    const files: File[] = [];
    for (const item of Array.from(items)) {
      if (item.kind === "file") {
        const f = item.getAsFile();
        if (f) files.push(f);
      }
    }
    if (files.length === 0) return; // 纯文本粘贴，走默认行为
    e.preventDefault(); // 阻止把图片 base64 文本粘进输入框
    for (const f of files) {
      await processFile(f);
    }
  };

  return (
    <div className="card record-box">
      <h3>记录工作</h3>
      <div className="sub">统一对话入口 · 文字 / 图片 / 文件</div>

      <div className="mode-toggle">
        <button
          className={"mode-btn" + (mode === "record" ? " active" : "")}
          onClick={() => {
            setMode("record");
            setResult(null);
          }}
        >
          记录
        </button>
        <button
          className={"mode-btn" + (mode === "todo" ? " active" : "")}
          onClick={() => {
            setMode("todo");
            setResult(null);
          }}
        >
          待办
        </button>
      </div>

      {mode === "record" && (
        <div className="model-pick">
          <label>模型</label>
          <select value={modelId} onChange={(e) => setModelId(e.target.value)}>
            {models
              .filter((m) => m.role !== "vision")
              .map((m) => (
                <option key={m.id} value={m.id}>
                  {m.name}
                </option>
              ))}
          </select>
        </div>
      )}

      <div className="input-area">
        <textarea
          placeholder={
            mode === "record"
              ? "今天做了什么？粘贴文字或图片，或点击下方图标添加 Word / PDF / 图片…"
              : "添加待办事项，每行一条…"
          }
          value={text}
          onChange={(e) => setText(e.target.value)}
          onPaste={handlePaste}
        />
      </div>

      <div className="input-foot">
        {mode === "record" && (
          <button className="icon-btn" title="添加图片 / 文件" onClick={() => fileRef.current?.click()}>
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="#6b7686" strokeWidth="1.4">
              <path d="M4 1.5h5l3 3v9a1 1 0 01-1 1H4a1 1 0 01-1-1V2.5a1 1 0 011-1z" />
              <path d="M9 1.5v3h3" />
            </svg>
          </button>
        )}
        <span className="hint">
          {parsing
            ? "正在识别图片 / 解析附件…"
            : mode === "record"
              ? "点击图标或直接粘贴图片 / 文件"
              : "每行一条待办，点击即可添加"}
        </span>
        {mode === "record" ? (
          <button className="btn btn-primary" onClick={send} disabled={loading}>
            {loading ? "转化中…" : "发送"}
          </button>
        ) : (
          <button className="btn btn-primary" onClick={addTodo}>
            添加待办
          </button>
        )}
      </div>
      <input
        ref={fileRef}
        type="file"
        accept=".docx,.pdf,.png,.jpg,.jpeg,.gif,.webp"
        style={{ display: "none" }}
        onChange={handleFile}
      />

      {error && (
        <div className="status-line" style={{ marginTop: 12, color: "var(--red)" }}>
          {error}
        </div>
      )}
      {saved && (
        <div className="status-line" style={{ marginTop: 12 }}>
          <span className="sync-dot" /> {saved}
        </div>
      )}

      {result && (
        <div className="entry-result" style={{ marginTop: 14 }}>
          <div className="sum">{result.summary || "（无摘要）"}</div>
          {result.tags.length > 0 && (
            <div className="tag-row">
              {result.tags.map((t, i) => (
                <span key={i} className="tag">
                  {t}
                </span>
              ))}
            </div>
          )}
          {result.tasks.length > 0 && (
            <div>
              {result.tasks.map((t, i) => (
                <div key={i} className="task">
                  {t}
                </div>
              ))}
            </div>
          )}
          {result.outputs.length > 0 && (
            <div className="weak" style={{ fontSize: 12, marginTop: 6 }}>
              产出：{result.outputs.join("、")}
            </div>
          )}
          {result.flowcharts.map((f, i) => (
            <div key={i} className="flow">
              <div className="flow-label">流程图 · {f.title}</div>
              <FlowDiagram code={f.mermaid} />
            </div>
          ))}
          {result.todos.length > 0 && (
            <div className="weak" style={{ fontSize: 12, marginTop: 6 }}>
              待办：{result.todos.join("、")}
            </div>
          )}
          <div style={{ marginTop: 12, display: "flex", gap: 10 }}>
            <button className="btn btn-primary" onClick={confirm}>确认入库</button>
            <button className="btn" onClick={() => setResult(null)}>重新生成</button>
          </div>
        </div>
      )}
    </div>
  );
}
