//! 版本与状态查询子模块（system_update_ops/status）
//!
//! 从原 `system_update_service.rs` 迁移 7 个方法：
//! - `get_releases_dir`：定位本地 releases 目录（私有，仅本模块 `list_local_releases` 调用）
//! - `list_local_releases`：列出本地 releases 目录下所有 bingxi-erp-*.zip 发布包
//! - `extract_version_from_filename`：从文件名提取版本号（`pub(crate)` 供 facade 测试调用）
//! - `check_local_updates`：检查本地是否有新版本可应用
//! - `get_current_version`：读取 VERSION 文件获取当前版本号
//! - `get_status`：聚合当前版本 / 更新中标记 / 上次更新时间 / 备份版本列表
//! - `list_backup_versions`：列出 backups 目录下所有 v 前缀版本号
//!
//! 跨模块依赖：`check_local_updates` / `list_local_releases` 调用 `github` 子模块的
//! `compare_versions` / `compare_versions_for_sort`（`pub(crate)` 可见性）。

use crate::services::system_update_service::{LocalRelease, LocalUpdateCheckResult, SystemUpdateService, UpdateError, UpdateStatus};
use std::fs;
use std::path::PathBuf;

impl SystemUpdateService {
    fn get_releases_dir(&self) -> PathBuf {
        let mut releases_dir = self.app_dir.clone();

        if releases_dir.ends_with("backend") || releases_dir.ends_with("backend/target/release") {
            releases_dir = releases_dir
                .ancestors()
                .find(|p| p.join("releases").exists() || p.join("backend").exists())
                .unwrap_or(&releases_dir)
                .to_path_buf();
        }

        releases_dir.join("releases")
    }

    pub fn list_local_releases(&self) -> Result<Vec<LocalRelease>, UpdateError> {
        let releases_dir = self.get_releases_dir();

        if !releases_dir.exists() {
            return Ok(Vec::new());
        }

        let mut releases = Vec::new();

        for entry in fs::read_dir(&releases_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.ends_with(".zip") && file_name.starts_with("bingxi-erp-") {
                        if let Some(version) = self.extract_version_from_filename(file_name) {
                            let metadata = fs::metadata(&path)?;
                            let created_at: chrono::DateTime<chrono::Utc> =
                                metadata.created()?.into();

                            releases.push(LocalRelease {
                                version,
                                file_name: file_name.to_string(),
                                file_path: path,
                                file_size: metadata.len(),
                                created_at: created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                            });
                        }
                    }
                }
            }
        }

        releases.sort_by(|a, b| self.compare_versions_for_sort(&a.version, &b.version));
        Ok(releases)
    }

    pub(crate) fn extract_version_from_filename(&self, filename: &str) -> Option<String> {
        let parts: Vec<&str> = filename.split('-').collect();
        if parts.len() >= 3 {
            let version_part = parts[2];
            let version = version_part.trim_end_matches(".zip");
            if !version.is_empty() {
                return Some(version.to_string());
            }
        }
        None
    }

    pub fn check_local_updates(&self) -> LocalUpdateCheckResult {
        let current_version = self.get_current_version();

        match self.list_local_releases() {
            Ok(releases) => {
                if let Some(latest_release) = releases.first() {
                    let has_update =
                        self.compare_versions(&current_version, &latest_release.version);

                    LocalUpdateCheckResult {
                        has_update,
                        current_version,
                        latest_version: latest_release.version.clone(),
                        local_release: Some(latest_release.clone()),
                        error: None,
                    }
                } else {
                    LocalUpdateCheckResult {
                        has_update: false,
                        current_version: current_version.clone(),
                        latest_version: current_version,
                        local_release: None,
                        error: None,
                    }
                }
            }
            Err(e) => LocalUpdateCheckResult {
                has_update: false,
                current_version: current_version.clone(),
                latest_version: current_version,
                local_release: None,
                error: Some(e.to_string()),
            },
        }
    }

    pub fn get_current_version(&self) -> String {
        let version_file = self.app_dir.join("VERSION");
        if version_file.exists() {
            fs::read_to_string(&version_file).unwrap_or_else(|_| "1.0.0".to_string())
        } else {
            "1.0.0".to_string()
        }
    }

    pub fn get_status(&self) -> UpdateStatus {
        let current_version = self.get_current_version();
        let is_updating = self.is_updating.load(std::sync::atomic::Ordering::SeqCst);

        let last_update_time = {
            let log_file = self.app_dir.join("update.log");
            if log_file.exists() {
                fs::metadata(&log_file)
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .map(|t| {
                        let datetime: chrono::DateTime<chrono::Utc> = t.into();
                        datetime.format("%Y-%m-%d %H:%M:%S").to_string()
                    })
            } else {
                None
            }
        };

        let backup_versions = self.list_backup_versions();

        UpdateStatus {
            current_version,
            is_updating,
            last_update_time,
            backup_versions,
        }
    }

    pub fn list_backup_versions(&self) -> Vec<String> {
        if !self.backup_dir.exists() {
            return Vec::new();
        }

        let mut versions = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.backup_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.starts_with("v") {
                            versions.push(name.to_string());
                        }
                    }
                }
            }
        }
        versions.sort();
        versions.reverse();
        versions
    }
}
