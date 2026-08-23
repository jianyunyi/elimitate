//! 垃圾清理：删除选中类别的文件（支持按修改时间过滤、占用预检、删除到回收站）

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use tauri::AppHandle;
use walkdir::WalkDir;

use super::categories::{resolve_template, CategorySpec, SpecialCategory};
use super::CleanReport;
use crate::util::{self, LockTracker};

/// 清空回收站（Shell API），返回释放的字节数
fn empty_recycle_bin(errors: &mut Vec<String>) -> u64 {
    use windows_sys::Win32::UI::Shell::SHEmptyRecycleBinW;

    let rb = Path::new("C:\\$Recycle.Bin");
    let size = if rb.exists() { util::dir_size(rb) } else { 0 };

    // SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND
    const SHERB_FLAGS: u32 = 0x0000_0001 | 0x0000_0002 | 0x0000_0004;
    let result = unsafe { SHEmptyRecycleBinW(std::ptr::null_mut(), std::ptr::null(), SHERB_FLAGS) };
    if result != 0 {
        errors.push(format!("清空回收站失败（错误码 0x{result:x}）"));
        return 0;
    }
    size
}

/// 收集目录中修改时间早于阈值的文件（跳过重解析点）
fn collect_old_files(path: &Path, cutoff: SystemTime) -> Vec<(PathBuf, u64)> {
    let mut files = Vec::new();
    let walker = WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            !(e.path_is_symlink() || e.metadata().map(|m| util::is_reparse(&m)).unwrap_or(false))
        });
    for entry in walker.flatten() {
        if entry.file_type().is_file() {
            if let Ok(meta) = entry.metadata() {
                if meta.modified().map(|t| t < cutoff).unwrap_or(true) {
                    files.push((entry.path().to_path_buf(), meta.len()));
                }
            }
        }
    }
    files
}

fn clean_category(
    spec: &CategorySpec,
    cutoff: Option<SystemTime>,
    to_recycle_bin: bool,
    report: &mut CleanReport,
    locked: &mut LockTracker,
) {
    if spec.special == Some(SpecialCategory::RecycleBin) {
        report.bytes_freed = empty_recycle_bin(&mut report.errors);
        return;
    }

    for t in spec.path_templates {
        let Some(p) = resolve_template(t) else { continue };
        let path = Path::new(&p);
        if !path.exists() {
            continue;
        }

        // 文件名过滤（前缀/后缀）：只删除匹配的文件，保留目录
        if !spec.file_prefixes.is_empty() || !spec.file_suffixes.is_empty() {
            let walker = WalkDir::new(path)
                .follow_links(false)
                .into_iter()
                .filter_entry(|e| {
                    !(e.path_is_symlink() || e.metadata().map(|m| util::is_reparse(&m)).unwrap_or(false))
                });
            for entry in walker.flatten() {
                if !entry.file_type().is_file() {
                    continue;
                }
                let name = entry.file_name().to_string_lossy().to_lowercase();
                let matched = spec.file_prefixes.iter().any(|p| name.starts_with(&p.to_lowercase()))
                    || spec.file_suffixes.iter().any(|s| name.ends_with(&s.to_lowercase()));
                if !matched {
                    continue;
                }
                if let Some(c) = cutoff {
                    if let Ok(meta) = entry.metadata() {
                        if meta.modified().map(|t| t < c).unwrap_or(false) {
                            let f = &[(entry.path().to_path_buf(), meta.len())];
                            let (f2, i2) = util::delete_file_batch(f, to_recycle_bin, &mut report.errors, locked);
                            report.bytes_freed += f2;
                            report.items_removed += i2;
                        }
                    }
                } else {
                    let (f, i) = util::delete_path_force(entry.path(), to_recycle_bin, &mut report.errors, locked);
                    report.bytes_freed += f;
                    report.items_removed += i;
                }
            }
            continue;
        }

        // 按年龄过滤：只删够龄文件，然后清理空目录
        if let Some(c) = cutoff {
            if path.is_dir() {
                let files = collect_old_files(path, c);
                let (f, i) = util::delete_file_batch(&files, to_recycle_bin, &mut report.errors, locked);
                report.bytes_freed += f;
                report.items_removed += i;
                util::prune_empty_dirs(path);
            } else if path.is_file() {
                if let Ok(meta) = path.metadata() {
                    if meta.modified().map(|t| t < c).unwrap_or(true) {
                        let (f, i) = util::delete_path_force(path, to_recycle_bin, &mut report.errors, locked);
                        report.bytes_freed += f;
                        report.items_removed += i;
                    }
                }
            }
            continue;
        }

        // 不过滤：整目录删除（内部逐文件处理，支持占用预检与回收站）
        if path.is_dir() {
            let (f, i) = util::delete_path_force(path, to_recycle_bin, &mut report.errors, locked);
            report.bytes_freed += f;
            report.items_removed += i;
        } else if path.is_file() {
            let (f, i) = util::delete_path_force(path, to_recycle_bin, &mut report.errors, locked);
            report.bytes_freed += f;
            report.items_removed += i;
        }
    }
}

#[tauri::command]
pub fn clean_junk(
    app: AppHandle,
    ids: Vec<String>,
    max_age_days: Option<u64>,
    to_recycle_bin: bool,
) -> Result<Vec<CleanReport>, String> {
    let cutoff = super::scan::age_cutoff(max_age_days);
    let selected: Vec<CategorySpec> = super::categories::specs()
        .into_iter()
        .filter(|s| ids.iter().any(|id| id == s.id))
        .collect();
    let total = selected.len() as u64;
    let mut reports = Vec::with_capacity(selected.len());

    for (i, spec) in selected.iter().enumerate() {
        util::emit_progress(&app, "clean", spec.id, spec.name, i as u64, total);
        let mut locked = LockTracker::default();
        let mut report = CleanReport {
            category_id: spec.id.to_string(),
            category_name: spec.name.to_string(),
            items_removed: 0,
            bytes_freed: 0,
            errors: Vec::new(),
            locked: 0,
            locked_paths: Vec::new(),
        };
        clean_category(spec, cutoff, to_recycle_bin, &mut report, &mut locked);
        report.locked = locked.count;
        report.locked_paths = locked.samples;
        reports.push(report);
        util::emit_progress(&app, "clean", spec.id, spec.name, i as u64 + 1, total);
    }
    Ok(reports)
}

/// 一键清理：扫描全部垃圾并清理
#[tauri::command]
pub fn clean_all(app: AppHandle, to_recycle_bin: bool) -> Result<Vec<CleanReport>, String> {
    let categories = super::scan::scan_junk(app.clone(), None)?;
    let ids: Vec<String> = categories.iter().map(|c| c.id.clone()).collect();
    clean_junk(app, ids, None, to_recycle_bin)
}
