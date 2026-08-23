//! 垃圾扫描：统计每个类别的文件数与大小（支持按修改时间过滤）

use std::path::Path;
use std::time::{Duration, SystemTime};

use tauri::AppHandle;
use walkdir::WalkDir;

use super::categories::{resolve_template, CategorySpec, SpecialCategory};
use super::JunkCategory;
use crate::util;

/// 计算年龄过滤的时间阈值（N 天前）；None 表示不过滤
pub fn age_cutoff(max_age_days: Option<u64>) -> Option<SystemTime> {
    max_age_days
        .filter(|d| *d > 0)
        .and_then(|d| SystemTime::now().checked_sub(Duration::from_secs(d * 86400)))
}

/// 文件修改时间是否早于阈值（够龄）。元数据缺失时视为够龄（垃圾位置本就该清理）
fn older_than(meta: &std::fs::Metadata, cutoff: Option<SystemTime>) -> bool {
    match cutoff {
        None => true,
        Some(c) => meta.modified().map(|t| t < c).unwrap_or(true),
    }
}

/// 统计目录内全部文件（跳过重解析点），返回 (文件数, 字节数)
fn count_dir(path: &Path, cutoff: Option<SystemTime>) -> (u64, u64) {
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
            if let Ok(meta) = entry.metadata() {
                if older_than(&meta, cutoff) {
                    files += 1;
                    bytes += meta.len();
                }
            }
        }
    }
    (files, bytes)
}

/// 统计目录内文件名匹配前缀/后缀的文件
fn count_matching(path: &Path, prefixes: &[&str], suffixes: &[&str], cutoff: Option<SystemTime>) -> (u64, u64) {
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
            if let Ok(meta) = entry.metadata() {
                if older_than(&meta, cutoff) {
                    files += 1;
                    bytes += meta.len();
                }
            }
        }
    }
    (files, bytes)
}

fn scan_category(spec: &CategorySpec, cutoff: Option<SystemTime>) -> (Vec<String>, u64, u64) {
    let mut paths: Vec<String> = Vec::new();
    let mut file_count = 0u64;
    let mut size_bytes = 0u64;

    if spec.special == Some(SpecialCategory::RecycleBin) {
        let rb = Path::new("C:\\$Recycle.Bin");
        if rb.exists() {
            paths.push(rb.display().to_string());
            let (c, s) = count_dir(rb, None); // 回收站不过滤年龄
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
                let (c, s) = count_dir(path, cutoff);
                file_count += c;
                size_bytes += s;
            } else {
                let (c, s) = count_matching(path, spec.file_prefixes, spec.file_suffixes, cutoff);
                file_count += c;
                size_bytes += s;
            }
        } else if path.is_file() {
            if let Ok(meta) = path.metadata() {
                if older_than(&meta, cutoff) {
                    file_count += 1;
                    size_bytes += meta.len();
                }
            }
        }
    }
    (paths, file_count, size_bytes)
}

#[tauri::command]
pub fn scan_junk(app: AppHandle, max_age_days: Option<u64>) -> Result<Vec<JunkCategory>, String> {
    let cutoff = age_cutoff(max_age_days);
    let specs = super::categories::specs();
    let total = specs.len() as u64;
    let mut out = Vec::with_capacity(specs.len());

    for (i, spec) in specs.iter().enumerate() {
        util::emit_progress(&app, "scan", spec.id, spec.name, i as u64, total);
        let (paths, file_count, size_bytes) = scan_category(spec, cutoff);
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
