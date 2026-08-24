import {
  deleteResidue,
  isTauri,
  listInstalledApps,
  scanResidue,
  uninstallApp,
} from "../api";
import { formatBytes, escapeHtml } from "../format";
import { icon, skeletonRows } from "../icons";
import { confirmDialog, kindLabel, riskBadge, showBrowserHint, toast } from "../ui";
import type { InstalledApp, ResidueItem } from "../types";

let apps: InstalledApp[] = [];
let selectedKey: string | null = null;
let residue: ResidueItem[] = [];
let containerEl: HTMLElement | null = null;

export function renderUninstall(container: HTMLElement): void {
  containerEl = container;
  container.innerHTML = `
    <div class="page-head">
      <div class="page-title-row">
        <span class="page-icon">${icon("package", 18)}</span>
        <h2>软件卸载</h2>
      </div>
      <p>运行官方卸载程序，再扫描删除卸载后残留的文件与注册表项，实现彻底卸载。</p>
    </div>
    <div class="uninstall-layout">
      <div class="card app-list-card">
        <div class="search-wrap">
          ${icon("search", 14)}
          <input id="app-search" class="search-input" placeholder="搜索软件名称…" />
        </div>
        <div class="app-list" id="app-list">${skeletonRows(6)}</div>
      </div>
      <div class="card app-detail" id="app-detail">
        <div class="empty-tip"><span class="empty-icon">${icon("package", 20)}</span>从左侧选择一个软件</div>
      </div>
    </div>
  `;

  loadApps(container);
  container.querySelector("#app-search")!.addEventListener("input", (e) => {
    renderAppList((e.target as HTMLInputElement).value);
  });
}

async function loadApps(container: HTMLElement): Promise<void> {
  if (!isTauri()) {
    container.querySelector("#app-list")!.innerHTML = `<div class="empty-tip">浏览器预览模式：请通过 Elimitate 桌面应用使用</div>`;
    return;
  }
  try {
    apps = await listInstalledApps();
    renderAppList("");
    toast(`已加载 ${apps.length} 个软件`, "info", 2000);
  } catch (e) {
    container.querySelector("#app-list")!.innerHTML =
      `<div class="empty-tip">加载软件列表失败：${escapeHtml(String(e))}</div>`;
  }
}

function renderAppList(filter: string): void {
  const list = containerEl?.querySelector<HTMLElement>("#app-list");
  if (!list) return;
  const q = filter.trim().toLowerCase();
  const filtered = q ? apps.filter((a) => a.name.toLowerCase().includes(q) || (a.publisher ?? "").toLowerCase().includes(q)) : apps;
  if (filtered.length === 0) {
    list.innerHTML = `<div class="empty-tip">没有匹配的软件</div>`;
    return;
  }
  list.innerHTML = filtered
    .map((a) => {
      const size = a.estimatedSizeKb > 0 ? formatBytes(a.estimatedSizeKb * 1024) : "";
      const cls = selectedKey === a.key ? "active" : "";
      const meta = [a.version, size, a.installDate].filter(Boolean).join(" · ");
      return `
      <div class="app-item ${cls}" data-key="${escapeHtml(a.key)}">
        <div class="app-item-name">${escapeHtml(a.name)}</div>
        <div class="app-item-meta muted">${escapeHtml(meta || "—")}</div>
      </div>`;
    })
    .join("");

  list.querySelectorAll<HTMLElement>(".app-item").forEach((el) => {
    el.addEventListener("click", () => {
      selectedKey = el.dataset.key!;
      renderAppList((containerEl!.querySelector<HTMLInputElement>("#app-search")!.value));
      renderDetail(selectedKey!);
    });
  });
}

function renderDetail(key: string): void {
  const detail = containerEl?.querySelector<HTMLElement>("#app-detail");
  if (!detail) return;
  const app = apps.find((a) => a.key === key);
  if (!app) return;
  residue = [];
  detail.innerHTML = `
    <div class="detail-head">
      <h3>${escapeHtml(app.name)}</h3>
      <div class="detail-meta">
        ${app.publisher ? `<div><span class="muted">发布者</span><br><b>${escapeHtml(app.publisher)}</b></div>` : ""}
        ${app.version ? `<div><span class="muted">版本</span><br><b>${escapeHtml(app.version)}</b></div>` : ""}
        ${app.estimatedSizeKb > 0 ? `<div><span class="muted">已用空间</span><br><b>${formatBytes(app.estimatedSizeKb * 1024)}</b></div>` : ""}
        ${app.installDate ? `<div><span class="muted">安装日期</span><br><b>${escapeHtml(app.installDate)}</b></div>` : ""}
        ${app.isUser ? '<div><span class="muted">安装于</span><br><b>当前用户</b></div>' : '<div><span class="muted">安装于</span><br><b>所有用户</b></div>'}
      </div>
      ${app.installLocation ? `<div class="detail-loc muted" title="${escapeHtml(app.installLocation)}">安装位置：${escapeHtml(app.installLocation)}</div>` : ""}
    </div>
    <div class="detail-actions">
      <button id="btn-uninstall" class="btn btn-danger" ${app.uninstallString ? "" : "disabled"}>${icon("trash", 15)}卸载</button>
      <button id="btn-scan-residue" class="btn btn-primary">${icon("search", 15)}扫描残留</button>
    </div>
    <div id="residue-area">
      <div class="empty-tip">卸载软件后，点击「扫描残留」查找遗留的文件与注册表项</div>
    </div>
  `;

  detail.querySelector("#btn-uninstall")!.addEventListener("click", async () => {
    if (!isTauri()) return showBrowserHint();
    const ok = await confirmDialog(
      `确认卸载「${app.name}」？`,
      "将启动官方卸载程序（UninstallString），卸载过程由软件自身完成。卸载完成后建议点击「扫描残留」清除遗留文件。",
    );
    if (!ok) return;
    try {
      const r = await uninstallApp(app.key);
      if (r.launched) {
        toast(`已启动「${app.name}」的卸载程序，请在弹出的窗口中完成卸载`, "info", 6000);
      } else {
        toast(r.message || "无法启动卸载程序", "error", 5000);
      }
    } catch (e) {
      toast(`启动卸载失败：${e}`, "error", 5000);
    }
  });

  detail.querySelector("#btn-scan-residue")!.addEventListener("click", () => {
    if (!isTauri()) return showBrowserHint();
    scanResidueFor(app);
  });
}

async function scanResidueFor(app: InstalledApp): Promise<void> {
  const detail = containerEl?.querySelector<HTMLElement>("#app-detail");
  const area = detail?.querySelector<HTMLElement>("#residue-area");
  if (!area) return;
  area.innerHTML = `<div class="empty-tip">正在扫描残留（文件系统 + 注册表）…</div>`;
  try {
    const report = await scanResidue(app.key);
    residue = report.items;
    renderResidue(report.name, report.totalSizeBytes);
  } catch (e) {
    area.innerHTML = `<div class="empty-tip error-text">残留扫描失败：${escapeHtml(String(e))}</div>`;
  }
}

function renderResidue(name: string, totalSize: number): void {
  const detail = containerEl?.querySelector<HTMLElement>("#app-detail");
  const area = detail?.querySelector<HTMLElement>("#residue-area");
  if (!area) return;

  const groupByKind = (kind: string) => residue.filter((r) => r.kind === kind);
  const groups: [string, ResidueItem[]][] = [
    ["dir", groupByKind("dir")],
    ["file", groupByKind("file")],
    ["shortcut", groupByKind("shortcut")],
    ["registry_key", groupByKind("registry_key")],
    ["registry_value", groupByKind("registry_value")],
  ].filter(([, items]) => items.length > 0) as [string, ResidueItem[]][];

  if (residue.length === 0) {
    area.innerHTML = `<div class="empty-tip"><span class="empty-icon success">${icon("check", 20)}</span>未发现「${escapeHtml(name)}」的残留，卸载得很干净</div>`;
    return;
  }

  const listHtml = groups
    .map(([kind, items]) => {
      const rows = items
        .map(
          (r) => `
          <label class="residue-row">
            <input type="checkbox" class="residue-check" data-path="${escapeHtml(r.path)}" ${r.risk === "high" ? 'data-high="1"' : ""} />
            <span class="residue-kind badge badge-kind-${r.kind}">${kindLabel(r.kind)}</span>
            <span class="residue-path" title="${escapeHtml(r.path)}">${escapeHtml(r.path)}</span>
            ${r.risk !== "low" ? riskBadge(r.risk) : ""}
            ${r.sizeBytes > 0 ? `<span class="residue-size">${formatBytes(r.sizeBytes)}</span>` : ""}
          </label>`,
        )
        .join("");
      const sub = items.reduce((a, r) => a + r.sizeBytes, 0);
      return `
        <div class="residue-group">
          <div class="residue-group-head">
            <span>${kindLabel(kind)}</span><span class="muted">${items.length} 项${sub > 0 ? ` · ${formatBytes(sub)}` : ""}</span>
          </div>
          ${rows}
        </div>`;
    })
    .join("");

  const highCount = residue.filter((r) => r.risk === "high").length;

  area.innerHTML = `
    <div class="residue-summary">
      <span>共发现 <b>${residue.length}</b> 项残留${totalSize > 0 ? `，文件类合计 <b>${formatBytes(totalSize)}</b>` : ""}</span>
      ${highCount > 0 ? `<span class="badge badge-high">${highCount} 项高风险（多为注册表，谨慎删除）</span>` : ""}
      <div class="residue-actions">
        <button id="btn-res-select-all" class="btn btn-ghost btn-sm">全选</button>
        <button id="btn-res-select-safe" class="btn btn-ghost btn-sm">仅选低/中风险</button>
        <label class="inline-opt toggle-opt btn-sm">
          <input type="checkbox" id="residue-recycle" checked />
          ${icon("trash", 13)}进回收站
        </label>
        <button id="btn-res-delete" class="btn btn-danger btn-sm">${icon("trash", 13)}删除选中</button>
      </div>
    </div>
    ${listHtml}
  `;

  area.querySelector("#btn-res-select-all")!.addEventListener("click", () => {
    area.querySelectorAll<HTMLInputElement>(".residue-check").forEach((cb) => (cb.checked = true));
  });
  area.querySelector("#btn-res-select-safe")!.addEventListener("click", () => {
    area.querySelectorAll<HTMLInputElement>(".residue-check").forEach((cb) => (cb.checked = !cb.dataset.high));
  });
  area.querySelector("#btn-res-delete")!.addEventListener("click", async () => {
    if (!isTauri()) return showBrowserHint();
    const checkedPaths = Array.from(area.querySelectorAll<HTMLInputElement>(".residue-check:checked")).map(
      (cb) => cb.dataset.path!,
    );
    const checkedItems = residue.filter((r) => checkedPaths.includes(r.path));
    if (checkedItems.length === 0) return;
    const highChecked = checkedItems.filter((r) => r.risk === "high").length;
    const toRecycleBin = area.querySelector<HTMLInputElement>("#residue-recycle")!.checked;
    const warn = highChecked > 0 ? `\n\n⚠️ 其中包含 ${highChecked} 项高风险项（可能被其他软件共用），请确认无误。` : "";
    const ok = await confirmDialog(
      `确认删除选中的 ${checkedItems.length} 项残留？`,
      `${toRecycleBin ? "文件类将移入回收站（可恢复）；注册表项直接删除。" : "⚠️ 文件将被永久删除，不可恢复！注册表项直接删除。"}${warn}`,
    );
    if (!ok) return;
    try {
      const r = await deleteResidue(selectedKey!, checkedItems, toRecycleBin);
      let msg = `删除完成：成功 ${r.deleted} 项，失败 ${r.failed} 项，释放 ${formatBytes(r.bytesFreed)}`;
      if (r.locked > 0) msg += `，${r.locked} 项被占用已跳过`;
      toast(msg, r.failed > 0 ? "error" : r.locked > 0 ? "info" : "success", 5000);
      residue = residue.filter((item) => !checkedPaths.includes(item.path));
      renderResidue(name, residue.filter((i) => i.sizeBytes > 0).reduce((a, i) => a + i.sizeBytes, 0));
      if (r.errors.length > 0) {
        console.warn("删除残留失败项：", r.errors);
      }
    } catch (e) {
      toast(`删除失败：${e}`, "error", 5000);
    }
  });
}
