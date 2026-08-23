//! 垃圾清理：分类定义、扫描与清理

pub mod categories;
pub mod clean;
pub mod scan;

use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JunkCategory {
    pub id: String,
    pub name: String,
    pub description: String,
    /// 实际存在的扫描路径（用于展示）
    pub paths: Vec<String>,
    pub file_count: u64,
    pub size_bytes: u64,
    pub risk: String,
    pub requires_admin: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanReport {
    pub category_id: String,
    pub category_name: String,
    pub items_removed: u64,
    pub bytes_freed: u64,
    /// 因被占用而跳过的文件数
    pub locked: u64,
    /// 被占用文件采样路径（最多 20 条）
    pub locked_paths: Vec<String>,
    pub errors: Vec<String>,
}
