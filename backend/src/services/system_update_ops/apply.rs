//! 更新应用主流程子模块（system_update_ops/apply）
//!
//! 从原 `system_update_service.rs` 迁移 10 个方法：
//! - `apply_local_update`：应用本地发布包（pub，handler 调用，委托 `apply_update`）
//! - `apply_update`：更新主入口，加锁防重入（pub，handler + github 子模块 `download_and_update` 调用）
//! - `do_update`：更新 5 步主流程（备份→解压→校验→应用→验证，私有，仅 `apply_update` 调用）
//! - `extract_update_package`：解压 zip 到 temp_update 目录（私有）
//! - `prepare_extract_dir`：清理并创建解压目录（私有）
//! - `validate_update_package`：校验 VERSION + UPDATE_MANIFEST.json 存在（私有）
//! - `read_version_from_dir`：从目录读取 VERSION 文件（私有）
//! - `apply_files`：把解压目录的 backend/frontend/config/VERSION 应用到 app_dir（私有）
//! - `verify_update`：验证更新后 backend 可执行文件或 VERSION 存在（私有）
//! - `log_update`：追加写入 update.log（`pub(crate)`，供 backup/github 子模块调用）
//!
//! 跨模块依赖：
//! - `do_update` 调用 `status::get_current_version`（pub）、`backup::create_backup` /
//!  `rollback` / `cleanup_old_backups`（`pub(crate)`）
//! - `apply_files` 调用 `backup::copy_dir`（`pub(crate)`）
//! - `extract_update_package` 调用 facade 纯函数 `extract_zip_entry`（`pub(crate)`）
//! - `log_update` 被 `backup::rollback` / `github::download_update` /
//!  `github::download_and_update` 跨模块调用

use crate::services::system_update_service::{
    extract_zip_entry, LocalRelease, SystemUpdateService, UpdateError,
};
use chrono::Utc;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

impl SystemUpdateService {
    pub async fn apply_local_update(&self, release: &LocalRelease) -> Result<String, UpdateError> {
        self.apply_update(&release.file_path).await
    }

    pub async fn apply_update(&self, update_file: &Path) -> Result<String, UpdateError> {
        if self
            .is_updating
            .swap(true, std::sync::atomic::Ordering::SeqCst)
        {
            return Err(UpdateError::AlreadyUpdating);
        }

        let result = self.do_update(update_file).await;

        self.is_updating
            .store(false, std::sync::atomic::Ordering::SeqCst);

        result
    }

    async fn do_update(&self, update_file: &Path) -> Result<String, UpdateError> {
        let current_version = self.get_current_version();
        self.log_update(&format!("开始更新，当前版本: {}", current_version));

        self.log_update("步骤1: 创建备份");
        let backup_path = self.create_backup(&current_version)?;
        self.log_update(&format!("备份已创建: {:?}", backup_path));

        self.log_update("步骤2: 解压更新包");
        let extract_dir = self.extract_update_package(update_file)?;
        self.log_update(&format!("更新包已解压到: {:?}", extract_dir));

        self.log_update("步骤3: 验证更新包");
        self.validate_update_package(&extract_dir)?;
        self.log_update("更新包验证通过");

        let new_version = self.read_version_from_dir(&extract_dir)?;

        self.log_update("步骤4: 应用更新");
        if let Err(e) = self.apply_files(&extract_dir) {
            self.log_update(&format!("应用更新文件失败: {}，正在回滚", e));
            // 批次 98 P2-C 修复（v5 复审）：吞错改日志记录，便于事后排查回滚失败
            if let Err(rollback_err) = self.rollback(&backup_path) {
                tracing::warn!("应用更新失败后回滚也失败: {}", rollback_err);
            }
            return Err(UpdateError::ValidationError(format!(
                "应用文件失败并已回滚: {}",
                e
            )));
        }
        self.log_update("文件已更新");

        self.log_update("步骤5: 验证更新结果");
        if !self.verify_update() {
            self.log_update("更新验证失败，正在回滚");
            self.rollback(&backup_path)?;
            return Err(UpdateError::ValidationError(
                "更新验证失败，已回滚".to_string(),
            ));
        }

        self.log_update(&format!("更新成功，新版本: {}", new_version));

        self.cleanup_old_backups();

        Ok(format!("系统已成功更新到版本 {}", new_version))
    }

    fn extract_update_package(&self, update_file: &Path) -> Result<PathBuf, UpdateError> {
        let extract_dir = self.app_dir.join("temp_update");

        self.prepare_extract_dir(&extract_dir)?;

        let file = fs::File::open(update_file)?;
        let mut archive =
            zip::ZipArchive::new(file).map_err(|e| UpdateError::UnzipError(e.to_string()))?;

        for i in 0..archive.len() {
            let mut zip_entry = archive
                .by_index(i)
                .map_err(|e| UpdateError::UnzipError(e.to_string()))?;
            extract_zip_entry(&mut zip_entry, &extract_dir)?;
        }

        Ok(extract_dir)
    }

    /// 准备解压目录（清理旧目录 + 创建新目录）
    ///
    /// 批次 323 v9 复审低危修复：从 extract_update_package 拆分，保持单一职责。
    fn prepare_extract_dir(&self, extract_dir: &Path) -> Result<(), UpdateError> {
        if extract_dir.exists() {
            fs::remove_dir_all(extract_dir)?;
        }
        fs::create_dir_all(extract_dir)?;
        Ok(())
    }

    fn validate_update_package(&self, extract_dir: &Path) -> Result<(), UpdateError> {
        let version_file = extract_dir.join("VERSION");
        if !version_file.exists() {
            return Err(UpdateError::ValidationError(
                "更新包缺少 VERSION 文件".to_string(),
            ));
        }

        let manifest_file = extract_dir.join("UPDATE_MANIFEST.json");
        if !manifest_file.exists() {
            return Err(UpdateError::ValidationError(
                "更新包缺少 UPDATE_MANIFEST.json 文件".to_string(),
            ));
        }

        Ok(())
    }

    fn read_version_from_dir(&self, dir: &Path) -> Result<String, UpdateError> {
        let version_file = dir.join("VERSION");
        fs::read_to_string(&version_file)
            .map(|v| v.trim().to_string())
            .map_err(|e| UpdateError::VersionError(e.to_string()))
    }

    fn apply_files(&self, extract_dir: &Path) -> Result<(), UpdateError> {
        let dirs_to_update = ["backend", "frontend", "config"];

        for dir in dirs_to_update {
            let src = extract_dir.join(dir);
            let dst = self.app_dir.join(dir);

            if src.exists() {
                if dst.exists() {
                    // Windows 下尝试重命名运行中的文件，而不是直接删除
                    let old_dst = self.app_dir.join(format!("{}.old", dir));
                    if old_dst.exists() {
                        // 批次 98 P2-C 修复（v5 复审）：吞错改日志记录，清理旧 .old 目录失败不阻塞更新
                        if let Err(e) = fs::remove_dir_all(&old_dst) {
                            tracing::warn!("清理旧 .old 目录失败 {:?}: {}", old_dst, e);
                        }
                    }
                    if let Err(e) = fs::rename(&dst, &old_dst) {
                        // 如果重命名失败，退退回直接删除（对非运行中的文件有效）
                        fs::remove_dir_all(&dst).map_err(|_| e)?;
                    }
                }
                self.copy_dir(&src, &dst)?;
            }
        }

        let version_src = extract_dir.join("VERSION");
        let version_dst = self.app_dir.join("VERSION");
        if version_src.exists() {
            fs::copy(&version_src, &version_dst)?;
        }

        Ok(())
    }

    fn verify_update(&self) -> bool {
        let backend_exe = if cfg!(windows) {
            self.app_dir.join("backend").join("bingxi_backend.exe")
        } else {
            self.app_dir.join("backend").join("bingxi_backend")
        };

        if backend_exe.exists() {
            return true;
        }

        let version_file = self.app_dir.join("VERSION");
        version_file.exists()
    }

    pub(crate) fn log_update(&self, message: &str) {
        let log_file = self.app_dir.join("update.log");
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S");
        let log_entry = format!("[{}] {}\n", timestamp, message);

        if let Ok(mut file) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
        {
            // 批次 98 P2-C 修复（v5 复审）：吞错改日志记录，日志写入失败不应阻塞业务
            if let Err(e) = file.write_all(log_entry.as_bytes()) {
                tracing::warn!("写入更新日志失败 {:?}: {}", log_file, e);
            }
        }
    }
}
