## 🧹 Elimitate v0.1.0 — Windows 垃圾清理与彻底卸载工具

开源的 Windows 桌面清理工具（Tauri 2 + Rust 后端 + Web 前端），帮你释放磁盘空间、提升系统性能。

### ✨ 功能特性

- **🗑️ 垃圾清理**：13 类垃圾分类扫描与清理 —— 临时文件、Internet 临时文件、浏览器缓存（Chrome/Edge）、回收站、Windows 更新缓存、传递优化缓存、缩略图缓存、系统日志、系统错误报告、Prefetch 预读取、DirectX 着色器缓存、崩溃转储、npm/pnpm/pip 开发工具缓存
- **📦 彻底卸载**：注册表枚举已安装软件 → 运行官方卸载程序 → 扫描并删除卸载残留（安装目录、开始菜单/桌面快捷方式、AppData 数据目录、注册表键、App Paths 项、卸载注册表项）
- **⚡ 一键清理**：全量扫描 + 清理 + 清空回收站，实时进度
- **🔐 安全设计**：每个类别/残留项标注风险等级；删除前确认并显示预计释放空间；只操作已知路径、绝不全盘扫描；失败项收集不静默
- 管理员权限检测，一键 UAC 提权重启

### 📥 下载

| 文件 | 说明 |
| --- | --- |
| `Elimitate_0.1.0_x64-setup.exe` | **NSIS 安装包（推荐）**，双击安装，带开始菜单快捷方式 |
| `elimitate.exe` | **免安装绿色版**，下载后直接双击运行 |
| `Elimitate_0.1.0_x64_en-US.msi` | MSI 安装包，适合企业批量部署 |

### ⚙️ 系统要求

- Windows 10 / 11（x64）
- 需要 [WebView2 运行时](https://developer.microsoft.com/microsoft-edge/webview2/)（Windows 11 自带；Win10 通常随 Edge 更新自动安装）
- 清理系统目录（`Windows\Temp`、更新缓存等）建议以管理员身份运行

### 🧑‍💻 从源码构建

```bash
git clone https://github.com/jianyunyi/elimitate.git
cd elimitate
pnpm install
pnpm tauri build
```

### 📄 License

MIT
