# 🧹 Elimitate · 清理助手

> 开源的 Windows 电脑清理与彻底卸载工具

**Elimitate** 是一款面向 Windows 的开源桌面工具，帮助你：

- 🗑️ **清理垃圾文件**：临时文件、浏览器缓存、回收站、Windows 更新缓存、缩略图缓存、系统日志、错误报告、预读取缓存、DirectX 着色器缓存、开发工具缓存等十余类垃圾，按类别扫描大小后选择性清理，释放磁盘空间。
- 📦 **彻底卸载软件**：从注册表枚举已安装软件，运行官方卸载程序后，自动扫描卸载遗留的文件、快捷方式与注册表项，实现"根本删除"。
- ⚡ **一键清理**：一条命令完成全部垃圾扫描与清理。

技术栈：**Tauri 2 + Rust + Web (TypeScript/Vite)**。后端 Rust 直接调用 Windows API（注册表、Shell 文件操作、权限提升），前端为轻量级现代界面，打包体积小、内存占用低。

---

## ✨ 功能特性

### 垃圾清理（13 个类别）

| 类别 | 说明 | 风险 |
| --- | --- | --- |
| 临时文件 | 用户与应用临时文件（`%TEMP%`、`C:\Windows\Temp`） | 低 |
| Internet 临时文件 | IE/Edge Internet 缓存（`INetCache`） | 低 |
| 浏览器缓存 | Chrome / Edge 网页缓存（Cache、Code Cache） | 低 |
| 回收站 | 通过 Shell API 安全清空回收站 | 中 |
| Windows 更新缓存 | `SoftwareDistribution\Download` 更新安装包 | 中 |
| 传递优化缓存 | Windows 更新 P2P 传递缓存 | 低 |
| 缩略图缓存 | `thumbcache_*` / `iconcache_*`（自动重建） | 低 |
| 系统日志 | `C:\Windows\Logs`（CBS、DPX 等） | 中 |
| 系统错误报告 | WER 崩溃报告数据 | 低 |
| 预读取缓存 | Prefetch 启动预读取数据 | 低 |
| DirectX 着色器缓存 | D3DSCache / NVIDIA DXCache、GLCache | 低 |
| 崩溃转储 | CrashDumps / Minidump 内存转储 | 低 |
| 开发工具缓存 | npm / pnpm / pip 下载缓存 | 低 |

每个类别可查看**实际路径、文件数量、占用大小、风险等级**，勾选后清理，支持清理进度实时反馈与失败项收集。

### 彻底卸载与残留清理

- 从 `HKLM`（含 `WOW6432Node`）与 `HKCU` 的 Uninstall 键枚举已安装软件，过滤系统组件与 Windows 更新。
- 自动处理 MSI 安装包：将注册表中的 `/I`（修复）命令转换为 `/X`（卸载）命令。
- 卸载完成后**扫描残留**：
  - 安装目录（`InstallLocation`）
  - 开始菜单 / 桌面快捷方式（按软件名与关键词匹配）
  - 应用数据目录（`%APPDATA%`、`%LOCALAPPDATA%`、`%ProgramData%`、`Program Files`）
  - 注册表 `SOFTWARE` 键（软件名 / 发布者）
  - `App Paths` 注册入口
  - 卸载注册表项本身
- 残留按类型分组展示大小与风险等级，可**全选 / 仅选低中风险 / 手动勾选**后删除；高风险项（可能被其他软件共用的发布者目录/注册表键）单独标注。

### 权限处理

- 非管理员运行时界面明确提示，可一键"以管理员身份重启"（UAC）。
- 清理系统目录（`Windows\Temp`、更新缓存等）需要管理员权限；删除 HKLM 注册表残留同样需要。

---

## 🚀 构建与运行

### 环境要求

- [Rust](https://rustup.rs/)（stable，≥ 1.77.2）
- [Node.js](https://nodejs.org/) ≥ 18 与 [pnpm](https://pnpm.io/)
- Windows 10/11（需 WebView2，Win11 自带）
- MSVC 构建工具（Visual Studio 2022 Build Tools，含 C++ 桌面开发）

### 开发运行

```bash
pnpm install
pnpm tauri dev
```

### 打包安装程序（NSIS / MSI）

```bash
pnpm tauri build
```

产物位于 `src-tauri/target/release/bundle/`。

> 提示：首次 `cargo build` 需要编译大量依赖，耗时较长属正常现象。

---

## 📁 项目结构

```
elimitate/
├── src/                    # 前端（TypeScript + Vite，无框架依赖）
│   ├── main.ts             # 应用入口与导航
│   ├── api.ts              # Tauri 命令封装
│   ├── types.ts            # 与后端序列化对应的类型
│   ├── ui.ts               # toast / 确认框 / 徽章
│   ├── format.ts           # 字节格式化等工具
│   ├── style.css           # 深色主题
│   └── pages/
│       ├── dashboard.ts    # 一键清理页
│       ├── junk.ts         # 垃圾清理页
│       └── uninstall.ts    # 软件卸载页
└── src-tauri/              # Rust 后端
    ├── src/
    │   ├── lib.rs          # Tauri 命令注册
    │   ├── system.rs       # 系统信息（磁盘、OS 版本）
    │   ├── admin.rs        # 管理员提权重启
    │   ├── util.rs         # 宽字符串 / 命令行解析 / 强制删除 / 进度事件
    │   ├── junk/           # 垃圾清理模块
    │   │   ├── categories.rs  # 分类定义（路径模板、风险）
    │   │   ├── scan.rs        # 扫描统计
    │   │   └── clean.rs       # 清理执行（含回收站）
    │   └── uninstall/      # 卸载模块
    │       ├── enumerate.rs   # 注册表软件枚举
    │       ├── residue.rs     # 残留扫描
    │       └── remove.rs      # 残留删除与卸载程序启动
    ├── capabilities/       # Tauri 权限配置
    └── icons/              # 应用图标
```

## 🔒 安全设计

- **只删已知路径**：垃圾清理仅操作预定义的已知垃圾目录，不做全盘扫描，杜绝误删。
- **风险分级**：每个类别与每条残留都标注风险等级；高风险项（发布者级目录/注册表键）需用户手动确认。
- **失败不静默**：被占用或权限不足的文件会收集到错误列表展示，不阻塞其他项目。
- **删除确认**：清理前弹出确认框并显示预计释放空间。
- **最小权限**：普通权限即可完成大部分用户级清理；仅系统级操作需要管理员。

## ⚠️ 免责声明

本工具会**永久删除**所选文件，删除后不可恢复。请在使用前确认所选项目确为垃圾或已卸载软件的残留。作者不对因使用本工具造成的数据丢失负责。

## 🗺️ Roadmap

- [ ] Firefox 浏览器缓存支持
- [ ] 大文件/磁盘空间占用分析
- [ ] 启动项管理（禁用/启用）
- [ ] 清理前自动备份（可恢复）
- [ ] 简体中文界面国际化（i18n）
- [ ] 安装程序图标与商店打包

## 📄 License

[MIT](./LICENSE) © Elimitate Contributors
