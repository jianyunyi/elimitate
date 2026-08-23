import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  CleanReport,
  DeleteReport,
  InstalledApp,
  JunkCategory,
  LargeFileReport,
  LargeProgressEvent,
  ProgressEvent,
  ResidueItem,
  ResidueReport,
  SystemInfo,
  UninstallResult,
} from "./types";

// ---------- 系统 ----------
export const systemInfo = () => invoke<SystemInfo>("system_info");

export const relaunchAsAdmin = () =>
  invoke<void>("relaunch_as_admin").catch((e) => {
    throw new Error(String(e));
  });

// ---------- 垃圾清理 ----------
/** maxAgeDays：仅统计/清理 N 天前修改的文件；null = 全部 */
export const scanJunk = (maxAgeDays: number | null = null) =>
  invoke<JunkCategory[]>("scan_junk", { maxAgeDays });

export const cleanJunk = (
  ids: string[],
  opts: { maxAgeDays?: number | null; toRecycleBin: boolean },
) =>
  invoke<CleanReport[]>("clean_junk", {
    ids,
    maxAgeDays: opts.maxAgeDays ?? null,
    toRecycleBin: opts.toRecycleBin,
  });

// 一键清理：全部垃圾 + 回收站
export const cleanAll = (toRecycleBin: boolean) =>
  invoke<CleanReport[]>("clean_all", { toRecycleBin });

// ---------- 软件卸载 ----------
export const listInstalledApps = () => invoke<InstalledApp[]>("list_installed_apps");

export const scanResidue = (appKey: string) =>
  invoke<ResidueReport>("scan_residue", { appKey });

export const deleteResidue = (appKey: string, items: ResidueItem[], toRecycleBin: boolean) =>
  invoke<DeleteReport>("delete_residue", { appKey, items, toRecycleBin });

export const uninstallApp = (appKey: string) =>
  invoke<UninstallResult>("uninstall_app", { appKey });

// ---------- 大文件分析 ----------
export const scanLargeFiles = (drive: string, top = 100, skipSystem = true) =>
  invoke<LargeFileReport>("scan_large_files", { drive, top, skipSystem });

export const cancelLargeScan = () => invoke<void>("cancel_large_scan");

export const openInExplorer = (path: string) => invoke<void>("open_in_explorer", { path });

export const deletePaths = (paths: string[], toRecycleBin: boolean) =>
  invoke<DeleteReport>("delete_paths", { paths, toRecycleBin });

/** 删除前预检：返回被占用的路径列表 */
export const precheckLocked = (paths: string[]) => invoke<string[]>("precheck_locked", { paths });

// ---------- 进度事件 ----------
export async function onProgress(handler: (p: ProgressEvent) => void): Promise<UnlistenFn> {
  return listen<ProgressEvent>("progress", (e) => handler(e.payload));
}

export async function onLargeProgress(handler: (p: LargeProgressEvent) => void): Promise<UnlistenFn> {
  return listen<LargeProgressEvent>("large-progress", (e) => handler(e.payload));
}
