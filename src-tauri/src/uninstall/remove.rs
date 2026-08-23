//! 删除残留与启动官方卸载程序

use std::path::Path;

use winreg::enums::*;
use winreg::RegKey;

use super::{parse_hive_path, DeleteReport, ResidueItem, UninstallResult};
use crate::util;

#[tauri::command]
pub fn delete_residue(_app_key: String, items: Vec<ResidueItem>) -> Result<DeleteReport, String> {
    let mut report = DeleteReport {
        deleted: 0,
        failed: 0,
        bytes_freed: 0,
        errors: Vec::new(),
    };

    for it in &items {
        let result = match it.kind.as_str() {
            "registry_key" => delete_registry_key(&it.path),
            "registry_value" => delete_registry_value(&it.path),
            _ => {
                let (freed, items_removed) = util::delete_path_force(Path::new(&it.path), &mut report.errors);
                if items_removed > 0 {
                    Ok(freed)
                } else {
                    Err(format!("删除失败: {}", it.path))
                }
            }
        };
        match result {
            Ok(freed) => {
                report.deleted += 1;
                report.bytes_freed += freed;
            }
            Err(e) => {
                report.failed += 1;
                report.errors.push(e);
            }
        }
    }
    Ok(report)
}

fn delete_registry_key(full: &str) -> Result<u64, String> {
    let (hive, rest) = parse_hive_path(full).ok_or_else(|| format!("无效的注册表路径: {full}"))?;
    let (parent, last) = rest
        .rsplit_once('\\')
        .ok_or_else(|| format!("无效的注册表路径: {full}"))?;
    let parent_key = RegKey::predef(hive)
        .open_subkey_with_flags(parent, KEY_READ | KEY_WRITE)
        .map_err(|e| format!("打开 {parent} 失败: {e}"))?;
    parent_key
        .delete_subkey_all(last)
        .map_err(|e| format!("删除注册表项 {last} 失败: {e}"))?;
    Ok(0)
}

fn delete_registry_value(full: &str) -> Result<u64, String> {
    let (hive, rest) = parse_hive_path(full).ok_or_else(|| format!("无效的注册表路径: {full}"))?;
    let (parent, last) = rest
        .rsplit_once('\\')
        .ok_or_else(|| format!("无效的注册表路径: {full}"))?;
    let key = RegKey::predef(hive)
        .open_subkey_with_flags(parent, KEY_READ | KEY_WRITE)
        .map_err(|e| format!("打开 {parent} 失败: {e}"))?;
    key.delete_value(last)
        .map_err(|e| format!("删除注册表值 {last} 失败: {e}"))?;
    Ok(0)
}

#[tauri::command]
pub fn uninstall_app(app_key: String) -> Result<UninstallResult, String> {
    let (hive, path) = parse_hive_path(&app_key).ok_or("无效的注册表路径")?;
    let key = RegKey::predef(hive)
        .open_subkey(&path)
        .map_err(|e| format!("打开注册表项失败: {e}"))?;

    let mut cmdline: String = key.get_value("UninstallString").unwrap_or_default();
    if cmdline.trim().is_empty() {
        cmdline = key.get_value("QuietUninstallString").unwrap_or_default();
    }
    if cmdline.trim().is_empty() {
        return Ok(UninstallResult {
            launched: false,
            message: "该软件没有提供卸载命令（UninstallString）".into(),
        });
    }

    let args = util::parse_command_line(&cmdline);
    if args.is_empty() {
        return Ok(UninstallResult {
            launched: false,
            message: format!("无法解析卸载命令: {cmdline}"),
        });
    }

    let mut cmd = std::process::Command::new(&args[0]);
    cmd.args(&args[1..]);
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::System::Threading::CREATE_NEW_CONSOLE;
    cmd.creation_flags(CREATE_NEW_CONSOLE);
    cmd.spawn().map_err(|e| format!("启动卸载程序失败: {e}"))?;

    Ok(UninstallResult {
        launched: true,
        message: format!("已启动卸载程序: {cmdline}"),
    })
}
