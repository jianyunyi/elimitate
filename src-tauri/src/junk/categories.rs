//! 垃圾分类定义：id、名称、风险、路径模板、文件匹配规则

/// 特殊分类（需要专门的处理逻辑）
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum SpecialCategory {
    /// 回收站：通过 Shell API 清空
    RecycleBin,
}

pub struct CategorySpec {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub risk: &'static str, // low | medium | high
    pub requires_admin: bool,
    /// 路径模板：以 %VAR% 开头的部分会被替换为环境变量
    pub path_templates: &'static [&'static str],
    /// 仅匹配这些文件名前缀的文件（为空则整目录清理）
    pub file_prefixes: &'static [&'static str],
    /// 仅匹配这些文件名后缀的文件
    pub file_suffixes: &'static [&'static str],
    pub special: Option<SpecialCategory>,
}

/// 展开路径模板中的 %VAR%
pub fn resolve_template(t: &str) -> Option<String> {
    if t.starts_with('%') {
        let close = t[1..].find('%')? + 1;
        let var = &t[1..close];
        let rest = &t[close + 1..];
        std::env::var(var).ok().map(|v| format!("{v}{rest}"))
    } else {
        Some(t.to_string())
    }
}

pub fn specs() -> Vec<CategorySpec> {
    vec![
        CategorySpec {
            id: "temp_files",
            name: "临时文件",
            description: "用户与应用产生的临时文件，删除后不影响系统，部分文件可能被占用。",
            risk: "low",
            requires_admin: true, // 包含 C:\Windows\Temp
            path_templates: &["%TEMP%", "C:\\Windows\\Temp"],
            file_prefixes: &[],
            file_suffixes: &[],
            special: None,
        },
        CategorySpec {
            id: "internet_temp",
            name: "Internet 临时文件",
            description: "IE/Edge 的 Internet 缓存，删除后网页首次访问会稍慢。",
            risk: "low",
            requires_admin: false,
            path_templates: &["%LOCALAPPDATA%\\Microsoft\\Windows\\INetCache"],
            file_prefixes: &[],
            file_suffixes: &[],
            special: None,
        },
        CategorySpec {
            id: "browser_cache",
            name: "浏览器缓存",
            description: "Chrome / Edge 网页缓存，删除后网页首次访问会重新下载资源。",
            risk: "low",
            requires_admin: false,
            path_templates: &[
                "%LOCALAPPDATA%\\Google\\Chrome\\User Data\\Default\\Cache\\Cache_Data",
                "%LOCALAPPDATA%\\Google\\Chrome\\User Data\\Default\\Code Cache",
                "%LOCALAPPDATA%\\Microsoft\\Edge\\User Data\\Default\\Cache\\Cache_Data",
                "%LOCALAPPDATA%\\Microsoft\\Edge\\User Data\\Default\\Code Cache",
            ],
            file_prefixes: &[],
            file_suffixes: &[],
            special: None,
        },
        CategorySpec {
            id: "recycle_bin",
            name: "回收站",
            description: "清空回收站中的所有已删除文件。此操作不可恢复，请谨慎。",
            risk: "medium",
            requires_admin: false,
            path_templates: &["C:\\$Recycle.Bin"],
            file_prefixes: &[],
            file_suffixes: &[],
            special: Some(SpecialCategory::RecycleBin),
        },
        CategorySpec {
            id: "update_cache",
            name: "Windows 更新缓存",
            description: "Windows Update 下载的安装包缓存，删除后可重新下载更新。",
            risk: "medium",
            requires_admin: true,
            path_templates: &["C:\\Windows\\SoftwareDistribution\\Download"],
            file_prefixes: &[],
            file_suffixes: &[],
            special: None,
        },
        CategorySpec {
            id: "delivery_optimization",
            name: "传递优化缓存",
            description: "Windows 更新传递优化（P2P）下载的缓存文件。",
            risk: "low",
            requires_admin: true,
            path_templates: &["C:\\Windows\\SoftwareDistribution\\DeliveryOptimization"],
            file_prefixes: &[],
            file_suffixes: &[],
            special: None,
        },
        CategorySpec {
            id: "thumbnail_cache",
            name: "缩略图缓存",
            description: "资源管理器生成的缩略图与图标缓存，删除后自动重建。",
            risk: "low",
            requires_admin: false,
            path_templates: &["%LOCALAPPDATA%\\Microsoft\\Windows\\Explorer"],
            file_prefixes: &["thumbcache_", "iconcache_"],
            file_suffixes: &[],
            special: None,
        },
        CategorySpec {
            id: "logs",
            name: "系统日志",
            description: "Windows 组件产生的日志文件（CBS、DPX 等），删除后可能影响排障。",
            risk: "medium",
            requires_admin: true,
            path_templates: &["C:\\Windows\\Logs"],
            file_prefixes: &[],
            file_suffixes: &[],
            special: None,
        },
        CategorySpec {
            id: "error_reports",
            name: "系统错误报告",
            description: "程序崩溃与错误报告（WER）数据，用于诊断问题，可安全删除。",
            risk: "low",
            requires_admin: false,
            path_templates: &[
                "C:\\ProgramData\\Microsoft\\Windows\\WER",
                "%LOCALAPPDATA%\\Microsoft\\Windows\\WER",
            ],
            file_prefixes: &[],
            file_suffixes: &[],
            special: None,
        },
        CategorySpec {
            id: "prefetch",
            name: "预读取缓存 (Prefetch)",
            description: "系统启动预读取数据，删除后下次开机启动速度会短暂变慢。",
            risk: "low",
            requires_admin: true,
            path_templates: &["C:\\Windows\\Prefetch"],
            file_prefixes: &[],
            file_suffixes: &[],
            special: None,
        },
        CategorySpec {
            id: "directx_shader",
            name: "DirectX 着色器缓存",
            description: "DirectX / NVIDIA 着色器编译缓存，删除后游戏首次运行会重新编译。",
            risk: "low",
            requires_admin: false,
            path_templates: &[
                "%LOCALAPPDATA%\\D3DSCache",
                "%LOCALAPPDATA%\\NVIDIA\\DXCache",
                "%LOCALAPPDATA%\\NVIDIA\\GLCache",
            ],
            file_prefixes: &[],
            file_suffixes: &[],
            special: None,
        },
        CategorySpec {
            id: "crash_dumps",
            name: "崩溃转储",
            description: "程序崩溃时生成的内存转储文件（CrashDumps / Minidump）。",
            risk: "low",
            requires_admin: false,
            path_templates: &["%LOCALAPPDATA%\\CrashDumps", "C:\\Windows\\Minidump"],
            file_prefixes: &[],
            file_suffixes: &[],
            special: None,
        },
        CategorySpec {
            id: "package_caches",
            name: "开发工具缓存",
            description: "npm / pnpm / pip 等包管理器的下载缓存，删除后需要时重新下载。",
            risk: "low",
            requires_admin: false,
            path_templates: &[
                "%LOCALAPPDATA%\\npm-cache",
                "%LOCALAPPDATA%\\pnpm-cache",
                "%LOCALAPPDATA%\\pnpm\\store",
                "%LOCALAPPDATA%\\pip\\cache",
                "%USERPROFILE%\\.cache\\pip",
            ],
            file_prefixes: &[],
            file_suffixes: &[],
            special: None,
        },
    ]
}
