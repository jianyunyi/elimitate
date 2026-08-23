//! 通用工具：宽字符串、命令行解析、强制删除、目录统计、进度事件

use std::ffi::c_void;
use std::fs;
use std::os::windows::fs::MetadataExt;
use std::path::Path;

use serde_json::json;
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

use windows_sys::Win32::Foundation::LocalFree;
use windows_sys::Win32::UI::Shell::CommandLineToArgvW;

/// 转换为以 \0 结尾的 UTF-16 宽字符串
pub fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 从以 \0 结尾的宽字符串指针读取 String
pub unsafe fn wide_to_string(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let mut len = 0usize;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
}

/// 解析 Windows 命令行（正确处理引号与转义），返回参数列表
pub fn parse_command_line(cmdline: &str) -> Vec<String> {
    let wide = to_wide(cmdline);
    let mut argc: i32 = 0;
    unsafe {
        let argv = CommandLineToArgvW(wide.as_ptr(), &mut argc);
        if argv.is_null() {
            // 回退：按空白简单切分
            return cmdline.split_whitespace().map(|s| s.trim_matches('"').to_string()).collect();
        }
        let mut out = Vec::with_capacity(argc as usize);
        for i in 0..argc as isize {
            let p = *argv.offset(i);
            out.push(wide_to_string(p));
        }
        LocalFree(argv as *mut c_void);
        out
    }
}

/// 展开路径中的 %VAR% 环境变量（若存在）
pub fn expand_env(path: &str) -> String {
    let mut result = path.trim().trim_matches('"').to_string();
    let mut idx = 0;
    while idx < result.len() {
        if let Some(start) = result[idx..].find('%') {
            let s = idx + start;
            if let Some(end_rel) = result[s + 1..].find('%') {
                let e = s + 1 + end_rel;
                let var = &result[s + 1..e];
                if let Ok(val) = std::env::var(var) {
                    result.replace_range(s..=e, &val);
                    idx = s + val.len();
                    continue;
                }
            }
            idx = s + 1;
        } else {
            break;
        }
    }
    result
}

/// 是否为重解析点（符号链接 / 目录联接），用于避免扫描循环
pub fn is_reparse(meta: &fs::Metadata) -> bool {
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

/// 统计目录大小（跳过重解析点）
pub fn dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    let walker = WalkDir::new(path).follow_links(false).into_iter().filter_entry(|e| {
        !(e.path_is_symlink() || e.metadata().map(|m| is_reparse(&m)).unwrap_or(false))
    });
    for entry in walker.flatten() {
        if entry.file_type().is_file() {
            total += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    total
}

/// 递归删除（含只读文件），返回 (释放字节数, 删除项数)
pub fn delete_path_force(path: &Path, errors: &mut Vec<String>) -> (u64, u64) {
    let meta = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(_) => return (0, 0),
    };
    if meta.is_dir() {
        let mut freed = 0u64;
        let mut items = 0u64;
        if let Ok(rd) = fs::read_dir(path) {
            for entry in rd.flatten() {
                let (f, i) = delete_path_force(&entry.path(), errors);
                freed += f;
                items += i;
            }
        }
        clear_readonly(path, &meta);
        match fs::remove_dir(path) {
            Ok(_) => items += 1,
            Err(e) => errors.push(format!("{}: {e}", path.display())),
        }
        (freed, items)
    } else {
        let size = meta.len();
        clear_readonly(path, &meta);
        match fs::remove_file(path) {
            Ok(_) => (size, 1),
            Err(e) => {
                errors.push(format!("{}: {e}", path.display()));
                (0, 0)
            }
        }
    }
}

/// 清除只读属性（std 在新版本移除了 from_readonly，改用 set_readonly）
fn clear_readonly(path: &Path, meta: &fs::Metadata) {
    let mut perms = meta.permissions();
    perms.set_readonly(false);
    let _ = fs::set_permissions(path, perms);
}

/// 向前端广播进度事件
pub fn emit_progress(app: &AppHandle, phase: &str, id: &str, name: &str, done: u64, total: u64) {
    let _ = app.emit(
        "progress",
        json!({
            "categoryId": id,
            "categoryName": name,
            "phase": phase,
            "done": done,
            "total": total,
        }),
    );
}
