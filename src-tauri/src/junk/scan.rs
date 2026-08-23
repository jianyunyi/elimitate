//! 垃圾扫描：统计每个类别的文件数与大小

use std::path::Path;

use tauri::AppHandle;
use walkdir::WalkDir;

use super::categories::{resolve_template, CategorySpec, SpecialCategory};
use super::JunkCategory;
use crate::util;

/// 统计目录内全部文件（跳过重解析点），返回 (文件数, 字节数)
fn count_dir(path: &Path) -> (u64, u64) {
    let mut files = 0u64;
    let mut bytes = 0u64;
    let walker = WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            !(e.path_is_symlink() || e.metadata().map(|m| util::is_reparse(&m)).unwrap_or(false))
        });
    for entry in walker.flatten() {
        if entry.file_type().is_file() {
            files += 1;
            bytes += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    (files, bytes)
}

/// 统计目录内文件名匹配前缀/后缀的文件
fn count_matching(path: &Path, prefixes: &[&str], suffixes: &[&str]) -> (u64, u64) {
    let mut files = 0u64;
    let mut bytes = 0u64;
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
        let matched = prefixes.iter().any(|p| name.starts_with(&p.to_lowercase()))
            || suffixes.iter().any(|s| name.ends_with(&s.to_lowercase()));
        if matched {
            files += 1;
            bytes += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    (files, bytes)
}

fn scan_category(spec: &CategorySpec) -> (Vec<String>, u64, u64) {
    let mut paths: Vec<String> = Vec::new();
    let mut file_count = 0u64;
    let mut size_bytes = 0u64;

    if spec.special == Some(SpecialCategory::RecycleBin) {
        let rb = Path::new("C:\\$Recycle.Bin");
        if rb.exists() {
            paths.push(rb.display().to_string());
            let (c, s) = count_dir(rb);
            file_count += c;
            size_bytes += s;
        }
        return (paths, file_count, size_bytes);
    }

    for t in spec.path_templates {
        let Some(p) = resolve_template(t) else { continue };
        let path = Path::new(&p);
        if !path.exists() {
            continue;
        }
        paths.push(p.clone());
        if path.is_dir() {
            if spec.file_prefixes.is_empty() && spec.file_suffixes.is_empty() {
                let (c, s) = count_dir(path);
                file_count += c;
                size_bytes += s;
            } else {
                let (c, s) = count_matching(path, spec.file_prefixes, spec.file_suffixes);
                file_count += c;
                size_bytes += s;
            }
        } else if path.is_file() {
            file_count += 1;
            size_bytes += path.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    (paths, file_count, size_bytes)
}

#[tauri::command]
pub fn scan_junk(app: AppHandle) -> Result<Vec<JunkCategory>, String> {
    let specs = super::categories::specs();
    let total = specs.len() as u64;
    let mut out = Vec::with_capacity(specs.len());

    for (i, spec) in specs.iter().enumerate() {
        util::emit_progress(&app, "scan", spec.id, spec.name, i as u64, total);
        let (paths, file_count, size_bytes) = scan_category(spec);
        out.push(JunkCategory {
            id: spec.id.to_string(),
            name: spec.name.to_string(),
            description: spec.description.to_string(),
            paths,
            file_count,
            size_bytes,
            risk: spec.risk.to_string(),
            requires_admin: spec.requires_admin,
        });
        util::emit_progress(&app, "scan", spec.id, spec.name, i as u64 + 1, total);
    }
    Ok(out)
}
