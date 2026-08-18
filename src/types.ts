// 与 Rust 后端对齐的数据类型

export interface Flowchart {
  title: string;
  mermaid: string;
}

// 一条工作记录（记录模式）
export interface RecordField {
  time: string;
  content: string;
  progress: string;
  people: string;
  next: string;
}

// 一条待办事项（待办模式）
export interface TodoField {
  timeLocation: string;
  item: string;
  note: string;
}

export interface Structured {
  // 旧字段（历史数据兼容）
  summary: string;
  tasks: string[];
  tags: string[];
  outputs: string[];
  flowcharts: Flowchart[];
  todos: string[];
  // 新字段（新提示词产出）
  records: RecordField[];
  todoItems: TodoField[];
}

export type EntryStatus = "pending" | "confirmed" | "edited";

export interface RecordEntry {
  id: string;
  kind: string; // record | todo
  createdAt: string;
  updatedAt: string;
  rawText: string;
  sourceAttachments: string[];
  modelUsed: string;
  status: EntryStatus;
  structured: Structured;
}

export interface DayRecord {
  version: number;
  date: string;
  entries: RecordEntry[];
}

export interface DayActive {
  date: string;
  count: number;
  todoCount: number;
}

export interface Report {
  weekStart: string;
  weekEnd: string;
  content: string;
  generatedAt: string;
  modelUsed: string;
}

export interface ModelConfig {
  id: string;
  name: string;
  baseUrl: string;
  model: string;
  apiKey: string;
  role: string; // record | report | vision
  isDefault: boolean;
}

export interface WebdavConfig {
  id: string;
  name: string;
  provider: string; // jianguoyun | infinicloud | other
  url: string;
  account: string;
  password: string;
  isDefault: boolean;
}

export interface BackupSettings {
  syncIntervalMin: number;
  attachmentRetentionDays: number;
}
