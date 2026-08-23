//! 通用工具：宽字符串、命令行解析、删除（回收站/永久、占用预检）、目录统计、进度事件

use std::ffi::c_void;
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};

use serde_json::json;
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

use windows_sys::Win32::Foundation::LocalFree;
use windows_sys::Win32::UI::Shell::{
    CommandLineToArgvW, SHFileOperationW, SHFILEOPSTRUCTW, FO_DELETE,
};

// Shell 删除标志
const FOF_ALLOWUNDO: u32 = 64; // 允许撤销 → 移入回收站
const FOF_NOCONFIRMATION: u32 = 16;
const FOF_NOERRORUI: u32 = 1024;
const FOF_SILENT: u32 = 4;

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

// ==================== 删除：回收站 / 永久 + 占用预检 ====================

/// 被占用（锁定）文件跟踪：总数 + 采样路径
#[derive(Default)]
pub struct LockTracker {
    pub count: u64,
    pub samples: Vec<String>,
}

impl LockTracker {
    pub fn add(&mut self, path: &Path) {
        self.count += 1;
        if self.samples.len() < 20 {
            self.samples.push(path.display().to_string());
        }
    }
}

/// 清除只读属性
pub fn clear_readonly(path: &Path) {
    if let Ok(meta) = fs::symlink_metadata(path) {
        let mut perms = meta.permissions();
        perms.set_readonly(false);
        let _ = fs::set_permissions(path, perms);
    }
}

/// 尝试以写权限打开文件；失败视为被占用（锁定）
pub fn is_locked(path: &Path) -> bool {
    fs::OpenOptions::new().write(true).create(false).open(path).is_err()
}

/// 将一批文件移入回收站（每批 ≤ 32 个，Shell API 支持多路径）
pub fn recycle_paths(files: &[(PathBuf, u64)], errors: &mut Vec<String>) -> (u64, u64) {
    let mut freed = 0u64;
    let mut items = 0u64;
    for chunk in files.chunks(32) {
        let chunk_freed: u64 = chunk.iter().map(|(_, s)| s).sum();
        match recycle_batch(chunk) {
            Ok(()) => {
                freed += chunk_freed;
                items += chunk.len() as u64;
            }
            Err(_) => {
                // 整批失败：逐条重试，定位失败项
                for (p, size) in chunk {
                    match recycle_batch(&[(p.clone(), *size)]) {
                        Ok(()) => {
                            freed += size;
                            items += 1;
                        }
                        Err(e) => errors.push(format!("{}: {e}", p.display())),
                    }
                }
            }
        }
    }
    (freed, items)
}

fn recycle_batch(files: &[(PathBuf, u64)]) -> std::io::Result<()> {
    if files.is_empty() {
        return Ok(());
    }
    // pFrom 格式：路径以 \0 分隔，整体以 \0\0 结尾
    let mut from: Vec<u16> = Vec::new();
    for (p, _) in files {
        let wide: Vec<u16> = p.as_os_str().encode_wide().collect();
        from.extend_from_slice(&wide);
        from.push(0);
    }
    from.push(0);

    let mut op = SHFILEOPSTRUCTW {
        hwnd: std::ptr::null_mut(),
        wFunc: FO_DELETE,
        pFrom: from.as_ptr(),
        pTo: std::ptr::null(),
        fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_NOERRORUI | FOF_SILENT) as u16,
        fAnyOperationsAborted: 0,
        hNameMappings: std::ptr::null_mut(),
        lpszProgressTitle: std::ptr::null(),
    };
    let result = unsafe { SHFileOperationW(&mut op) };
    if result != 0 {
        return Err(std::io::Error::from_raw_os_error(result));
    }
    Ok(())
}

/// 批量删除文件（预检占用；回收站批量或永久逐个）
/// 返回 (释放字节, 删除项数)；占用文件记入 locked，失败记入 errors
pub fn delete_file_batch(
    files: &[(PathBuf, u64)],
    to_recycle_bin: bool,
    errors: &mut Vec<String>,
    locked: &mut LockTracker,
) -> (u64, u64) {
    let mut todo: Vec<(PathBuf, u64)> = Vec::with_capacity(files.len());
    for (p, size) in files {
        clear_readonly(p);
        if is_locked(p) {
            locked.add(p);
            continue;
        }
        todo.push((p.clone(), *size));
    }
    if todo.is_empty() {
        return (0, 0);
    }
    if to_recycle_bin {
        recycle_paths(&todo, errors)
    } else {
        let mut freed = 0u64;
        let mut items = 0u64;
        for (p, size) in &todo {
            match fs::remove_file(p) {
                Ok(_) => {
                    freed += size;
                    items += 1;
                }
                Err(e) => errors.push(format!("{}: {e}", p.display())),
            }
        }
        (freed, items)
    }
}

fn is_dir_empty(path: &Path) -> bool {
    fs::read_dir(path).map(|mut rd| rd.next().is_none()).unwrap_or(false)
}

/// 递归删除路径（文件或目录）。目录内的文件逐个处理（支持占用预检与回收站），空目录直接移除
pub fn delete_path_force(
    path: &Path,
    to_recycle_bin: bool,
    errors: &mut Vec<String>,
    locked: &mut LockTracker,
) -> (u64, u64) {
    let meta = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(_) => return (0, 0),
    };
    if meta.is_file() {
        return delete_file_batch(&[(path.to_path_buf(), meta.len())], to_recycle_bin, errors, locked);
    }
    let mut freed = 0u64;
    let mut items = 0u64;
    if let Ok(rd) = fs::read_dir(path) {
        for entry in rd.flatten() {
            let (f, i) = delete_path_force(&entry.path(), to_recycle_bin, errors, locked);
            freed += f;
            items += i;
        }
    }
    clear_readonly(path);
    match fs::remove_dir(path) {
        Ok(_) => items += 1,
        Err(e) => {
            // 目录仍非空（残留被占用文件）或正被占用：占用已计入 locked，此处仅记录真正的异常
            if is_dir_empty(path) {
                errors.push(format!("{}: {e}", path.display()));
            }
        }
    }
    (freed, items)
}

/// 自底向上删除空目录（忽略失败：可能仍含被占用文件）
pub fn prune_empty_dirs(root: &Path) {
    let mut dirs: Vec<PathBuf> = Vec::new();
    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            !(e.path_is_symlink() || e.metadata().map(|m| is_reparse(&m)).unwrap_or(false))
        });
    for entry in walker.flatten() {
        if entry.file_type().is_dir() {
            dirs.push(entry.path().to_path_buf());
        }
    }
    for d in dirs.iter().rev() {
        let _ = fs::remove_dir(d);
    }
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
