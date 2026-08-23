// 与 Rust 后端序列化结构一一对应（后端使用 camelCase 重命名）

export type Risk = "low" | "medium" | "high";

export interface JunkCategory {
  id: string;
  name: string;
  description: string;
  paths: string[];
  fileCount: number;
  sizeBytes: number;
  risk: Risk;
  requiresAdmin: boolean;
}

export interface CleanReport {
  categoryId: string;
  categoryName: string;
  itemsRemoved: number;
  bytesFreed: number;
  errors: string[];
}

export interface InstalledApp {
  key: string;
  name: string;
  version: string;
  publisher: string;
  installLocation: string;
  uninstallString: string;
  displayIcon: string;
  estimatedSizeKb: number;
  installDate: string;
  isUser: boolean;
  systemComponent: boolean;
}

export type ResidueKind = "file" | "dir" | "registry_key" | "registry_value" | "shortcut";

export interface ResidueItem {
  path: string;
  kind: ResidueKind;
  sizeBytes: number;
  risk: Risk;
  note: string;
}

export interface ResidueReport {
  appKey: string;
  name: string;
  items: ResidueItem[];
  totalSizeBytes: number;
}

export interface DeleteReport {
  deleted: number;
  failed: number;
  bytesFreed: number;
  errors: string[];
}

export interface UninstallResult {
  launched: boolean;
  message: string;
}

export interface DriveInfo {
  letter: string;
  totalBytes: number;
  freeBytes: number;
}

export interface SystemInfo {
  osVersion: string;
  isAdmin: boolean;
  drives: DriveInfo[];
}

export interface ProgressEvent {
  categoryId: string;
  categoryName: string;
  phase: "scan" | "clean";
  done: number;
  total: number;
}
