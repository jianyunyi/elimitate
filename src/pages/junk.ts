import { cleanJunk, isTauri, onProgress, scanJunk } from "../api";
import { formatBytes, escapeHtml } from "../format";
import { confirmDialog, riskBadge, showBrowserHint, toast } from "../ui";
import type { JunkCategory } from "../types";

let categories: JunkCategory[] = [];
let scanning = false;
let cleaning = false;
let containerEl: HTMLElement | null = null;

export function renderJunk(container: HTMLElement): void {
  containerEl = container;
  container.innerHTML = `
    <div class="page-head">
      <h2>垃圾清理</h2>
      <p>扫描常见垃圾文件，按类别查看大小后选择性清理。</p>
    </div>
    <div class="toolbar">
      <button id="btn-scan" class="btn btn-primary">🔍 开始扫描</button>
      <button id="btn-clean" class="btn btn-danger" disabled>🗑️ 清理选中</button>
      <button id="btn-select-all" class="btn btn-ghost">全选</button>
      <button id="btn-select-none" class="btn btn-ghost">清空</button>
      <label class="inline-opt">
        仅清理
        <select id="age-filter">
          <option value="">全部</option>
          <option value="7">7 天前</option>
          <option value="30">30 天前</option>
          <option value="90">90 天前</option>
          <option value="180">180 天前</option>
        </select>
        修改的文件
      </label>
      <label class="inline-opt toggle-opt">
        <input type="checkbox" id="recycle-toggle" checked />
        🗑️ 删除到回收站（可恢复）
      </label>
    </div>
    <div class="progress-wrap" id="junk-progress" hidden>
      <div class="progress-track"><div class="progress-fill" id="junk-progress-fill" style="width:0%"></div></div>
      <div class="progress-label" id="junk-progress-label"></div>
    </div>
    <div class="junk-list" id="junk-list">
      <div class="empty-tip">点击「开始扫描」查看各类垃圾的大小</div>
    </div>
  `;

  const list = container.querySelector<HTMLElement>("#junk-list")!;
  const progressWrap = container.querySelector<HTMLElement>("#junk-progress")!;
  const fill = container.querySelector<HTMLElement>("#junk-progress-fill")!;
  const label = container.querySelector<HTMLElement>("#junk-progress-label")!;

  const ageFilter = () => {
    const v = container.querySelector<HTMLSelectElement>("#age-filter")!.value;
    return v === "" ? null : Number(v);
  };

  container.querySelector("#btn-scan")!.addEventListener("click", async () => {
    if (scanning) return;
    if (!isTauri()) return showBrowserHint();
    scanning = true;
    list.innerHTML = `<div class="empty-tip">正在扫描…</div>`;
    let un: (() => void) | null = null;
    try {
      un = await onProgress((p) => {
        progressWrap.hidden = false;
        const pct = p.total > 0 ? Math.round((p.done / p.total) * 100) : 0;
        fill.style.width = `${pct}%`;
        label.textContent = `扫描：${p.categoryName}（${p.done}/${p.total}）`;
      });
      categories = await scanJunk(ageFilter());
      renderList();
      progressWrap.hidden = true;
      const total = categories.reduce((a, c) => a + c.sizeBytes, 0);
      toast(`扫描完成，共发现 ${categories.length} 类垃圾，合计 ${formatBytes(total)}`, "success");
    } catch (e) {
      list.innerHTML = `<div class="empty-tip">扫描失败：${escapeHtml(String(e))}</div>`;
      toast(`扫描失败：${e}`, "error", 5000);
    } finally {
      un?.();
      scanning = false;
    }
  });

  container.querySelector("#btn-clean")!.addEventListener("click", async () => {
    if (cleaning) return;
    if (!isTauri()) return showBrowserHint();
    const ids = selectedIds();
    if (ids.length === 0) return;
    const total = categories.filter((c) => ids.includes(c.id)).reduce((a, c) => a + c.sizeBytes, 0);
    const toRecycleBin = container.querySelector<HTMLInputElement>("#recycle-toggle")!.checked;
    const days = ageFilter();
    const ageText = days ? `仅清理 ${days} 天前修改的文件。` : "";
    const ok = await confirmDialog(
      `确认清理选中的 ${ids.length} 个类别？`,
      `预计可释放 ${formatBytes(total)}。${ageText}\n${toRecycleBin ? "文件将移入回收站（可恢复）。" : "⚠️ 文件将被永久删除，不可恢复！"}`,
    );
    if (!ok) return;
    cleaning = true;
    setCleanBtn(true);
    let un: (() => void) | null = null;
    try {
      un = await onProgress((p) => {
        progressWrap.hidden = false;
        const pct = p.total > 0 ? Math.round((p.done / p.total) * 100) : 0;
        fill.style.width = `${pct}%`;
        label.textContent = `清理：${p.categoryName}（${p.done}/${p.total}）`;
      });
      const reports = await cleanJunk(ids, { maxAgeDays: days, toRecycleBin });
      const freed = reports.reduce((a, r) => a + r.bytesFreed, 0);
      const items = reports.reduce((a, r) => a + r.itemsRemoved, 0);
      const errors = reports.reduce((a, r) => a + r.errors.length, 0);
      const locked = reports.reduce((a, r) => a + r.locked, 0);
      let msg = `完成：清理 ${items} 项，释放 ${formatBytes(freed)}`;
      if (locked > 0) msg += `，${locked} 项被占用已跳过`;
      if (errors > 0) msg += `，${errors} 项失败`;
      label.textContent = msg;
      toast(msg, errors > 0 ? "error" : locked > 0 ? "info" : "success", 5000);
      // 重新扫描以刷新大小
      categories = await scanJunk(days);
      renderList();
      progressWrap.hidden = true;
    } catch (e) {
      toast(`清理失败：${e}`, "error", 5000);
    } finally {
      un?.();
      cleaning = false;
      setCleanBtn(false);
    }
  });

  container.querySelector("#btn-select-all")!.addEventListener("click", () => {
    for (const cb of container.querySelectorAll<HTMLInputElement>("input[type=checkbox]")) cb.checked = true;
    setCleanBtn(false);
  });
  container.querySelector("#btn-select-none")!.addEventListener("click", () => {
    for (const cb of container.querySelectorAll<HTMLInputElement>("input[type=checkbox]")) cb.checked = false;
    setCleanBtn(false);
  });
}

function selectedIds(): string[] {
  if (!containerEl) return [];
  return Array.from(containerEl.querySelectorAll<HTMLInputElement>("input[type=checkbox]:checked")).map(
    (cb) => cb.dataset.id!,
  );
}

function setCleanBtn(disabled: boolean): void {
  if (!containerEl) return;
  const btn = containerEl.querySelector<HTMLButtonElement>("#btn-clean")!;
  btn.disabled = disabled || selectedIds().length === 0;
  const ids = selectedIds();
  btn.textContent = ids.length > 0 ? `🗑️ 清理选中（${ids.length} 类）` : "🗑️ 清理选中";
}

function renderList(): void {
  const list = containerEl?.querySelector<HTMLElement>("#junk-list");
  if (!list) return;
  if (categories.length === 0) {
    list.innerHTML = `<div class="empty-tip">未发现可清理的垃圾，系统很干净 🎉</div>`;
    return;
  }
  list.innerHTML = categories
    .map((c) => {
      const paths = c.paths.map((p) => `<li title="${escapeHtml(p)}">${escapeHtml(p)}</li>`).join("");
      return `
      <div class="junk-item">
        <label class="junk-check">
          <input type="checkbox" data-id="${escapeHtml(c.id)}" />
        </label>
        <div class="junk-main">
          <div class="junk-name">
            ${escapeHtml(c.name)}
            ${riskBadge(c.risk)}
            ${c.requiresAdmin ? '<span class="badge badge-admin">需管理员</span>' : ""}
          </div>
          <div class="junk-desc">${escapeHtml(c.description)}</div>
          <details class="junk-paths">
            <summary>查看路径（${c.paths.length} 个位置）</summary>
            <ul>${paths}</ul>
          </details>
        </div>
        <div class="junk-meta">
          <div class="junk-size">${formatBytes(c.sizeBytes)}</div>
          <div class="junk-count muted">${c.fileCount.toLocaleString()} 项</div>
        </div>
      </div>`;
    })
    .join("");

  list.querySelectorAll<HTMLInputElement>("input[type=checkbox]").forEach((cb) => {
    cb.addEventListener("change", () => setCleanBtn(false));
  });
  setCleanBtn(false);
}
