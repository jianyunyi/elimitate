//! 从注册表枚举已安装软件

use winreg::enums::*;
use winreg::RegKey;
use windows_sys::Win32::System::Registry::HKEY;

use super::{hive_name, InstalledApp};

const UNINSTALL: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
const UNINSTALL_32: &str = r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";

/// 从卸载命令中提取 MSI GUID（支持 /I 或 /X 后跟 {GUID} 或裸 GUID）
fn extract_msi_guid(s: &str) -> Option<String> {
    let upper = s.to_uppercase();
    let idx = upper.find("/I").or_else(|| upper.find("/X"))?;
    let rest = upper[idx + 2..].trim_start();
    let guid = rest.split_whitespace().next()?;
    Some(guid.trim_matches(|c| c == '{' || c == '}').to_string())
}

fn extract_guid(s: &str) -> Option<String> {
    let t = s.trim();
    if t.starts_with('{') && t.ends_with('}') && t.len() > 2 {
        Some(t[1..t.len() - 1].to_string())
    } else {
        None
    }
}

fn read_app(hive: HKEY, path: &str, is_user: bool) -> Option<InstalledApp> {
    let key = RegKey::predef(hive).open_subkey(path).ok()?;
    let name: String = key.get_value("DisplayName").ok()?;
    if name.trim().is_empty() {
        return None;
    }
    // 过滤系统组件与更新
    if key.get_value::<u32, _>("SystemComponent").unwrap_or(0) == 1 {
        return None;
    }
    if key.get_value::<String, _>("ParentKeyName").is_ok() {
        return None;
    }
    if let Ok(rt) = key.get_value::<String, _>("ReleaseType") {
        if !rt.trim().is_empty() {
            return None;
        }
    }

    let version: String = key.get_value("DisplayVersion").unwrap_or_default();
    let publisher: String = key.get_value("Publisher").unwrap_or_default();
    let install_location: String = key.get_value("InstallLocation").unwrap_or_default();
    let mut uninstall_string: String = key.get_value("UninstallString").unwrap_or_default();
    let display_icon: String = key.get_value("DisplayIcon").unwrap_or_default();
    let estimated_size_kb: u64 = key.get_value::<u32, _>("EstimatedSize").unwrap_or(0) as u64;
    let mut install_date: String = key.get_value("InstallDate").unwrap_or_default();
    if install_date.len() == 8 && install_date.chars().all(|c| c.is_ascii_digit()) {
        install_date = format!("{}-{}-{}", &install_date[0..4], &install_date[4..6], &install_date[6..8]);
    }

    // MSI 安装：把注册表里的 /I（修复）命令转换为 /X（卸载）命令
    if key.get_value::<u32, _>("WindowsInstaller").unwrap_or(0) == 1 {
        let key_name = path.rsplit('\\').next().unwrap_or("");
        if let Some(guid) = extract_msi_guid(&uninstall_string).or_else(|| extract_guid(key_name)) {
            uninstall_string = format!("msiexec /x {{{guid}}}");
        }
    }

    let key_path = format!("{}\\{path}", hive_name(hive));
    Some(InstalledApp {
        key: key_path,
        name,
        version,
        publisher,
        install_location,
        uninstall_string,
        display_icon,
        estimated_size_kb,
        install_date,
        is_user,
        system_component: false,
    })
}

#[tauri::command]
pub fn list_installed_apps() -> Result<Vec<InstalledApp>, String> {
    let mut out = Vec::new();
    for (hive, base, is_user) in [
        (HKEY_LOCAL_MACHINE, UNINSTALL, false),
        (HKEY_LOCAL_MACHINE, UNINSTALL_32, false),
        (HKEY_CURRENT_USER, UNINSTALL, true),
    ] {
        let Ok(root) = RegKey::predef(hive).open_subkey(base) else { continue };
        for name in root.enum_keys().flatten() {
            let path = format!("{base}\\{name}");
            if let Some(app) = read_app(hive, &path, is_user) {
                out.push(app);
            }
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}
