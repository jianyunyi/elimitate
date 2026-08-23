// 轻量 UI 工具：toast、徽章、风险标签
import { escapeHtml } from "./format";

let toastRoot: HTMLDivElement | null = null;

function ensureToastRoot(): HTMLDivElement {
  if (!toastRoot || !document.body.contains(toastRoot)) {
    toastRoot = document.createElement("div");
    toastRoot.id = "toast-root";
    document.body.appendChild(toastRoot);
  }
  return toastRoot;
}

export type ToastKind = "info" | "success" | "error";

export function toast(message: string, kind: ToastKind = "info", timeoutMs = 3500): void {
  const root = ensureToastRoot();
  const el = document.createElement("div");
  el.className = `toast toast-${kind}`;
  el.textContent = message;
  root.appendChild(el);
  setTimeout(() => {
    el.classList.add("toast-out");
    setTimeout(() => el.remove(), 300);
  }, timeoutMs);
}

export function riskBadge(risk: "low" | "medium" | "high"): string {
  const labels: Record<string, string> = { low: "低风险", medium: "中风险", high: "高风险" };
  return `<span class="badge badge-${risk}">${labels[risk] ?? risk}</span>`;
}

export function kindLabel(kind: string): string {
  const map: Record<string, string> = {
    file: "文件",
    dir: "文件夹",
    registry_key: "注册表项",
    registry_value: "注册表值",
    shortcut: "快捷方式",
  };
  return map[kind] ?? kind;
}

export function riskClass(risk: "low" | "medium" | "high"): string {
  return `badge-${risk}`;
}

// 简单确认对话框
export function confirmDialog(message: string, detail = ""): Promise<boolean> {
  return new Promise((resolve) => {
    const overlay = document.createElement("div");
    overlay.className = "modal-overlay";
    overlay.innerHTML = `
      <div class="modal">
        <h3>${escapeHtml(message)}</h3>
        ${detail ? `<p class="modal-detail">${escapeHtml(detail)}</p>` : ""}
        <div class="modal-actions">
          <button class="btn btn-ghost" data-act="cancel">取消</button>
          <button class="btn btn-danger" data-act="ok">确定</button>
        </div>
      </div>`;
    document.body.appendChild(overlay);
    const close = (v: boolean) => {
      overlay.remove();
      resolve(v);
    };
    overlay.querySelector("[data-act=cancel]")!.addEventListener("click", () => close(false));
    overlay.querySelector("[data-act=ok]")!.addEventListener("click", () => close(true));
    overlay.addEventListener("click", (e) => {
      if (e.target === overlay) close(false);
    });
  });
}

/** 非 Tauri 环境（浏览器预览）时的操作提示 */
export function showBrowserHint(): void {
  toast(
    "当前不在 Elimitate 桌面应用中运行（浏览器预览模式），清理功能不可用。请运行 `pnpm tauri dev` 或使用安装版 Elimitate。",
    "error",
    6000,
  );
}
