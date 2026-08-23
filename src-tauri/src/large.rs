//! 大文件分析：并行扫描磁盘，找出占用空间最大的文件
//!
//! 支持：跳过系统目录、实时进度、取消、打开所在文件夹、删除（回收站/永久）

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use jwalk::WalkDir;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::util;

/// 扫描状态（跨命令共享，用于取消）
#[derive(Default)]
pub struct ScanState {
    pub cancel: Arc<AtomicBool>,
    pub scanned: Arc<AtomicU64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LargeFileItem {
    pub path: String,
    pub size_bytes: u64,
    /// 修改时间（Unix 毫秒，0 表示未知）
    pub modified: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LargeFileReport {
    pub drive: String,
    pub scanned_files: u64,
    pub elapsed_ms: u64,
    pub cancelled: bool,
    pub items: Vec<LargeFileItem>,
}

/// 默认跳过的系统/垃圾目录（根目录下）
const SKIP_DIRS: &[&str] = &[
    "windows",
    "program files",
    "program files (x86)",
    "programdata",
    "$recycle.bin",
    "system volume information",
    "recycler",
];

fn emit_progress_large(app: &AppHandle, scanned: u64, elapsed_ms: u64) {
    let _ = app.emit(
        "large-progress",
        serde_json::json!({ "scanned": scanned, "elapsedMs": elapsed_ms }),
    );
}

/// 扫描指定磁盘，返回 Top N 大文件
#[tauri::command]
pub fn scan_large_files(
    app: AppHandle,
    state: State<'_, ScanState>,
    drive: String,
    top: u32,
    skip_system: bool,
) -> Result<LargeFileReport, String> {
    state.cancel.store(false, Ordering::SeqCst);
    state.scanned.store(0, Ordering::SeqCst);
    let top_n = (top as usize).clamp(10, 500);
    let root = format!("{drive}\\");
    if !Path::new(&root).exists() {
        return Err(format!("磁盘 {drive} 不存在"));
    }
    let started = Instant::now();
    let mut scanned = 0u64;

    // 大顶堆（Reverse 使堆顶为最小）→ 只保留最大的 top_n 个 (大小, 修改毫秒, 路径)
    let heap: Arc<Mutex<BinaryHeap<Reverse<(u64, u64, String)>>>> =
        Arc::new(Mutex::new(BinaryHeap::new()));

    let walker = WalkDir::new(&root)
        .skip_hidden(false)
        .follow_links(false)
        .process_read_dir({
            let skip_system = skip_system;
            move |_, _, _, children| {
                if !skip_system {
                    return;
                }
                children.retain(|child| match child {
                    Ok(entry) => {
                        let name = entry.file_name().to_string_lossy().to_lowercase();
                        !SKIP_DIRS.contains(&name.as_str())
                    }
                    Err(_) => true,
                });
            }
        });

    for entry in walker {
        if state.cancel.load(Ordering::Relaxed) {
            break;
        }
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_file() {
            continue;
        }
        scanned += 1;
        if scanned % 20000 == 0 {
            state.scanned.store(scanned, Ordering::Relaxed);
            emit_progress_large(&app, scanned, started.elapsed().as_millis() as u64);
        }
        let Ok(meta) = entry.metadata() else { continue };
        let size = meta.len();
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let path = entry.path().display().to_string();

        let mut h = heap.lock().unwrap();
        if h.len() < top_n {
            h.push(Reverse((size, modified, path)));
        } else if let Some(&Reverse((min_size, _, _))) = h.peek() {
            if size > min_size {
                h.pop();
                h.push(Reverse((size, modified, path)));
            }
        }
    }

    state.scanned.store(scanned, Ordering::Relaxed);
    emit_progress_large(&app, scanned, started.elapsed().as_millis() as u64);
    let cancelled = state.cancel.load(Ordering::Relaxed);

    let h = heap.lock().unwrap();
    let mut entries: Vec<Reverse<(u64, u64, String)>> = h.iter().cloned().collect();
    drop(h);
    entries.sort(); // Reverse 排序 → 从大到小

    let items: Vec<LargeFileItem> = entries
        .into_iter()
        .map(|Reverse((size, modified, path))| LargeFileItem {
            path,
            size_bytes: size,
            modified,
        })
        .collect();

    Ok(LargeFileReport {
        drive,
        scanned_files: scanned,
        elapsed_ms: started.elapsed().as_millis() as u64,
        cancelled,
        items,
    })
}

/// 取消正在进行的扫描
#[tauri::command]
pub fn cancel_large_scan(state: State<'_, ScanState>) {
    state.cancel.store(true, Ordering::SeqCst);
}

/// 在资源管理器中定位文件
#[tauri::command]
pub fn open_in_explorer(path: String) -> Result<(), String> {
    std::process::Command::new("explorer.exe")
        .arg(format!("/select,{}", path))
        .spawn()
        .map_err(|e| format!("打开资源管理器失败: {e}"))?;
    Ok(())
}

/// 删除前提示用（供前端确认时展示占用情况）
#[tauri::command]
pub fn precheck_locked(paths: Vec<String>) -> Vec<String> {
    let mut locked = Vec::new();
    for p in &paths {
        if util::is_locked(Path::new(p)) {
            locked.push(p.clone());
        }
    }
    locked
}
