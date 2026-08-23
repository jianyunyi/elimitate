//! 卸载残留扫描：查找卸载后遗留的文件、快捷方式与注册表项

use std::collections::HashSet;
use std::path::Path;

use walkdir::WalkDir;
use winreg::enums::*;
use winreg::RegKey;
use windows_sys::Win32::System::Registry::HKEY;

use super::{hive_name, parse_hive_path, ResidueItem, ResidueReport};
use crate::util;

fn item(path: String, kind: &str, size: u64, risk: &str, note: &str) -> ResidueItem {
    ResidueItem {
        path,
        kind: kind.to_string(),
        size_bytes: size,
        risk: risk.to_string(),
        note: note.to_string(),
    }
}

/// 文件名是否包含软件名（>=2 个非空词）以避免误匹配
fn word_match(fname: &str, words: &[String]) -> bool {
    let count = words.iter().filter(|w| fname.contains(w.as_str())).count();
    count >= 2
}

/// 注册表子键是否存在
fn reg_key_exists(hive: HKEY, path: &str) -> bool {
    RegKey::predef(hive).open_subkey(path).is_ok()
}

#[tauri::command]
pub fn scan_residue(app_key: String) -> Result<ResidueReport, String> {
    let (hive, path) = parse_hive_path(&app_key).ok_or("无效的注册表路径")?;
    let key = RegKey::predef(hive)
        .open_subkey(&path)
        .map_err(|e| format!("打开注册表项失败: {e}"))?;
    let name: String = key.get_value("DisplayName").unwrap_or_default();
    let publisher: String = key.get_value("Publisher").unwrap_or_default();
    let install_location: String = key.get_value("InstallLocation").unwrap_or_default();

    let name_lower = name.to_lowercase();
    let name_words: Vec<String> = name_lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 2)
        .map(|w| w.to_string())
        .collect();

    let mut items: Vec<ResidueItem> = Vec::new();
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut push = |it: ResidueItem| {
        if seen.insert((it.path.clone(), it.kind.clone())) {
            items.push(it);
        }
    };

    // 1) 安装目录
    let install_dir = util::expand_env(&install_location);
    if !install_dir.is_empty() {
        let p = Path::new(&install_dir);
        if p.exists() {
            let kind = if p.is_dir() { "dir" } else { "file" };
            let size = if p.is_dir() {
                util::dir_size(p)
            } else {
                p.metadata().map(|m| m.len()).unwrap_or(0)
            };
            push(item(
                install_dir.clone(),
                kind,
                size,
                "medium",
                "安装目录（请确认软件已卸载后再删除）",
            ));
        }
    }

    // 2) 开始菜单 / 桌面快捷方式
    let program_data = std::env::var("ProgramData").ok();
    let shortcut_bases = [
        program_data.as_ref().map(|p| std::path::PathBuf::from(format!("{p}\\Microsoft\\Windows\\Start Menu\\Programs"))),
        dirs::data_dir().map(|p| p.join("Microsoft\\Windows\\Start Menu\\Programs")),
        dirs::desktop_dir(),
        dirs::public_dir().map(|p| p.join("Desktop")),
    ];
    for base in shortcut_bases.into_iter().flatten() {
        if !base.exists() {
            continue;
        }
        let walker = WalkDir::new(&base).follow_links(false).into_iter().filter_entry(|e| {
            !(e.path_is_symlink() || e.metadata().map(|m| util::is_reparse(&m)).unwrap_or(false))
        });
        for entry in walker.flatten() {
            if !entry.file_type().is_file() {
                continue;
            }
            let fname = entry.file_name().to_string_lossy().to_lowercase();
            let no_ext = fname.trim_end_matches(".lnk");
            let matched = fname.ends_with(".lnk")
                && (no_ext.contains(&name_lower) || word_match(&fname, &name_words));
            if matched {
                let p = entry.path().display().to_string();
                push(item(p, "shortcut", 0, "low", "快捷方式"));
            }
        }
    }

    // 3) 应用数据 / 安装目录候选（按名称与发布者）
    let data_dir = dirs::data_dir();
    let data_local_dir = dirs::data_local_dir();
    let candidates = vec![
        (data_dir.as_ref().map(|p| p.join(&name).display().to_string()), false),
        (data_local_dir.as_ref().map(|p| p.join(&name).display().to_string()), false),
        (program_data.as_ref().map(|p| format!("{p}\\{name}")), false),
        (Some(format!("C:\\Program Files\\{name}")), false),
        (Some(format!("C:\\Program Files (x86)\\{name}")), false),
        // 发布者目录可能被同厂商其他软件共用 → 高风险
        (data_dir.as_ref().map(|p| p.join(&publisher).display().to_string()), true),
        (data_local_dir.as_ref().map(|p| p.join(&publisher).display().to_string()), true),
    ]
    .into_iter()
    .filter_map(|(p, high)| p.map(|p| (p, high)));
    for (dir, high) in candidates {
        if dir.is_empty() {
            continue;
        }
        let p = Path::new(&dir);
        if p.exists() {
            let risk = if high { "high" } else { "medium" };
            let note = if high {
                "发布者目录（可能被同厂商其他软件共用，删除前请仔细确认）"
            } else {
                "应用数据目录（卸载后遗留的配置与数据）"
            };
            let size = if p.is_dir() { util::dir_size(p) } else { p.metadata().map(|m| m.len()).unwrap_or(0) };
            push(item(dir.clone(), if p.is_dir() { "dir" } else { "file" }, size, risk, note));
        }
    }

    // 4) 注册表 SOFTWARE 下的软件/发布者键
    let software_candidates: Vec<(HKEY, String, bool)> = {
        let mut v = Vec::new();
        for (h, prefix) in [
            (HKEY_LOCAL_MACHINE, r"SOFTWARE"),
            (HKEY_LOCAL_MACHINE, r"SOFTWARE\WOW6432Node"),
            (HKEY_CURRENT_USER, r"SOFTWARE"),
        ] {
            if !name.is_empty() {
                v.push((h, format!("{prefix}\\{name}"), false));
            }
            if !publisher.is_empty() && publisher.to_lowercase() != name_lower {
                v.push((h, format!("{prefix}\\{publisher}"), true));
            }
        }
        v
    };
    for (h, sub, high) in software_candidates {
        if reg_key_exists(h, &sub) {
            let full = format!("{}\\{sub}", hive_name(h));
            let risk = if high { "high" } else { "medium" };
            let note = if high {
                "发布者注册表键（可能被其他软件共用）"
            } else {
                "软件注册表键"
            };
            push(item(full, "registry_key", 0, risk, note));
        }
    }

    // 5) App Paths 注册表项（指向安装目录的注册入口）
    let app_paths = r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths";
    for h in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
        let Ok(ap) = RegKey::predef(h).open_subkey(app_paths) else { continue };
        for sub in ap.enum_keys().flatten() {
            let Ok(subkey) = ap.open_subkey(&sub) else { continue };
            let target: String = subkey.get_value("").unwrap_or_default();
            let target = util::expand_env(&target);
            let related = (!install_dir.is_empty()
                && target.to_lowercase().starts_with(&install_dir.to_lowercase()))
                || target.to_lowercase().contains(&name_lower);
            if related {
                let full = format!("{}\\{app_paths}\\{sub}", hive_name(h));
                push(item(full, "registry_key", 0, "low", "App Paths 注册表项"));
            }
        }
    }

    // 6) 卸载注册表项本身
    push(item(
        app_key.clone(),
        "registry_key",
        0,
        "low",
        "卸载注册表项（若软件仍在使用，请勿删除）",
    ));

    // 排序：目录 > 文件 > 快捷方式 > 注册表项
    let order = |k: &str| match k {
        "dir" => 0,
        "file" => 1,
        "shortcut" => 2,
        "registry_key" => 3,
        _ => 4,
    };
    items.sort_by(|a, b| order(&a.kind).cmp(&order(&b.kind)).then(a.path.cmp(&b.path)));

    let total_size_bytes = items.iter().map(|i| i.size_bytes).sum();
    Ok(ResidueReport {
        app_key,
        name,
        items,
        total_size_bytes,
    })
}
