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
use std::io::Read;
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
    /// GitHub Releases assets API 返回的官方校验值，形如 `sha256:<hex>`（服务端上传时计算）。
    /// 作为 SHA-256 强校验的兜底源（优先 CI 上传的 `.sha256` 资产，其次此 digest）。
    /// `#[serde(default)]`：老/无 digest 的 release 仍可反序列化（digest=None 时按 fail-closed 处理）。
    #[serde(default)]
    pub digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckResult {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub release_info: Option<GitHubRelease>,
    /// 当前版本对应的 GitHub Release（按 tag 精确查询；历史/开发构建无匹配 tag 时为 None）
    pub current_release_info: Option<GitHubRelease>,
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
    /// 官方 SHA-256 校验值存在但与本地重算结果不一致 → 文件损坏或被投毒，拒绝 apply。
    #[error("完整性校验失败：{0}")]
    IntegrityError(String),
    /// 两官方校验源（CI `.sha256` 资产 / API `assets[].digest`）均不可得 →
    /// fail-closed 拒绝 apply（严禁因取不到校验值而放行）。
    #[error("校验值不可用：{0}")]
    ChecksumUnavailable(String),
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
            // 完整性/校验值错误属服务端安全护栏判定，映射为 internal（不外泄细节），
            // 真实原因已在 tracing 中文日志留痕，出参按 AppError 统一脱敏。
            UpdateError::IntegrityError(e) => AppError::internal(format!("完整性校验失败: {}", e)),
            UpdateError::ChecksumUnavailable(e) => {
                AppError::internal(format!("校验值不可用: {}", e))
            }
        }
    }
}

/// 系统更新服务
/// struct 定义保留在 facade，impl 块按职责分散到 `system_update_ops/` 子模块。；字段使用 `pub(crate)` 可见性：status/apply/backup/github 子模块的 impl 块需直接访问。
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
/// 批次 322 v9 复审低危修复：抽取 `compare_versions` 和 `compare_versions_for_sort`；中重复的版本号解析逻辑为共享函数，遵循 DRY 原则。；`pub(crate)`：github 子模块的 `compare_versions` / `compare_versions_for_sort` 调用。；# 示例；"1.2.3" → [1, 2, 3]；"2.0" → [2, 0]；"1.0.0-beta" → [1, 0, 0]（非数字部分被 filter_map 忽略）
pub fn parse_version(v: &str) -> Vec<u32> {
    let mut result = Vec::new();
    for segment in v.split('.') {
        if let Some((n, _)) = segment.split_once('-') {
            if let Ok(n) = n.parse() {
                result.push(n);
            }
            break;
        }
        if let Ok(n) = segment.parse() {
            result.push(n);
        }
    }
    result
}

// =====================================================
// 编译期权威四段版本 + 三段(MD 折叠)↔四段 跨格式确定性反解
// =====================================================

/// 本项目 CalVer 特征年份下限。首段 >= 此值才认定为本项目双编码版本，
/// 方可对三段 MD 折叠格式做确定性反解；否则不臆测（退回 element-wise）。
const PROJECT_CALVER_MIN_YEAR: u32 = 2000;

/// 权威当前版本：编译期注入的四段版本（`BINGXI_RELEASE_VERSION`，与 release tag 同格式）优先，
/// 未注入（本地开发 / 历史二进制）时回退构建内嵌三段 `CARGO_PKG_VERSION`。
///
/// 回退分支用 [`std::sync::Once`] 在首次调用时 `tracing::warn!` 一次说明跨格式比较将走
/// MD 反解兜底，不静默。后端 `get_current_version` 与 CLI `cmd_upgrade` 同源调用此函数，
/// 避免各写一份注入/回退逻辑。
pub fn authoritative_current_version() -> String {
    static FALLBACK_WARNED: std::sync::Once = std::sync::Once::new();
    if let Some(v) = option_env!("BINGXI_RELEASE_VERSION")
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return v.to_string();
    }
    FALLBACK_WARNED.call_once(|| {
        tracing::warn!(
            "未注入编译期权威版本 BINGXI_RELEASE_VERSION，回退构建内嵌三段版本 \
             CARGO_PKG_VERSION={}；跨格式（三段 current vs 四段 tag）版本比较将走 MD 反解兜底",
            env!("CARGO_PKG_VERSION")
        );
    });
    env!("CARGO_PKG_VERSION").to_string()
}

/// 将解析后的版本段数归一为本项目四元组 `(year, month, day, time)`。
///
/// - 四段（tag / 注入格式）`[Y, M, D, T]` → 透传。
/// - 三段（Cargo 格式，月日折叠进第二段）`[Y, MD, T]`：当首段为本项目 CalVer 年份
///   （`>= PROJECT_CALVER_MIN_YEAR`）且 MD 可无损反解出合法月/日（月 1..=12、日 1..=31）时，
///   反解为 `[Y, MD/100, MD%100, T]`；否则 `None`（不可归类，交由调用方决定 element-wise
///   或 fail-open，均不臆测）。
/// - 其它段数（含 0）一律 `None`。
///
/// 该函数为纯函数，被 `normalize_versions_for_compare`（后端比较归一）与 CLI
/// `check_version_downgrade` 共用，MD 反解公式只此一份。
pub fn to_calver_quad(parts: &[u32]) -> Option<[u32; 4]> {
    match parts.len() {
        4 => Some([parts[0], parts[1], parts[2], parts[3]]),
        3 => {
            let year = parts[0];
            let md = parts[1];
            let time = parts[2];
            if year < PROJECT_CALVER_MIN_YEAR {
                return None;
            }
            let month = md / 100;
            let day = md % 100;
            if (1..=12).contains(&month) && (1..=31).contains(&day) {
                Some([year, month, day, time])
            } else {
                None
            }
        }
        _ => None,
    }
}

/// 比较前归一：返回一对等长（或原样）的数字段，供 `compare_versions` /
/// `compare_versions_for_sort` 逐段数值比较。
///
/// 归一策略（保证既有同段数用例零回归）：
/// 1. 两侧段数相同 → 原样返回，维持逐段比（如 `1.0.0` vs `1.0.1`、`2026.7.1` vs `2026.7.2`）。
/// 2. 段数为 3 vs 4 且两侧首段均 >= 本项目 CalVer 年份 → 把三段 `[Y, MD, T]` 反解为四元组
///    `[Y, MD/100, MD%100, T]`，与四段 `[Y, M, D, T]` 对齐成同长 4 元组逐段比。
/// 3. 段数 3 vs 4 但年份非本项目 CalVer → 不臆测，原样返回（调用方 element-wise 补 0）。
/// 4. 段数 3 vs 4 且年份符合但三段 MD 反解落非法月/日 → `tracing::warn!` 记录"版本方案无法
///    确定"后原样返回（element-wise 补 0），不强行归类。
/// 5. 其它长度差（含 0 段 / 2 段 vs 4 段 等）→ 不臆测，原样返回。
pub(crate) fn normalize_versions_for_compare(a: &str, b: &str) -> (Vec<u32>, Vec<u32>) {
    let a_parts = parse_version(a);
    let b_parts = parse_version(b);

    // 1. 同段数：原样逐段比
    if a_parts.len() == b_parts.len() {
        return (a_parts, b_parts);
    }

    // 仅处理 3↔4 跨格式，其它长度差不臆测
    let a_three_b_four = a_parts.len() == 3 && b_parts.len() == 4;
    let b_three_a_four = b_parts.len() == 3 && a_parts.len() == 4;
    if !(a_three_b_four || b_three_a_four) {
        return (a_parts, b_parts);
    }

    let three = if a_three_b_four { &a_parts } else { &b_parts };
    let four = if a_three_b_four { &b_parts } else { &a_parts };

    // 2. 两侧首段须均为本项目 CalVer 年份，否则不归类
    let calver_year = three.first().copied().unwrap_or(0) >= PROJECT_CALVER_MIN_YEAR
        && four.first().copied().unwrap_or(0) >= PROJECT_CALVER_MIN_YEAR;
    if !calver_year {
        return (a_parts, b_parts);
    }

    match (to_calver_quad(three), to_calver_quad(four)) {
        // 2. 三段成功反解为四元组，与四段对齐逐段比
        (Some(t), Some(f)) => {
            let (tv, fv) = (t.to_vec(), f.to_vec());
            if a_three_b_four { (tv, fv) } else { (fv, tv) }
        }
        // 4. 三段形似 CalVer 但 MD 反解落非法月/日：记录不可判定后退回 element-wise
        _ => {
            tracing::warn!(
                "版本跨格式比较不可判定：三段 MD 折叠无法反解出合法月/日（a={} b={}），\
                 退回逐段 element-wise 补 0 比较",
                a,
                b
            );
            (a_parts, b_parts)
        }
    }
}

// =====================================================
// 批次 323 v9 复审低危修复：extract_zip_entry 拆分
// =====================================================

/// 解压单个 zip 条目到指定目录（含路径校验 + 权限掩码）
/// 批次 323 v9 复审低危修复：从 extract_update_package 拆分，保持单一职责。；原函数 60+ 行混合了目录准备、循环遍历、路径校验、权限设置多种职责。；`pub(crate)`：apply 子模块的 `extract_update_package` 调用。；# 安全；路径校验：`enclosed_name` + `starts_with` 双重防护 Tar Slip 路径穿越；权限掩码：`set_safe_permissions` 重置 SUID/SGID/sticky bit（P0-2 修复）
pub(crate) fn extract_zip_entry(
    zip_entry: &mut zip::read::ZipFile,
    extract_dir: &Path,
) -> Result<u64, UpdateError> {
    // P1-03-6 修复：返回单文件解压字节数，供调用方累计总解压大小防 zip bomb
    let filepath = zip_entry
        .enclosed_name()
        .ok_or_else(|| UpdateError::ValidationError("更新包中包含无效的文件路径".to_string()))?;

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
        Ok(0)
    } else {
        if let Some(p) = outpath.parent() {
            if !p.exists() {
                std::fs::create_dir_all(p)?;
            }
        }
        let mut outfile = std::fs::File::create(&outpath)?;
        // P1-03-6 修复：限制单文件解压大小 100MB，防 zip bomb
        const MAX_SINGLE_FILE_SIZE: u64 = 100 * 1024 * 1024;
        let mut limited = zip_entry.take(MAX_SINGLE_FILE_SIZE + 1);
        let copied = io::copy(&mut limited, &mut outfile)?;
        if copied > MAX_SINGLE_FILE_SIZE {
            return Err(UpdateError::ValidationError(format!(
                "单文件解压大小超过限制 ({}MB)，疑似 zip bomb",
                MAX_SINGLE_FILE_SIZE / 1024 / 1024
            )));
        }

        // P0-2 修复（v9 复审）：文件权限掩码在文件分支内设置（mode & 0o600）
        #[cfg(unix)]
        {
            if let Some(mode) = zip_entry.unix_mode() {
                set_safe_permissions(&outpath, mode, false);
            }
        }
        Ok(copied)
    }
}

// =====================================================
// TS-S-7 安全加固：下载域名校验
// =====================================================

/// 设置安全权限掩码（P0-2 修复 v9 复审）
/// is_dir=true 时应用 0o755（所有者可写，其他可读可执行），；is_dir=false 时应用 0o600（仅所有者可读写），；重置 SUID/SGID/粘性位，防止恶意更新包设置特殊权限位导致权限提升；私有可见性：仅本 facade 的 `extract_zip_entry` 调用。
#[cfg(unix)]
pub fn set_safe_permissions(path: &Path, mode: u32, is_dir: bool) {
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

/// 官方 GitHub 域白名单（信任模型：校验基准**只**从这些域取；镜像仅搬 tar 字节）。
pub(crate) const OFFICIAL_DOWNLOAD_HOSTS: &[&str] =
    &["github.com", "objects.githubusercontent.com"];

/// 下载/重定向链允许的 host：官方域 ∪ `config.update.mirrors` 派生 host ∪（默认档）内置默认镜像派生 host。
/// 仅用于**字节下载** URL 与重定向终点复核；校验值取源走 `is_official_digest_host`（官方域收窄）。
/// 内置默认镜像（`DEFAULT_RELEASE_MIRRORS`）在 `use_default_mirrors==true` 时并入允许域，
/// 使 `{mirror}/{official}` 候选能过 `validate_download_url`；但**绝不**并入
/// `OFFICIAL_DOWNLOAD_HOSTS`，故这些 host 在 `is_official_digest_host` 恒为 false（取不到校验值）。
fn allowed_download_hosts() -> Vec<String> {
    let cfg = crate::config::settings::global_update_config();
    let mut hosts: Vec<String> = OFFICIAL_DOWNLOAD_HOSTS
        .iter()
        .map(|s| s.to_string())
        .collect();

    // 运维显式镜像 +（默认档）内置默认镜像 → 派生 host 并入字节下载允许域（大小写无关去重）。
    let mut mirror_sources: Vec<String> = cfg.mirrors.clone();
    if cfg.use_default_mirrors {
        mirror_sources.extend(
            crate::config::settings::DEFAULT_RELEASE_MIRRORS
                .iter()
                .map(|s| s.to_string()),
        );
    }

    for raw in &mirror_sources {
        if let Some(h) = url::Url::parse(raw.trim())
            .ok()
            .and_then(|u| u.host_str().map(|h| h.to_string()))
        {
            if !hosts
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(&h))
            {
                hosts.push(h);
            }
        }
    }
    hosts
}

/// host 是否为下载允许域（官方 ∪ 配置镜像）。
fn is_allowed_download_host(host: &str) -> bool {
    allowed_download_hosts()
        .iter()
        .any(|h| h.eq_ignore_ascii_case(host))
}

/// host 是否为**官方**域（校验值取源专用收窄，绝不从镜像取校验值）。
pub fn is_official_digest_host(host: &str) -> bool {
    OFFICIAL_DOWNLOAD_HOSTS
        .iter()
        .any(|h| h.eq_ignore_ascii_case(host))
}

/// 校验**下载/字节** URL：scheme=https + host ∈（官方域 ∪ 配置镜像派生 host）。
/// `pub(crate)`：github 子模块 `download_update` / `build_safe_download_client` 调用。
pub fn validate_download_url(url_str: &str) -> Result<(), UpdateError> {
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
    if !is_allowed_download_host(host) {
        return Err(UpdateError::NetworkError(format!(
            "下载域名 {host} 不在允许列表中（官方域 ∪ config.update.mirrors ∪ 内置默认镜像 派生 host）"
        )));
    }

    Ok(())
}

/// 校验**校验值取源** URL：scheme=https + host ∈ 官方域（收窄，不含镜像， 信任模型）。
/// CI 上传的官方 `.sha256`/`.md5` 资产其 `browser_download_url` 必须过此判定才可信。
pub fn validate_official_digest_url(url_str: &str) -> Result<(), UpdateError> {
    let parsed = url::Url::parse(url_str)
        .map_err(|e| UpdateError::NetworkError(format!("无效的校验值 URL: {e}")))?;

    if parsed.scheme() != "https" {
        return Err(UpdateError::NetworkError(format!(
            "校验值 URL 必须使用 HTTPS，当前 scheme: {}",
            parsed.scheme()
        )));
    }

    let host = parsed.host_str().unwrap_or("");
    if !is_official_digest_host(host) {
        return Err(UpdateError::IntegrityError(format!(
            "校验值取源 host {host} 非官方域，拒绝（严禁从镜像获取校验基准，防连带投毒）"
        )));
    }

    Ok(())
}

// =====================================================
// 完整性校验纯函数（SHA-256 为权威安全判据）
// =====================================================

/// 流式计算文件 SHA-256，返回小写 64 位 hex（用仓内既有 `sha2` + `hex`，不新增依赖）。
/// 大 tar 分块读，避免整包进内存。
pub(crate) fn sha256_hex_of_file(path: &Path) -> Result<String, UpdateError> {
    use sha2::{Digest, Sha256};
    use std::io::BufReader;

    let file = std::fs::File::open(path)?;
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let mut hasher = Sha256::new();
    std::io::copy(&mut reader, &mut hasher)?;
    Ok(hex::encode(hasher.finalize()))
}

/// 解析 `sha256sum`/`md5sum` 输出行（格式 `<hash>  <filename>`），返回小写 hash（空白/换行前首字段）。
/// 空或不含 hash → `None`（调用方据此判定校验值不可得，fail-closed）。
pub(crate) fn parse_checksum_line(line: &str) -> Option<String> {
    let first = line.split_whitespace().next()?;
    let trimmed = first.trim().to_lowercase();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// 从 GitHub API `assets[].digest`（形如 `sha256:<hex>`）提取小写 hex；
/// 非 sha256 前缀或格式异常返回 `None`（调用方据此回退或 fail-closed）。
pub(crate) fn sha256_from_api_digest(digest: &str) -> Option<String> {
    let rest = digest.trim().strip_prefix("sha256:")?;
    let hex_only = rest.trim().to_lowercase();
    // 合法 sha256 hex：64 个 [0-9a-f]
    if hex_only.len() == 64 && hex_only.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(hex_only)
    } else {
        None
    }
}

/// 从官方 Release 中查找与主包对应的 `.sha256` 资产（CI 上传的第一方校验资产，优先级最高）。
/// 匹配 `name == format!("{}.sha256", asset_name)`（CI 产物 `release-<ver>.tar.gz.sha256`）。
pub(crate) fn find_official_sha256_asset<'a>(
    assets: &'a [GitHubAsset],
    asset_name: &str,
) -> Option<&'a GitHubAsset> {
    let want = format!("{}.sha256", asset_name);
    assets.iter().find(|a| a.name == want)
}

/// 从官方 Release 中查找与主包对应的 `.md5` 资产（仅作**补充记录**，非安全门槛，见 verify 调用处注释）。
pub(crate) fn find_official_md5_asset<'a>(
    assets: &'a [GitHubAsset],
    asset_name: &str,
) -> Option<&'a GitHubAsset> {
    let want = format!("{}.md5", asset_name);
    assets.iter().find(|a| a.name == want)
}

/// 依据校验优先级解析**期望的 SHA-256**：
/// ① 优先 CI 上传的官方 `.sha256` 资产内容（`official_sha256_content`，已由调用方限定官方域取回）；
/// ② 兜底 GitHub API `assets[].digest`（`api_digest`）。
/// 两者皆无 → `Err(ChecksumUnavailable)`（fail-closed）。返回值均为小写 hex。
/// 纯函数：不触网、不做本地计算，便于单测覆盖决策树。
pub(crate) fn resolve_expected_sha256(
    official_sha256_content: Option<&str>,
    api_digest: Option<&str>,
) -> Result<String, UpdateError> {
    // 优先级 ①：官方 .sha256 资产
    if let Some(content) = official_sha256_content
        && let Some(expected) = parse_checksum_line(content)
    {
        return Ok(expected);
    }
    // 优先级 ②：GitHub API digest 兜底
    if let Some(d) = api_digest
        && let Some(expected) = sha256_from_api_digest(d)
    {
        return Ok(expected);
    }
    Err(UpdateError::ChecksumUnavailable(
        "官方 .sha256 资产与 GitHub API assets[].digest 均不可得，按 fail-closed 拒绝".to_string(),
    ))
}

/// 比对期望与实际 SHA-256：相等 → Ok；不等 → `Err(IntegrityError)`（不静默，调用方删除已下文件）。
pub(crate) fn verify_sha256_matches(expected: &str, actual: &str) -> Result<(), UpdateError> {
    if expected.eq_ignore_ascii_case(actual) {
        Ok(())
    } else {
        Err(UpdateError::IntegrityError(format!(
            "SHA-256 不匹配（期望 {}，实际 {}），疑似损坏或镜像投毒",
            expected, actual
        )))
    }
}

/// M-2 修复（v9 复审）：校验 asset.name 防止路径穿越
/// asset.name 来自 GitHub API，若账号被入侵可设置为恶意路径；仅允许字母、数字、点、下划线、连字符，拒绝路径分隔符和特殊字符；`pub(crate)`：github 子模块的 `download_update` 调用。
pub fn validate_asset_name(name: &str) -> Result<(), UpdateError> {
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

// =====================================================
// 官方强校验决策树单测（校验值只信官方 + fail-closed）
// 集成测试（backend/tests）只能访问 pub 项，此处覆盖 pub(crate) 纯函数决策树。
// =====================================================
#[cfg(test)]
mod integrity_tests {
    use super::*;

    fn hex64(c: char) -> String {
        std::iter::repeat_n(c, 64).collect()
    }

    /// 官方 .sha256 资产命中 → pass（优先级最高）
    #[test]
    fn resolve_prefers_official_sha256_asset() {
        let content = format!("{}  release-1.0.0.tar.gz", hex64('a'));
        let got = resolve_expected_sha256(Some(content.as_str()), Some("sha256:deadbeef"))
            .expect("官方 .sha256 命中应返回 Ok");
        assert_eq!(got, hex64('a'));
    }

    /// 官方 sha256 缺失 → API assets[].digest 兜底 → pass
    #[test]
    fn resolve_falls_back_to_api_digest() {
        let got = resolve_expected_sha256(None, Some(&format!("sha256:{}", hex64('b'))))
            .expect("API digest 兜底应返回 Ok");
        assert_eq!(got, hex64('b'));
    }

    /// 两官方源皆无 → Err ChecksumUnavailable（fail-closed，绝不放行）
    #[test]
    fn resolve_fail_closed_when_no_source() {
        let err = resolve_expected_sha256(None, None).expect_err("两源皆无必须 fail-closed");
        assert!(
            matches!(err, UpdateError::ChecksumUnavailable(_)),
            "必须为 ChecksumUnavailable，实得: {err:?}"
        );
    }

    /// 官方 .sha256 内容为空/非法（无有效 hash）→ 回退 API digest
    #[test]
    fn resolve_skips_empty_official_content() {
        let got = resolve_expected_sha256(Some("   \n"), Some(&format!("sha256:{}", hex64('c'))))
            .expect("官方内容无 hash 时应回退 API digest");
        assert_eq!(got, hex64('c'));
    }

    /// sha256_from_api_digest 仅接受 `sha256:<64hex>`
    #[test]
    fn api_digest_parsing() {
        assert_eq!(
            sha256_from_api_digest(&format!("sha256:{}", hex64('a'))).as_deref(),
            Some(hex64('a').as_str())
        );
        // 大写 hex 归一为小写
        assert_eq!(
            sha256_from_api_digest(&format!("sha256:{}", hex64('A'))).as_deref(),
            Some(hex64('a').as_str())
        );
        assert!(
            sha256_from_api_digest("md5:aaa").is_none(),
            "非 sha256 前缀拒绝"
        );
        assert!(
            sha256_from_api_digest("sha256:tooshort").is_none(),
            "长度非法拒绝"
        );
        // 64 长度但含非 hex 字符 'z' → 必须被 hex 校验拒绝
        let non_hex = format!("sha256:{}", "z".repeat(64));
        assert!(
            sha256_from_api_digest(&non_hex).is_none(),
            "非 hex 字符拒绝"
        );
    }

    /// parse_checksum_line 取 `<hash>  <file>` 首字段并小写
    #[test]
    fn parse_checksum_line_works() {
        assert_eq!(
            parse_checksum_line(&format!("{}  release.tar.gz", hex64('a'))).as_deref(),
            Some(hex64('a').as_str())
        );
        assert_eq!(parse_checksum_line("  \n"), None, "空白行应返回 None");
    }

    /// verify_sha256_matches：相等（大小写不敏感）pass；不等 Err IntegrityError
    #[test]
    fn verify_sha256_matches_decision() {
        assert!(
            verify_sha256_matches(&hex64('a'), &hex64('A')).is_ok(),
            "大小写不敏感应相等"
        );
        let err = verify_sha256_matches(&hex64('a'), &hex64('b')).expect_err("不匹配必须拒绝");
        assert!(
            matches!(err, UpdateError::IntegrityError(_)),
            "不匹配必须 IntegrityError，实得: {err:?}"
        );
    }

    /// is_official_digest_host：官方资产域 true，镜像域名 false（校验值只信官方）
    #[test]
    fn is_official_digest_host_narrows_to_official() {
        assert!(is_official_digest_host("github.com"));
        assert!(is_official_digest_host("objects.githubusercontent.com"));
        // 校验值文件（.sha256 资产）恒从 github.com / objects.githubusercontent.com 下载；
        // API assets[].digest 走 JSON 字段不经此 host 判定，故 host 白名单只收资产下载官方域。
        assert!(!is_official_digest_host("ghproxy.net"), "镜像 host 必须拒");
        assert!(
            !is_official_digest_host("mirror.example.com"),
            "任意镜像 host 必须拒"
        );
    }

    /// 不变量：内置默认镜像 host **绝不允许取校验值**（信任锚永远收窄到官方域）。
    /// 默认镜像允许作字节下载候选由 github.rs 候选生成 + 运行期逐候选尝试保证（失败优雅跳官方）；
    /// 此处用纯函数（不依赖进程级 `global_update_config` 单例，避免跨测试污染）钉死"绝不入校验锚"。
    #[test]
    fn default_mirrors_are_never_official_digest_hosts() {
        for raw in crate::config::settings::DEFAULT_RELEASE_MIRRORS {
            let host = url::Url::parse(raw)
                .expect("默认镜像须为合法 URL")
                .host_str()
                .expect("默认镜像须含 host")
                .to_string();
            assert!(
                !is_official_digest_host(&host),
                "默认镜像 host {host} 绝不能进入校验值信任锚（防连带投毒）"
            );
            let digest_url = format!("{}/release-1.tar.gz.sha256", raw.trim_end_matches('/'));
            let err =
                validate_official_digest_url(&digest_url).expect_err("默认镜像取校验值必须被拒");
            assert!(
                matches!(err, UpdateError::IntegrityError(_)),
                "默认镜像校验值 URL 应为 IntegrityError，实得: {err:?}"
            );
        }
    }

    /// validate_official_digest_url：官方 https pass；镜像 host 被 IntegrityError 拒；http 被 NetworkError 拒
    #[test]
    fn validate_official_digest_url_rejects_mirror() {
        assert!(
            validate_official_digest_url("https://github.com/o/r/releases/download/v1/x.sha256")
                .is_ok()
        );
        let err = validate_official_digest_url("https://ghproxy.net/x.sha256")
            .expect_err("镜像取校验值必须被拒");
        assert!(
            matches!(err, UpdateError::IntegrityError(_)),
            "镜像 host 应为 IntegrityError，实得: {err:?}"
        );
        let insecure = validate_official_digest_url("http://github.com/x.sha256")
            .expect_err("非 https 必须被拒");
        assert!(
            matches!(insecure, UpdateError::NetworkError(_)),
            "非 https 应为 NetworkError，实得: {insecure:?}"
        );
    }
}
