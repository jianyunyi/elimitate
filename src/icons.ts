// 统一 SVG 图标系统（24 viewBox、1.7px 描边、currentColor）
// 替代 emoji：同一字重、同一视觉语言

export type IconName =
  | "sparkle"
  | "trash"
  | "search"
  | "folder"
  | "package"
  | "shield"
  | "stop"
  | "drive"
  | "check"
  | "x"
  | "chevron"
  | "warn";

const PATHS: Record<IconName, string> = {
  // 四角星（品牌 / 一键清理）
  sparkle: `<path d="M12 2.6l2.3 7.1 7.1 2.3-7.1 2.3-2.3 7.1-2.3-7.1-7.1-2.3 7.1-2.3z"/>`,
  // 垃圾桶
  trash: `<path d="M4.2 6.6h15.6M9.2 6.6V4.9a1.3 1.3 0 0 1 1.3-1.3h3a1.3 1.3 0 0 1 1.3 1.3v1.7M6.2 6.6l.8 12a1.6 1.6 0 0 0 1.6 1.4h6.8a1.6 1.6 0 0 0 1.6-1.4l.8-12M10.2 10.4v6.4M13.8 10.4v6.4"/>`,
  // 放大镜
  search: `<circle cx="10.6" cy="10.6" r="6.4"/><path d="M15.5 15.5L20.6 20.6"/>`,
  // 文件夹
  folder: `<path d="M3.6 7.4a1.9 1.9 0 0 1 1.9-1.9h3.7l1.9 2.1h7.4a1.9 1.9 0 0 1 1.9 1.9v7.2a1.9 1.9 0 0 1-1.9 1.9H5.5a1.9 1.9 0 0 1-1.9-1.9z"/>`,
  // 安装包
  package: `<path d="M12 3.2l7.8 4.1v9.4L12 20.8l-7.8-4.1V7.3zM12 12l7.8-4.1M12 12v8.8M4.2 7.9l7.8 4.1"/>`,
  // 盾牌（权限）
  shield: `<path d="M12 3.4l6.8 2.7v5.1c0 4.4-2.9 8-6.8 9.6-3.9-1.6-6.8-5.2-6.8-9.6V6.1zM8.9 11.9l2.2 2.2 4.1-4.3"/>`,
  // 停止
  stop: `<rect x="6.7" y="6.7" width="10.6" height="10.6" rx="2.2"/>`,
  // 硬盘
  drive: `<rect x="3.6" y="7.6" width="16.8" height="8.8" rx="1.6"/><path d="M3.6 11.2h16.8M7 14.4h.01M10.4 14.4h.01"/>`,
  // 对勾
  check: `<path d="M5 12.6l4.4 4.4L19 7.4"/>`,
  // 关闭
  x: `<path d="M6.5 6.5l11 11M17.5 6.5l-11 11"/>`,
  // 下拉箭头
  chevron: `<path d="M6.6 9.6l5.4 5.4 5.4-5.4"/>`,
  // 警示
  warn: `<path d="M12 4.4L21 19.4H3zM12 10v4.2M12 17.2v.01"/>`,
};

export function icon(name: IconName, size = 16, cls = ""): string {
  return `<svg class="icon${cls ? ` ${cls}` : ""}" width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${PATHS[name]}</svg>`;
}

/** 通用骨架屏行（加载占位） */
export function skeletonRows(count: number): string {
  let out = "";
  for (let i = 0; i < count; i++) {
    out += `
      <div class="skel-row" aria-hidden="true">
        <div class="skeleton" style="width:16px;height:16px;border-radius:5px;flex-shrink:0"></div>
        <div style="flex:1;min-width:0">
          <div class="skeleton" style="width:42%;height:14px;margin-bottom:9px"></div>
          <div class="skeleton" style="width:72%;height:11px"></div>
        </div>
        <div class="skeleton" style="width:66px;height:14px;flex-shrink:0"></div>
      </div>`;
  }
  return out;
}
