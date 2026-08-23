import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  CleanReport,
  DeleteReport,
  InstalledApp,
  JunkCategory,
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
export const scanJunk = () => invoke<JunkCategory[]>("scan_junk");

export const cleanJunk = (ids: string[]) => invoke<CleanReport[]>("clean_junk", { ids });

// 一键清理：全部垃圾 + 回收站
export const cleanAll = () => invoke<CleanReport[]>("clean_all");

// ---------- 软件卸载 ----------
export const listInstalledApps = () => invoke<InstalledApp[]>("list_installed_apps");

export const scanResidue = (appKey: string) =>
  invoke<ResidueReport>("scan_residue", { appKey });

export const deleteResidue = (appKey: string, items: ResidueItem[]) =>
  invoke<DeleteReport>("delete_residue", { appKey, items });

export const uninstallApp = (appKey: string) =>
  invoke<UninstallResult>("uninstall_app", { appKey });

// ---------- 进度事件 ----------
export async function onProgress(handler: (p: ProgressEvent) => void): Promise<UnlistenFn> {
  return listen<ProgressEvent>("progress", (e) => handler(e.payload));
}
