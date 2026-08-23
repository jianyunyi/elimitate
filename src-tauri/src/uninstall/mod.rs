//! 软件卸载与残留清理

pub mod enumerate;
pub mod residue;
pub mod remove;

use serde::{Deserialize, Serialize};
use winreg::enums::{HKEY_CLASSES_ROOT, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, HKEY_USERS};
use windows_sys::Win32::System::Registry::HKEY;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InstalledApp {
    /// 注册表项完整路径，如 "HKLM\SOFTWARE\...\Uninstall\Foo"
    pub key: String,
    pub name: String,
    pub version: String,
    pub publisher: String,
    pub install_location: String,
    /// 已转换为卸载命令（MSI 的 /I 已转为 /X）
    pub uninstall_string: String,
    pub display_icon: String,
    pub estimated_size_kb: u64,
    pub install_date: String,
    pub is_user: bool,
    pub system_component: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ResidueItem {
    pub path: String,
    /// file | dir | shortcut | registry_key | registry_value
    pub kind: String,
    pub size_bytes: u64,
    /// low | medium | high
    pub risk: String,
    pub note: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResidueReport {
    pub app_key: String,
    pub name: String,
    pub items: Vec<ResidueItem>,
    pub total_size_bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteReport {
    pub deleted: u64,
    pub failed: u64,
    pub bytes_freed: u64,
    /// 因被占用而跳过的文件数
    pub locked: u64,
    /// 被占用文件采样路径（最多 20 条）
    pub locked_paths: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UninstallResult {
    pub launched: bool,
    pub message: String,
}

/// 解析 "HKLM\SOFTWARE\..." 形式的路径为 (hive, 子键路径)
pub fn parse_hive_path(s: &str) -> Option<(HKEY, String)> {
    let (hive, rest) = s.split_once('\\')?;
    let hive = match hive.to_ascii_uppercase().as_str() {
        "HKLM" | "HKEY_LOCAL_MACHINE" => HKEY_LOCAL_MACHINE,
        "HKCU" | "HKEY_CURRENT_USER" => HKEY_CURRENT_USER,
        "HKCR" | "HKEY_CLASSES_ROOT" => HKEY_CLASSES_ROOT,
        "HKU" | "HKEY_USERS" => HKEY_USERS,
        _ => return None,
    };
    Some((hive, rest.to_string()))
}

pub fn hive_name(h: HKEY) -> &'static str {
    if h == HKEY_LOCAL_MACHINE {
        "HKLM"
    } else if h == HKEY_CURRENT_USER {
        "HKCU"
    } else if h == HKEY_CLASSES_ROOT {
        "HKCR"
    } else {
        "HKU"
    }
}
