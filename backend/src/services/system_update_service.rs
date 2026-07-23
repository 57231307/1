//! 系统更新服务（facade）
//!
//! 本文件为 facade 入口，仅保留：
//! - 公共类型定义（`LocalRelease` / `LocalUpdateCheckResult` / `GitHubRelease` /
//!   `GitHubAsset` / `UpdateCheckResult` / `UpdateStatus` / `UpdateError`）
//! - `SystemUpdateService` struct 定义 + `new` 构造器 + `Default` 实现
//! - 常量 `GITHUB_REPO` / `GITHUB_API_URL`（`pub(crate)`，供 ops 子模块访问）
//! - 纯函数：`parse_version` / `extract_zip_entry` / `set_safe_permissions` /
//!   `validate_download_url` / `validate_asset_name`（`pub(crate)` 供 ops 子模块调用）
//! - 单元测试模块
//!
//! 业务实现已按职责拆分到 [`crate::services::system_update_ops`] 子模块：
//! - `status`：版本号与状态查询 + 本地发布包列表（7 方法）
//! - `apply`：更新应用主流程 + 解压/校验/应用/日志（10 方法）
//! - `backup`：备份创建 + 回滚 + 旧备份清理（5 方法）
//! - `github`：GitHub 远程更新检查 + 下载（8 方法）
//!
//! `app_dir` / `backup_dir` / `is_updating` 字段使用 `pub(crate)` 可见性，
//! system_update_ops 子模块的 impl 块可直接访问。
//! 外部调用路径不变：`crate::services::system_update_service::SystemUpdateService` 等保持稳定。

use crate::utils::error::AppError;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

// GitHub 仓库与 API 地址（pub(crate) 供 github 子模块 `fetch_latest_release` 访问）
pub(crate) const GITHUB_REPO: &str = "57231307/1";
pub(crate) const GITHUB_API_URL: &str = "https://api.github.com";

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

impl From<UpdateError> for AppError {
    fn from(err: UpdateError) -> Self {
        match err {
            UpdateError::IoError(e) => AppError::internal(format!("IO错误: {}", e)),
            UpdateError::UnzipError(e) => AppError::internal(format!("解压错误: {}", e)),
            UpdateError::BackupError(e) => AppError::internal(format!("备份错误: {}", e)),
            UpdateError::ValidationError(e) => AppError::validation(e),
            UpdateError::VersionError(e) => AppError::bad_request(format!("版本错误: {}", e)),
            UpdateError::AlreadyUpdating => AppError::business("更新正在进行中"),
            UpdateError::NetworkError(e) => AppError::internal(format!("网络错误: {}", e)),
        }
    }
}

/// 系统更新服务
///
/// struct 定义保留在 facade，impl 块按职责分散到 `system_update_ops/` 子模块。
/// 字段使用 `pub(crate)` 可见性：status/apply/backup/github 子模块的 impl 块需直接访问。
pub struct SystemUpdateService {
    pub(crate) app_dir: PathBuf,
    pub(crate) backup_dir: PathBuf,
    pub(crate) is_updating: Arc<std::sync::atomic::AtomicBool>,
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
}

impl Default for SystemUpdateService {
    fn default() -> Self {
        Self::new()
    }
}

// =====================================================
// 批次 322 v9 复审低危修复：版本号解析共享函数
// =====================================================

/// 解析语义版本号字符串为数字数组
///
/// 批次 322 v9 复审低危修复：抽取 `compare_versions` 和 `compare_versions_for_sort`
/// 中重复的版本号解析逻辑为共享函数，遵循 DRY 原则。
///
/// `pub(crate)`：github 子模块的 `compare_versions` / `compare_versions_for_sort` 调用。
///
/// # 示例
/// - "1.2.3" → [1, 2, 3]
/// - "2.0" → [2, 0]
/// - "1.0.0-beta" → [1, 0, 0]（非数字部分被 filter_map 忽略）
pub(crate) fn parse_version(v: &str) -> Vec<u32> {
    v.split('.').filter_map(|s| s.parse().ok()).collect()
}

// =====================================================
// 批次 323 v9 复审低危修复：extract_zip_entry 拆分
// =====================================================

/// 解压单个 zip 条目到指定目录（含路径校验 + 权限掩码）
///
/// 批次 323 v9 复审低危修复：从 extract_update_package 拆分，保持单一职责。
/// 原函数 60+ 行混合了目录准备、循环遍历、路径校验、权限设置多种职责。
///
/// `pub(crate)`：apply 子模块的 `extract_update_package` 调用。
///
/// # 安全
/// - 路径校验：`enclosed_name` + `starts_with` 双重防护 Tar Slip 路径穿越
/// - 权限掩码：`set_safe_permissions` 重置 SUID/SGID/sticky bit（P0-2 修复）
pub(crate) fn extract_zip_entry(
    zip_entry: &mut zip::read::ZipFile,
    extract_dir: &Path,
) -> Result<(), UpdateError> {
    let filepath = zip_entry.enclosed_name().ok_or_else(|| {
        UpdateError::ValidationError("更新包中包含无效的文件路径".to_string())
    })?;

    let outpath = extract_dir.join(filepath);

    if !outpath.starts_with(extract_dir) {
        return Err(UpdateError::ValidationError(
            "检测到路径遍历攻击，更新包中包含不安全的路径".to_string(),
        ));
    }

    if zip_entry.name().ends_with('/') {
        std::fs::create_dir_all(&outpath)?;
        // P0-2 修复（v9 复审）：目录权限掩码必须在目录分支内设置
        #[cfg(unix)]
        {
            if let Some(mode) = zip_entry.unix_mode() {
                set_safe_permissions(&outpath, mode, true);
            }
        }
    } else {
        if let Some(p) = outpath.parent() {
            if !p.exists() {
                std::fs::create_dir_all(p)?;
            }
        }
        let mut outfile = std::fs::File::create(&outpath)?;
        io::copy(zip_entry, &mut outfile)?;

        // P0-2 修复（v9 复审）：文件权限掩码在文件分支内设置（mode & 0o600）
        #[cfg(unix)]
        {
            if let Some(mode) = zip_entry.unix_mode() {
                set_safe_permissions(&outpath, mode, false);
            }
        }
    }
    Ok(())
}

// =====================================================
// TS-S-7 安全加固：下载域名校验
// =====================================================

/// 设置安全权限掩码（P0-2 修复 v9 复审）
/// is_dir=true 时应用 0o755（所有者可写，其他可读可执行），
/// is_dir=false 时应用 0o600（仅所有者可读写），
/// 重置 SUID/SGID/粘性位，防止恶意更新包设置特殊权限位导致权限提升
///
/// 私有可见性：仅本 facade 的 `extract_zip_entry` 调用。
#[cfg(unix)]
fn set_safe_permissions(path: &Path, mode: u32, is_dir: bool) {
    use std::os::unix::fs::PermissionsExt;
    let safe_mode = if is_dir { mode & 0o755 } else { mode & 0o600 };
    if let Ok(metadata) = std::fs::metadata(path) {
        let mut perms = metadata.permissions();
        perms.set_mode(safe_mode);
        // 权限设置失败不阻塞解压（与原批次 98 P2-C 行为一致）
        if let Err(e) = std::fs::set_permissions(path, perms) {
            tracing::warn!("设置权限失败 {:?}: {}", path, e);
        }
    }
}

/// 校验下载 URL 的域名是否为允许的 GitHub 域名
///
/// `pub(crate)`：github 子模块的 `download_update` / `build_safe_download_client` 调用。
pub(crate) fn validate_download_url(url_str: &str) -> Result<(), UpdateError> {
    let parsed = url::Url::parse(url_str)
        .map_err(|e| UpdateError::NetworkError(format!("无效的下载 URL: {e}")))?;

    // 仅允许 HTTPS
    if parsed.scheme() != "https" {
        return Err(UpdateError::NetworkError(format!(
            "下载 URL 必须使用 HTTPS，当前 scheme: {}",
            parsed.scheme()
        )));
    }

    let host = parsed.host_str().unwrap_or("");
    let allowed_hosts = ["github.com", "objects.githubusercontent.com"];

    if !allowed_hosts.contains(&host) {
        return Err(UpdateError::NetworkError(format!(
            "下载域名 {host} 不在允许列表中（仅允许 github.com / objects.githubusercontent.com）"
        )));
    }

    Ok(())
}

/// M-2 修复（v9 复审）：校验 asset.name 防止路径穿越
/// asset.name 来自 GitHub API，若账号被入侵可设置为恶意路径
/// 仅允许字母、数字、点、下划线、连字符，拒绝路径分隔符和特殊字符
///
/// `pub(crate)`：github 子模块的 `download_update` 调用。
pub(crate) fn validate_asset_name(name: &str) -> Result<(), UpdateError> {
    if name.is_empty() {
        return Err(UpdateError::ValidationError("asset.name 为空".to_string()));
    }

    // 拒绝路径穿越和绝对路径
    if name.contains('/') || name.contains('\\') || name.contains("..") || name.starts_with('.') {
        return Err(UpdateError::ValidationError(format!(
            "asset.name 包含不安全字符: {name}"
        )));
    }

    // 仅允许字母、数字、点、下划线、连字符
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '.' || c == '_' || c == '-')
    {
        return Err(UpdateError::ValidationError(format!(
            "asset.name 包含非法字符: {name}"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// M8 测试：validate_download_url 合法 GitHub URL 通过
    #[test]
    fn test_validate_download_url_valid() {
        assert!(validate_download_url("https://github.com/57231307/1/releases/download/v1.0.0/pkg.zip").is_ok());
        assert!(validate_download_url("https://objects.githubusercontent.com/assets/123").is_ok());
    }

    /// M8 测试：validate_download_url 非 HTTPS 被拒绝
    #[test]
    fn test_validate_download_url_not_https() {
        assert!(validate_download_url("http://github.com/repo/releases").is_err());
    }

    /// M8 测试：validate_download_url 非允许域名被拒绝
    #[test]
    fn test_validate_download_url_invalid_host() {
        assert!(validate_download_url("https://evil.com/exploit").is_err());
        assert!(validate_download_url("https://169.254.169.254/metadata").is_err());
    }

    /// M8 测试：validate_download_url 无效 URL 被拒绝
    #[test]
    fn test_validate_download_url_invalid_url() {
        assert!(validate_download_url("not-a-url").is_err());
        assert!(validate_download_url("").is_err());
    }

    /// M8 测试：compare_versions 新版本大于旧版本返回 true
    #[test]
    fn test_compare_versions_newer() {
        let svc = SystemUpdateService::new();
        assert!(svc.compare_versions("1.0.0", "1.0.1"));
        assert!(svc.compare_versions("1.0.0", "2.0.0"));
        assert!(svc.compare_versions("2026.7.1", "2026.7.2"));
    }

    /// M8 测试：compare_versions 旧版本大于等于新版本返回 false
    #[test]
    fn test_compare_versions_older_or_equal() {
        let svc = SystemUpdateService::new();
        assert!(!svc.compare_versions("1.0.1", "1.0.0"));
        assert!(!svc.compare_versions("1.0.0", "1.0.0"));
    }

    /// M8 测试：extract_version_from_filename 正确提取版本号
    #[test]
    fn test_extract_version_from_filename() {
        let svc = SystemUpdateService::new();
        assert_eq!(
            svc.extract_version_from_filename("bingxi-erp-1.0.0.zip"),
            Some("1.0.0".to_string())
        );
        assert_eq!(
            svc.extract_version_from_filename("bingxi-erp-2026.7.12.zip"),
            Some("2026.7.12".to_string())
        );
    }

    /// M8 测试：extract_version_from_filename 无效文件名返回 None
    #[test]
    fn test_extract_version_from_filename_invalid() {
        let svc = SystemUpdateService::new();
        assert_eq!(svc.extract_version_from_filename("invalid.zip"), None);
        assert_eq!(svc.extract_version_from_filename("bingxi-erp-.zip"), None);
    }

    /// P0-2 测试（v9 复审）：set_safe_permissions 文件分支应用 0o600 掩码
    #[cfg(unix)]
    #[test]
    fn test_set_safe_permissions_file_mode() {
        use std::os::unix::fs::PermissionsExt;
        let temp = std::env::temp_dir().join("bingxi_test_perm_file");
        let _ = std::fs::write(&temp, b"test");
        // 模拟恶意 zip 设置 SUID + SGID + sticky + 全读写（0o7777）
        set_safe_permissions(&temp, 0o7777, false);
        let mode = std::fs::metadata(&temp).unwrap().permissions().mode();
        // 0o7777 & 0o600 = 0o600
        assert_eq!(mode & 0o7777, 0o600, "文件权限应为 0o600，实际 {:#o}", mode);
        let _ = std::fs::remove_file(&temp);
    }

    /// P0-2 测试（v9 复审）：set_safe_permissions 目录分支应用 0o755 掩码
    #[cfg(unix)]
    #[test]
    fn test_set_safe_permissions_dir_mode() {
        use std::os::unix::fs::PermissionsExt;
        let temp = std::env::temp_dir().join("bingxi_test_perm_dir");
        let _ = std::fs::create_dir(&temp);
        // 模拟恶意 zip 设置 SUID + SGID + sticky + 全读写（0o7777）
        set_safe_permissions(&temp, 0o7777, true);
        let mode = std::fs::metadata(&temp).unwrap().permissions().mode();
        // 0o7777 & 0o755 = 0o755
        assert_eq!(mode & 0o7777, 0o755, "目录权限应为 0o755，实际 {:#o}", mode);
        let _ = std::fs::remove_dir(&temp);
    }

    /// M-2 测试（v9 复审）：合法 asset.name 通过校验
    #[test]
    fn test_validate_asset_name_valid() {
        assert!(validate_asset_name("bingxi-erp-1.0.0.zip").is_ok());
        assert!(validate_asset_name("release-2026.7.12.tar.gz").is_ok());
        assert!(validate_asset_name("update_v2.tar.gz").is_ok());
    }

    /// M-2 测试（v9 复审）：路径穿越 asset.name 被拒绝
    #[test]
    fn test_validate_asset_name_path_traversal() {
        assert!(validate_asset_name("../../../etc/cron.d/evil").is_err());
        assert!(validate_asset_name("..\\..\\windows\\evil").is_err());
        assert!(validate_asset_name("/etc/passwd").is_err());
        assert!(validate_asset_name(".hidden").is_err());
        assert!(validate_asset_name("..").is_err());
    }

    /// M-2 测试（v9 复审）：特殊字符 asset.name 被拒绝
    #[test]
    fn test_validate_asset_name_special_chars() {
        assert!(validate_asset_name("file name.zip").is_err()); // 空格
        assert!(validate_asset_name("file;evil.zip").is_err()); // 分号
        assert!(validate_asset_name("file|evil.zip").is_err()); // 管道符
        assert!(validate_asset_name("").is_err()); // 空
    }

    // ============ 批次 322 v9 复审低危修复：parse_version 单元测试 ============

    /// 测试 parse_version 正确解析标准语义版本号
    #[test]
    fn test_parse_version_standard() {
        assert_eq!(parse_version("1.2.3"), vec![1, 2, 3]);
        assert_eq!(parse_version("2.0"), vec![2, 0]);
        assert_eq!(parse_version("2026.7.12"), vec![2026, 7, 12]);
    }

    /// 测试 parse_version 解析带预发布标签的版本号（非数字部分被忽略）
    #[test]
    fn test_parse_version_pre_release() {
        assert_eq!(parse_version("1.0.0-beta"), vec![1, 0, 0]);
        assert_eq!(parse_version("2.0.0-rc.1"), vec![2, 0, 0]);
    }

    /// 测试 parse_version 解析空字符串和无效输入
    #[test]
    fn test_parse_version_invalid() {
        assert!(parse_version("").is_empty());
        assert!(parse_version("abc").is_empty());
        assert_eq!(parse_version("1.a.3"), vec![1, 3]);
    }
}
