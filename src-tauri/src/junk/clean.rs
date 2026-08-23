//! 垃圾清理：删除选中类别的文件

use std::path::Path;

use tauri::AppHandle;
use walkdir::WalkDir;

use super::categories::{resolve_template, CategorySpec, SpecialCategory};
use super::CleanReport;
use crate::util;

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

fn clean_category(spec: &CategorySpec, report: &mut CleanReport) {
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

        // 文件名过滤：只删除匹配的文件，保留目录
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
                if matched {
                    let (f, i) = util::delete_path_force(entry.path(), &mut report.errors);
                    report.bytes_freed += f;
                    report.items_removed += i;
                }
            }
            continue;
        }

        if path.is_dir() {
            let (f, i) = util::delete_path_force(path, &mut report.errors);
            report.bytes_freed += f;
            report.items_removed += i;
        } else if path.is_file() {
            let (f, i) = util::delete_path_force(path, &mut report.errors);
            report.bytes_freed += f;
            report.items_removed += i;
        }
    }
}

#[tauri::command]
pub fn clean_junk(app: AppHandle, ids: Vec<String>) -> Result<Vec<CleanReport>, String> {
    let selected: Vec<CategorySpec> = super::categories::specs()
        .into_iter()
        .filter(|s| ids.iter().any(|id| id == s.id))
        .collect();
    let total = selected.len() as u64;
    let mut reports = Vec::with_capacity(selected.len());

    for (i, spec) in selected.iter().enumerate() {
        util::emit_progress(&app, "clean", spec.id, spec.name, i as u64, total);
        let mut report = CleanReport {
            category_id: spec.id.to_string(),
            category_name: spec.name.to_string(),
            items_removed: 0,
            bytes_freed: 0,
            errors: Vec::new(),
        };
        clean_category(spec, &mut report);
        reports.push(report);
        util::emit_progress(&app, "clean", spec.id, spec.name, i as u64 + 1, total);
    }
    Ok(reports)
}

/// 一键清理：扫描全部垃圾并清理
#[tauri::command]
pub fn clean_all(app: AppHandle) -> Result<Vec<CleanReport>, String> {
    let categories = super::scan::scan_junk(app.clone())?;
    let ids: Vec<String> = categories.iter().map(|c| c.id.clone()).collect();
    clean_junk(app, ids)
}
