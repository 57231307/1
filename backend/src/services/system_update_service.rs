//! 系统更新服务

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const GITHUB_REPO: &str = "57231307/1";
const GITHUB_API_URL: &str = "https://api.github.com";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalRelease {
    pub version: String,
    pub file_name: String,
    pub file_path: PathBuf,
    pub file_size: u64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalUpdateCheckResult {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub local_release: Option<LocalRelease>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: String,
    pub release_date: String,
    pub changelog: Option<String>,
    pub download_url: Option<String>,
    pub file_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: String,
    pub body: Option<String>,
    pub published_at: String,
    pub assets: Vec<GitHubAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
    pub content_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckResult {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub release_info: Option<GitHubRelease>,
    pub error: Option<String>,
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
    #[error("网络错误：{0}")]
    NetworkError(String),
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

    fn get_releases_dir(&self) -> PathBuf {
        let mut releases_dir = self.app_dir.clone();
        
        if releases_dir.ends_with("backend") || releases_dir.ends_with("backend/target/release") {
            releases_dir = releases_dir.ancestors()
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
                            let created_at: chrono::DateTime<chrono::Utc> = metadata.created()?.into();
                            
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

    fn extract_version_from_filename(&self, filename: &str) -> Option<String> {
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
                    let has_update = self.compare_versions(&current_version, &latest_release.version);
                    
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
            Err(e) => {
                LocalUpdateCheckResult {
                    has_update: false,
                    current_version: current_version.clone(),
                    latest_version: current_version,
                    local_release: None,
                    error: Some(e.to_string()),
                }
            }
        }
    }

    pub async fn apply_local_update(&self, release: &LocalRelease) -> Result<String, UpdateError> {
        self.apply_update(&release.file_path).await
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
        if let Err(e) = self.apply_files(&extract_dir) {
            self.log_update(&format!("应用更新文件失败: {}，正在回滚", e));
            let _ = self.rollback(&backup_path); // 尽量回滚
            return Err(UpdateError::ValidationError(format!("应用文件失败并已回滚: {}", e)));
        }
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

                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Some(mode) = file.unix_mode() {
                        if let Ok(metadata) = fs::metadata(&outpath) {
                            let mut perms = metadata.permissions();
                            perms.set_mode(mode);
                            let _ = fs::set_permissions(&outpath, perms);
                        }
                    }
                }
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
                    // Windows 下尝试重命名运行中的文件，而不是直接删除
                    let old_dst = self.app_dir.join(format!("{}.old", dir));
                    if old_dst.exists() {
                        let _ = fs::remove_dir_all(&old_dst); // 尽量清理之前的遗留
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
            let _ = file.write_all(log_entry.as_bytes());
        }
    }

    pub async fn check_for_updates(&self) -> UpdateCheckResult {
        let current_version = self.get_current_version();
        
        match self.fetch_latest_release().await {
            Ok(release) => {
                let latest_version = release.tag_name.trim_start_matches('v').to_string();
                let has_update = self.compare_versions(&current_version, &latest_version);
                
                UpdateCheckResult {
                    has_update,
                    current_version,
                    latest_version,
                    release_info: Some(release),
                    error: None,
                }
            }
            Err(e) => {
                UpdateCheckResult {
                    has_update: false,
                    current_version: current_version.clone(),
                    latest_version: current_version,
                    release_info: None,
                    error: Some(e.to_string()),
                }
            }
        }
    }

    async fn fetch_latest_release(&self) -> Result<GitHubRelease, UpdateError> {
        let url = format!("{}/repos/{}/releases/latest", GITHUB_API_URL, GITHUB_REPO);
        
        let client = reqwest::Client::builder()
            .user_agent("BingxiERP/1.0")
            .build()
            .map_err(|e| UpdateError::NetworkError(e.to_string()))?;
        
        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| UpdateError::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            return Err(UpdateError::NetworkError(format!(
                "GitHub API返回错误状态: {}",
                response.status()
            )));
        }
        
        let release: GitHubRelease = response
            .json()
            .await
            .map_err(|e| UpdateError::NetworkError(e.to_string()))?;
        
        Ok(release)
    }

    fn compare_versions(&self, current: &str, latest: &str) -> bool {
        let parse_version = |v: &str| -> Vec<u32> {
            v.split('.')
                .filter_map(|s| s.parse().ok())
                .collect()
        };
        
        let current_parts = parse_version(current);
        let latest_parts = parse_version(latest);
        
        for i in 0..std::cmp::max(current_parts.len(), latest_parts.len()) {
            let current_val = current_parts.get(i).unwrap_or(&0);
            let latest_val = latest_parts.get(i).unwrap_or(&0);
            
            if latest_val > current_val {
                return true;
            } else if latest_val < current_val {
                return false;
            }
        }
        
        false
    }

    fn compare_versions_for_sort(&self, a: &str, b: &str) -> std::cmp::Ordering {
        let parse_version = |v: &str| -> Vec<u32> {
            v.split('.')
                .filter_map(|s| s.parse().ok())
                .collect()
        };
        
        let a_parts = parse_version(a);
        let b_parts = parse_version(b);
        
        for i in 0..std::cmp::max(a_parts.len(), b_parts.len()) {
            let a_val = a_parts.get(i).unwrap_or(&0);
            let b_val = b_parts.get(i).unwrap_or(&0);
            
            match b_val.cmp(a_val) {
                std::cmp::Ordering::Equal => continue,
                ord => return ord,
            }
        }
        
        std::cmp::Ordering::Equal
    }

    pub async fn download_update(&self, asset_name: Option<&str>) -> Result<PathBuf, UpdateError> {
        let check_result = self.check_for_updates().await;
        
        if !check_result.has_update {
            return Err(UpdateError::VersionError("当前已是最新版本".to_string()));
        }
        
        let release = check_result.release_info.ok_or_else(|| {
            UpdateError::NetworkError("无法获取发布信息".to_string())
        })?;
        
        let asset = if let Some(name) = asset_name {
            release.assets.iter()
                .find(|a| a.name.contains(name))
                .ok_or_else(|| UpdateError::NetworkError(format!("找不到资源: {}", name)))?
        } else {
            release.assets.iter()
                .find(|a| a.name.ends_with(".zip") || a.name.ends_with(".tar.gz"))
                .ok_or_else(|| UpdateError::NetworkError("找不到更新包".to_string()))?
        };
        
        self.log_update(&format!("开始下载更新包: {}", asset.name));
        
        let download_dir = self.app_dir.join("downloads");
        if !download_dir.exists() {
            fs::create_dir_all(&download_dir)?;
        }
        
        let download_path = download_dir.join(&asset.name);
        
        let client = reqwest::Client::builder()
            .user_agent("BingxiERP/1.0")
            .build()
            .map_err(|e| UpdateError::NetworkError(e.to_string()))?;
        
        let mut response = client
            .get(&asset.browser_download_url)
            .send()
            .await
            .map_err(|e| UpdateError::NetworkError(e.to_string()))?;
        
        let mut file = fs::File::create(&download_path)?;
        
        while let Some(chunk) = response.chunk().await.map_err(|e| {
            UpdateError::NetworkError(e.to_string())
        })? {
            io::copy(&mut chunk.as_ref(), &mut file)?;
        }
        
        self.log_update(&format!("更新包下载完成: {:?}", download_path));
        
        Ok(download_path)
    }

    pub async fn download_and_update(&self) -> Result<String, UpdateError> {
        let download_path = self.download_update(None).await?;
        let result = self.apply_update(&download_path).await?;
        
        if let Err(e) = fs::remove_file(&download_path) {
            self.log_update(&format!("清理下载文件失败: {}", e));
        }
        
        Ok(result)
    }
}

impl Default for SystemUpdateService {
    fn default() -> Self {
        Self::new()
    }
}
