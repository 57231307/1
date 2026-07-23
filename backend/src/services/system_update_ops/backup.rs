//! 备份与回滚子模块（system_update_ops/backup）
//!
//! 从原 `system_update_service.rs` 迁移 5 个方法：
//! - `create_backup`：创建版本备份目录（`pub(crate)`，供 apply 子模块 `do_update` 调用）
//! - `copy_dir`：递归复制目录（`pub(crate)`，供 apply 子模块 `apply_files` 调用）
//! - `rollback`：从备份路径回滚 backend/frontend/config/VERSION（`pub(crate)`，供 apply 子模块 `do_update` 调用）
//! - `rollback_to_version`：按版本号查找备份并回滚（pub，handler 调用）
//! - `cleanup_old_backups`：保留最近 3 个备份，清理更旧版本（`pub(crate)`，供 apply 子模块 `do_update` 调用）
//!
//! 跨模块依赖：`rollback` / `rollback_to_version` 调用 `apply` 子模块的 `log_update`（`pub(crate)`）；
//! `rollback_to_version` / `cleanup_old_backups` 调用 `status` 子模块的 `list_backup_versions`（pub）。

use crate::services::system_update_service::{SystemUpdateService, UpdateError};
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};

impl SystemUpdateService {
    pub(crate) fn create_backup(&self, version: &str) -> Result<PathBuf, UpdateError> {
        if !self.backup_dir.exists() {
            fs::create_dir_all(&self.backup_dir)?;
        }

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let backup_name = format!("v{}_{}", version, timestamp);
        let backup_path = self.backup_dir.join(&backup_name);

        fs::create_dir_all(&backup_path)?;

        let dirs_to_backup = ["backend", "frontend", "config"];
        for dir in dirs_to_backup {
            let src = self.app_dir.join(dir);
            let dst = backup_path.join(dir);
            if src.exists() {
                self.copy_dir(&src, &dst)?;
            }
        }

        let version_file = self.app_dir.join("VERSION");
        if version_file.exists() {
            fs::copy(&version_file, backup_path.join("VERSION"))?;
        }

        Ok(backup_path)
    }

    pub(crate) fn copy_dir(&self, src: &Path, dst: &Path) -> std::io::Result<()> {
        fs::create_dir_all(dst)?;

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if ty.is_dir() {
                self.copy_dir(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path)?;
            }
        }

        Ok(())
    }

    /// L9 修复（v8 复审）：降为 `pub(crate)`，仅内部 `do_update` 调用，不对外暴露
    pub(crate) fn rollback(&self, backup_path: &Path) -> Result<(), UpdateError> {
        self.log_update(&format!("正在回滚到备份: {:?}", backup_path));

        let dirs_to_restore = ["backend", "frontend", "config"];
        for dir in dirs_to_restore {
            let src = backup_path.join(dir);
            let dst = self.app_dir.join(dir);

            if dst.exists() {
                fs::remove_dir_all(&dst)?;
            }

            if src.exists() {
                self.copy_dir(&src, &dst)?;
            }
        }

        let version_src = backup_path.join("VERSION");
        let version_dst = self.app_dir.join("VERSION");
        if version_src.exists() {
            fs::copy(&version_src, &version_dst)?;
        }

        self.log_update("回滚完成");
        Ok(())
    }

    pub fn rollback_to_version(&self, version: &str) -> Result<String, UpdateError> {
        let backup_versions = self.list_backup_versions();
        let backup_path = backup_versions
            .iter()
            .find(|v| v.starts_with(&format!("v{}", version)))
            .map(|v| self.backup_dir.join(v))
            .ok_or_else(|| UpdateError::BackupError(format!("找不到版本 {} 的备份", version)))?;

        self.rollback(&backup_path)?;
        Ok(format!("已回滚到版本 {}", version))
    }

    pub(crate) fn cleanup_old_backups(&self) {
        let mut versions = self.list_backup_versions();
        if versions.len() > 3 {
            for old_version in versions.drain(3..) {
                let old_path = self.backup_dir.join(&old_version);
                if let Err(e) = fs::remove_dir_all(&old_path) {
                    tracing::warn!("清理旧备份失败: {}", e);
                }
            }
        }
    }
}
