import { isTauri, systemInfo } from "./api";
import { icon } from "./icons";
import { renderDashboard } from "./pages/dashboard";
import { renderJunk } from "./pages/junk";
import { renderLargeFiles } from "./pages/largefiles";
import { renderUninstall } from "./pages/uninstall";
import "./style.css";

const app = document.getElementById("app")!;

app.innerHTML = `
  <div class="app-shell">
    <aside class="sidebar">
      <div class="logo">
        <span class="logo-icon">${icon("sparkle", 20)}</span>
        <div class="logo-text">
          <b>Elimitate</b>
          <span class="muted">清理助手</span>
        </div>
      </div>
      <nav class="nav">
        <button class="nav-item active" data-page="dashboard">${icon("sparkle", 16)}<span>一键清理</span></button>
        <button class="nav-item" data-page="junk">${icon("trash", 16)}<span>垃圾清理</span></button>
        <button class="nav-item" data-page="largefiles">${icon("folder", 16)}<span>大文件分析</span></button>
        <button class="nav-item" data-page="uninstall">${icon("package", 16)}<span>软件卸载</span></button>
      </nav>
      <div class="sidebar-foot">
        <span id="admin-badge" class="badge">…</span>
        <span class="version muted">v0.2.3</span>
      </div>
    </aside>
    <main class="content" id="content"></main>
  </div>
`;

const pages: Record<string, (el: HTMLElement) => void> = {
  dashboard: renderDashboard,
  junk: renderJunk,
  largefiles: renderLargeFiles,
  uninstall: renderUninstall,
};

function navigate(page: string): void {
  document
    .querySelectorAll(".nav-item")
    .forEach((b) => b.classList.toggle("active", b.getAttribute("data-page") === page));
  const content = document.getElementById("content")!;
  content.innerHTML = "";
  pages[page]?.(content);
}

document.querySelectorAll(".nav-item").forEach((btn) => {
  btn.addEventListener("click", () => navigate(btn.getAttribute("data-page")!));
});

// 侧边栏权限徽章
systemInfo()
  .then((info) => {
    const badge = document.getElementById("admin-badge")!;
    badge.textContent = info.isAdmin ? "管理员权限" : "普通权限";
    badge.classList.add(info.isAdmin ? "badge-admin" : "badge-warn");
  })
  .catch(() => {});

// 浏览器预览模式提示（非 Tauri 环境）
if (!isTauri()) {
  const banner = document.createElement("div");
  banner.className = "browser-banner";
  banner.innerHTML =
    `⚠️ 当前以「浏览器预览」模式运行，清理/卸载功能不可用。` +
    `请运行 <code>pnpm tauri dev</code> 或下载安装版 Elimitate（GitHub Releases）。`;
  document.body.prepend(banner);
}

navigate("dashboard");
