import {
  cancelLargeScan,
  deletePaths,
  isTauri,
  onLargeProgress,
  openInExplorer,
  precheckLocked,
  scanLargeFiles,
  systemInfo,
} from "../api";
import { formatBytes, escapeHtml } from "../format";
import { icon, skeletonRows } from "../icons";
import { confirmDialog, showBrowserHint, toast } from "../ui";
import type { LargeFileItem, SystemInfo } from "../types";

let containerEl: HTMLElement | null = null;
let scanning = false;
let unlisten: (() => void) | null = null;

export function renderLargeFiles(container: HTMLElement): void {
  containerEl = container;
  container.innerHTML = `
    <div class="page-head">
      <div class="page-title-row">
        <span class="page-icon">${icon("folder", 18)}</span>
        <h2>大文件分析</h2>
      </div>
      <p>扫描磁盘中占用空间最大的文件，由你亲自判断是否删除。</p>
    </div>
    <div class="toolbar">
      <label class="inline-opt">
        磁盘
        <select id="lf-drive"></select>
      </label>
      <label class="inline-opt">
        数量
        <select id="lf-top">
          <option value="50">前 50</option>
          <option value="100" selected>前 100</option>
          <option value="200">前 200</option>
        </select>
      </label>
      <label class="inline-opt toggle-opt">
        <input type="checkbox" id="lf-skip-system" checked />
        跳过系统目录（Windows / Program Files 等）
      </label>
      <button id="lf-scan" class="btn btn-primary">${icon("search", 15)}开始扫描</button>
      <button id="lf-cancel" class="btn btn-ghost" disabled>${icon("stop", 15)}停止</button>
      <label class="inline-opt toggle-opt">
        <input type="checkbox" id="lf-recycle" checked />
        ${icon("trash", 14)}删除到回收站（可恢复）
      </label>
      <button id="lf-open" class="btn btn-ghost" disabled>${icon("folder", 15)}打开所在文件夹</button>
      <button id="lf-delete" class="btn btn-danger" disabled>${icon("trash", 15)}删除选中</button>
    </div>
    <div class="progress-wrap" id="lf-progress" hidden>
      <div class="progress-track"><div class="progress-fill static" id="lf-progress-fill" style="width:30%"></div></div>
      <div class="progress-label" id="lf-progress-label"></div>
    </div>
    <div class="card lf-card">
      <div class="lf-table-wrap" id="lf-table-wrap">
        <div class="empty-tip">选择磁盘后点击「开始扫描」。全盘扫描可能需要几分钟，可随时停止。</div>
      </div>
    </div>
  `;

  // 填充磁盘下拉
  systemInfo()
    .then((info: SystemInfo) => {
      const sel = container.querySelector<HTMLSelectElement>("#lf-drive")!;
      if (!info.drives || info.drives.length === 0) {
        sel.innerHTML = `<option value="">未检测到磁盘</option>`;
        return;
      }
      sel.innerHTML = info.drives
        .map((d) => `<option value="${escapeHtml(d.letter.replace(":", ""))}">${escapeHtml(d.letter)}（可用 ${formatBytes(d.freeBytes)}）</option>`)
        .join("");
    })
    .catch((e) => {
      const sel = container.querySelector<HTMLSelectElement>("#lf-drive")!;
      sel.innerHTML = `<option value="">磁盘信息获取失败</option>`;
      toast(`获取磁盘信息失败：${e}`, "error", 5000);
    });

  container.querySelector("#lf-scan")!.addEventListener("click", () => startScan(container));
  container.querySelector("#lf-cancel")!.addEventListener("click", async () => {
    await cancelLargeScan();
  });
  container.querySelector("#lf-open")!.addEventListener("click", () => {
    const paths = selectedPaths(container);
    if (paths.length === 1) openInExplorer(paths[0]).catch((e) => toast(`打开失败：${e}`, "error", 4000));
  });
  container.querySelector("#lf-delete")!.addEventListener("click", () => deleteSelected(container));
}

function selectedPaths(container: HTMLElement): string[] {
  return Array.from(container.querySelectorAll<HTMLInputElement>(".lf-check:checked")).map(
    (cb) => cb.dataset.path!,
  );
}

function updateActions(container: HTMLElement): void {
  const n = selectedPaths(container).length;
  const open = container.querySelector<HTMLButtonElement>("#lf-open")!;
  const del = container.querySelector<HTMLButtonElement>("#lf-delete")!;
  open.disabled = n !== 1;
  del.disabled = n === 0;
  del.innerHTML = `${icon("trash", 15)}${n > 0 ? `删除选中（${n}）` : "删除选中"}`;
}

async function startScan(container: HTMLElement): Promise<void> {
  if (scanning) return;
  if (!isTauri()) return showBrowserHint();
  // 未选择磁盘（下拉为空/加载失败）时直接提示，不再调用后端
  const drive = container.querySelector<HTMLSelectElement>("#lf-drive")!.value;
  if (!drive) {
    toast("请先在磁盘列表中选择要扫描的磁盘", "error", 4000);
    return;
  }
  scanning = true;
  const scanBtn = container.querySelector<HTMLButtonElement>("#lf-scan")!;
  const cancelBtn = container.querySelector<HTMLButtonElement>("#lf-cancel")!;
  const progressWrap = container.querySelector<HTMLElement>("#lf-progress")!;
  const fill = container.querySelector<HTMLElement>("#lf-progress-fill")!;
  const label = container.querySelector<HTMLElement>("#lf-progress-label")!;
  const wrap = container.querySelector<HTMLElement>("#lf-table-wrap")!;
  scanBtn.disabled = true;
  cancelBtn.disabled = false;
  wrap.innerHTML = skeletonRows(4);

  try {
    unlisten?.();
    unlisten = await onLargeProgress((p) => {
      progressWrap.hidden = false;
      label.textContent = `已扫描 ${p.scanned.toLocaleString()} 个文件，用时 ${(p.elapsedMs / 1000).toFixed(1)}s`;
    });

    const top = Number(container.querySelector<HTMLSelectElement>("#lf-top")!.value);
    const skipSystem = container.querySelector<HTMLInputElement>("#lf-skip-system")!.checked;
    const report = await scanLargeFiles(drive, top, skipSystem);
    renderResults(container, report.items);
    label.textContent = `${report.cancelled ? "已停止" : "扫描完成"}：共扫描 ${report.scannedFiles.toLocaleString()} 个文件，用时 ${(report.elapsedMs / 1000).toFixed(1)}s${report.cancelled ? "（显示已发现的部分结果）" : ""}`;
    toast(
      report.cancelled ? `已停止扫描，发现 ${report.items.length} 个大文件` : `扫描完成，发现 ${report.items.length} 个大文件`,
      "success",
    );
  } catch (e) {
    wrap.innerHTML = `<div class="empty-tip error-text">扫描失败：${escapeHtml(String(e))}</div>`;
    toast(`扫描失败：${e}`, "error", 5000);
  } finally {
    progressWrap.hidden = true;
    scanning = false;
    scanBtn.disabled = false;
    cancelBtn.disabled = true;
  }
}

function renderResults(container: HTMLElement, items: LargeFileItem[]): void {
  const wrap = container.querySelector<HTMLElement>("#lf-table-wrap")!;
  if (items.length === 0) {
    wrap.innerHTML = `<div class="empty-tip"><span class="empty-icon success">${icon("check", 20)}</span>未发现大文件</div>`;
    return;
  }
  const rows = items
    .map((f, idx) => {
      const mod = f.modified > 0 ? new Date(f.modified).toLocaleDateString() : "未知";
      // 大小分档（数字始终可见，颜色为辅助编码）
      const tier = f.sizeBytes >= 1073741824 ? "huge" : f.sizeBytes >= 209715200 ? "big" : "";
      return `
      <label class="lf-row">
        <input type="checkbox" class="lf-check" data-path="${escapeHtml(f.path)}" />
        <span class="lf-rank">#${idx + 1}</span>
        <span class="lf-size ${tier}">${formatBytes(f.sizeBytes)}</span>
        <span class="lf-name" title="${escapeHtml(f.path)}">${escapeHtml(f.path)}</span>
        <span class="lf-mod muted">${mod}</span>
      </label>`;
    })
    .join("");
  wrap.innerHTML = `
    <div class="lf-head">
      <span class="lf-rank">排名</span><span class="lf-size">大小</span><span class="lf-name">文件路径</span><span class="lf-mod">修改日期</span>
    </div>
    ${rows}
  `;
  wrap.querySelectorAll<HTMLInputElement>(".lf-check").forEach((cb) => {
    cb.addEventListener("change", () => updateActions(container));
  });
  updateActions(container);
}

async function deleteSelected(container: HTMLElement): Promise<void> {
  if (!isTauri()) return showBrowserHint();
  const paths = selectedPaths(container);
  if (paths.length === 0) return;
  const toRecycleBin = container.querySelector<HTMLInputElement>("#lf-recycle")!.checked;

  // 占用预检：删除前先提示哪些会被跳过
  let lockedNote = "";
  try {
    const locked = await precheckLocked(paths);
    if (locked.length > 0) {
      lockedNote = `\n\n⚠️ 其中 ${locked.length} 项当前被占用，将自动跳过：\n${locked.slice(0, 5).map((p) => `· ${p}`).join("\n")}${locked.length > 5 ? `\n…等 ${locked.length} 项` : ""}`;
    }
  } catch {
    /* 预检失败不阻塞删除 */
  }

  const ok = await confirmDialog(
    `确认删除选中的 ${paths.length} 个大文件？`,
    `${toRecycleBin ? "文件将移入回收站（可恢复）。" : "⚠️ 文件将被永久删除，不可恢复！"}${lockedNote}`,
  );
  if (!ok) return;
  try {
    const r = await deletePaths(paths, toRecycleBin);
    let msg = `删除完成：成功 ${r.deleted} 项，失败 ${r.failed} 项，释放 ${formatBytes(r.bytesFreed)}`;
    if (r.locked > 0) msg += `，${r.locked} 项被占用已跳过`;
    toast(msg, r.failed > 0 ? "error" : r.locked > 0 ? "info" : "success", 5000);
    // 从列表中移除已删除项
    const wrap = container.querySelector<HTMLElement>("#lf-table-wrap")!;
    for (const p of paths) {
      wrap.querySelectorAll<HTMLElement>(".lf-row").forEach((row) => {
        if (row.querySelector<HTMLInputElement>(".lf-check")?.dataset.path === p) row.remove();
      });
    }
    updateActions(container);
  } catch (e) {
    toast(`删除失败：${e}`, "error", 5000);
  }
}
