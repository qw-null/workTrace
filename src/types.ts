// 与 Rust 后端对齐的数据类型

export interface Flowchart {
  title: string;
  mermaid: string;
}

export interface Structured {
  summary: string;
  tasks: string[];
  tags: string[];
  outputs: string[];
  flowcharts: Flowchart[];
  todos: string[];
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
