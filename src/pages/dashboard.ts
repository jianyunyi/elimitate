import { cleanAll, isTauri, onProgress, relaunchAsAdmin, systemInfo } from "../api";
import { formatBytes, escapeHtml } from "../format";
import { icon } from "../icons";
import { showBrowserHint, toast } from "../ui";
import type { SystemInfo } from "../types";

export function renderDashboard(container: HTMLElement): void {
  container.innerHTML = `
    <div class="page-head">
      <h2>一键清理</h2>
      <p>扫描并清理常见垃圾文件，释放磁盘空间，提升系统性能。</p>
    </div>
    <div class="dashboard">
      <div class="card">
        <div class="overview-head">
          <div class="overview-title">${icon("drive", 16)}磁盘空间</div>
          <div class="overview-summary" id="ov-summary"></div>
        </div>
        <div class="overview-facts" id="ov-facts"></div>
        <div id="drive-list"><div class="muted">加载中…</div></div>
      </div>
      <div class="card">
        <div class="card-head">
          <h3>执行清理</h3>
        </div>
        <p class="hint">
          一键清理将按顺序扫描并清理：Windows 临时文件、浏览器缓存、回收站、Windows 更新缓存、
          缩略图缓存、系统日志、错误报告、预读取与传递优化缓存等全部类别。
        </p>
        <div id="clean-progress" class="progress-wrap" hidden>
          <div class="progress-track"><div class="progress-fill" id="clean-progress-fill"></div></div>
          <div class="progress-label" id="clean-progress-label"></div>
        </div>
        <div class="clean-cta">
          <button id="btn-clean-all" class="btn btn-primary btn-lg">${icon("sparkle", 18)}一键清理</button>
          <label class="inline-opt toggle-opt">
            <input type="checkbox" id="clean-all-recycle" checked />
            ${icon("trash", 14)}删除到回收站（可恢复）
          </label>
        </div>
      </div>
    </div>
  `;

  loadSystemInfo(container);
  container.querySelector("#btn-clean-all")!.addEventListener("click", () => runCleanAll(container));
}

async function loadSystemInfo(container: HTMLElement): Promise<void> {
  let info: SystemInfo;
  try {
    info = await systemInfo();
  } catch (e) {
    container.querySelector("#drive-list")!.innerHTML =
      `<div class="muted">获取系统信息失败：${escapeHtml(String(e))}</div>`;
    return;
  }
  const { osVersion, isAdmin, drives } = info;
  const totalFree = drives.reduce((a, d) => a + d.freeBytes, 0);
  const totalSize = drives.reduce((a, d) => a + d.totalBytes, 0);
  const freePct = totalSize > 0 ? Math.round((totalFree / totalSize) * 100) : 0;

  container.querySelector("#ov-summary")!.textContent = `可用 ${formatBytes(totalFree)} / ${formatBytes(totalSize)}（${freePct}%）`;
  container.querySelector("#ov-facts")!.innerHTML = `
    <span>系统 <b>${escapeHtml(osVersion)}</b></span>
    <span>磁盘 <b>${drives.length}</b> 个</span>
    <span>权限 ${isAdmin ? '<b class="badge badge-admin">管理员</b>' : '<b class="badge badge-warn">普通权限</b>'}</span>
  `;

  container.querySelector("#drive-list")!.innerHTML = drives
    .map(
      (d) => `
      <div class="drive-row">
        <div class="drive-label"><span class="drive-letter">${escapeHtml(d.letter)}</span>
          <span class="muted">${formatBytes(d.freeBytes)} / ${formatBytes(d.totalBytes)}</span></div>
        <div class="progress-track"><div class="progress-fill drive-fill" style="width:${d.totalBytes > 0 ? Math.round((d.freeBytes / d.totalBytes) * 100) : 0}%"></div></div>
      </div>`,
    )
    .join("");

  if (!isAdmin) {
    const cta = container.querySelector(".clean-cta")!;
    const hint = document.createElement("div");
    hint.className = "hint warn";
    hint.innerHTML = `当前以普通权限运行，部分系统目录（如 Windows\\Temp、更新缓存）无法清理。<button id="btn-relaunch-admin" class="btn btn-ghost btn-sm">以管理员身份重启</button>`;
    cta.prepend(hint);
    hint.querySelector("#btn-relaunch-admin")!.addEventListener("click", async () => {
      try {
        await relaunchAsAdmin();
        toast("正在请求管理员权限重启…", "info");
      } catch (e) {
        toast(`提权失败：${e}`, "error", 5000);
      }
    });
  }
}

async function runCleanAll(container: HTMLElement): Promise<void> {
  if (!isTauri()) return showBrowserHint();
  const btn = container.querySelector<HTMLButtonElement>("#btn-clean-all")!;
  const progressWrap = container.querySelector<HTMLElement>("#clean-progress")!;
  const fill = container.querySelector<HTMLElement>("#clean-progress-fill")!;
  const label = container.querySelector<HTMLElement>("#clean-progress-label")!;
  btn.disabled = true;

  let un: (() => void) | null = null;
  try {
    un = await onProgress((p) => {
      progressWrap.hidden = false;
      const pct = p.total > 0 ? Math.round((p.done / p.total) * 100) : 0;
      fill.style.transform = `scaleX(${pct / 100})`;
      label.textContent = `${p.phase === "scan" ? "扫描" : "清理"}：${p.categoryName}（${p.done}/${p.total}）`;
    });
  } catch (e) {
    btn.disabled = false;
    toast(`初始化失败：${e}`, "error", 5000);
    return;
  }

  try {
    const toRecycleBin = container.querySelector<HTMLInputElement>("#clean-all-recycle")!.checked;
    const reports = await cleanAll(toRecycleBin);
    const freed = reports.reduce((a, r) => a + r.bytesFreed, 0);
    const items = reports.reduce((a, r) => a + r.itemsRemoved, 0);
    const errors = reports.reduce((a, r) => a + r.errors.length, 0);
    const locked = reports.reduce((a, r) => a + r.locked, 0);
    let msg = `完成：共清理 ${items} 项，释放 ${formatBytes(freed)}`;
    if (locked > 0) msg += `，${locked} 项被占用已跳过`;
    if (errors > 0) msg += `，${errors} 项失败（可能需要更高权限）`;
    label.textContent = msg;
    toast(msg, errors > 0 ? "error" : locked > 0 ? "info" : "success", 5000);
    loadSystemInfo(container);
  } catch (e) {
    toast(`一键清理失败：${e}`, "error", 5000);
    label.textContent = "一键清理失败";
  } finally {
    un?.();
    btn.disabled = false;
  }
}
