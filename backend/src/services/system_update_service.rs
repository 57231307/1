//! 系统更新服务

use std::fs;
use std::io::{self, Copy};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use sea_orm::DatabaseConnection;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: String,
    pub release_date: String,
    pub changelog: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStatus {
    pub current_version: String,
    pub is_updating: bool,
    pub last_update_time: Option<String>,
    pub backup_versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProgress {
    pub stage: String,
    pub progress: u8,
    pub message: String,
}

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("IO错误：{0}")]
    IoError(#[from] io::Error),
    #[error("解压错误：{0}")]
    UnzipError(String),
    #[error("备份错误：{0}")]
    BackupError(String),
    #[error("验证错误：{0}")]
    ValidationError(String),
    #[error("版本错误：{0}")]
    VersionError(String),
    #[error("更新正在进行中")]
    AlreadyUpdating,
}

pub struct SystemUpdateService {
    app_dir: PathBuf,
    backup_dir: PathBuf,
    is_updating: Arc<std::sync::atomic::AtomicBool>,
}

impl SystemUpdateService {
    pub fn new() -> Self {
        let app_dir = std::env::current_exe()
            .map(|p| p.parent().unwrap_or(Path::new(".")).to_path_buf())
            .unwrap_or_else(|_| PathBuf::from("."));

        let backup_dir = app_dir.join("backups");

        Self {
            app_dir,
            backup_dir,
            is_updating: Arc::new(std::sync::atomic::AtomicBool::new(false)),
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

    pub fn get_update_status(&self) -> UpdateStatus {
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

    pub async fn apply_update(&self, update_file: &Path) -> Result<String, UpdateError> {
        if self.is_updating.swap(true, std::sync::atomic::Ordering::SeqCst) {
            return Err(UpdateError::AlreadyUpdating);
        }

        let result = self.do_update(update_file).await;

        self.is_updating.store(false, std::sync::atomic::Ordering::SeqCst);

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
        self.apply_files(&extract_dir)?;
        self.log_update("文件已更新");

        self.log_update("步骤5: 验证更新结果");
        if !self.verify_update() {
            self.log_update("更新验证失败，正在回滚");
            self.rollback(&backup_path)?;
            return Err(UpdateError::ValidationError("更新验证失败，已回滚".to_string()));
        }

        self.log_update(&format!("更新成功，新版本: {}", new_version));

        self.cleanup_old_backups();

        Ok(format!("系统已成功更新到版本 {}", new_version))
    }

    fn create_backup(&self, version: &str) -> Result<PathBuf, UpdateError> {
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

    fn copy_dir(&self, src: &Path, dst: &Path) -> io::Result<()> {
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

    fn extract_update_package(&self, update_file: &Path) -> Result<PathBuf, UpdateError> {
        let extract_dir = self.app_dir.join("temp_update");

        if extract_dir.exists() {
            fs::remove_dir_all(&extract_dir)?;
        }
        fs::create_dir_all(&extract_dir)?;

        let file = fs::File::open(update_file)?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| UpdateError::UnzipError(e.to_string()))?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| UpdateError::UnzipError(e.to_string()))?;
            let outpath = match file.enclosed_name() {
                Some(path) => extract_dir.join(path),
                None => continue,
            };

            if file.name().ends_with('/') {
                fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        fs::create_dir_all(p)?;
                    }
                }
                let mut outfile = fs::File::create(&outpath)?;
                io::copy(&mut file, &mut outfile)?;
            }
        }

        Ok(extract_dir)
    }

    fn validate_update_package(&self, extract_dir: &Path) -> Result<(), UpdateError> {
        let version_file = extract_dir.join("VERSION");
        if !version_file.exists() {
            return Err(UpdateError::ValidationError("更新包缺少 VERSION 文件".to_string()));
        }

        let manifest_file = extract_dir.join("UPDATE_MANIFEST.json");
        if !manifest_file.exists() {
            return Err(UpdateError::ValidationError("更新包缺少 UPDATE_MANIFEST.json 文件".to_string()));
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
                    fs::remove_dir_all(&dst)?;
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

    pub fn rollback(&self, backup_path: &Path) -> Result<(), UpdateError> {
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

    fn cleanup_old_backups(&self) {
        let mut versions = self.list_backup_versions();
        if versions.len() > 3 {
            for old_version in versions.drain(3..) {
                let old_path = self.backup_dir.join(&old_version);
                if let Err(e) = fs::remove_dir_all(&old_path) {
                    eprintln!("清理旧备份失败: {}", e);
                }
            }
        }
    }

    fn log_update(&self, message: &str) {
        let log_file = self.app_dir.join("update.log");
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S");
        let log_entry = format!("[{}] {}\n", timestamp, message);

        if let Ok(mut file) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
        {
            use std::io::Write;
            let _ = file.write_all(log_entry.as_bytes());
        }
    }
}
