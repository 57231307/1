//! 任务 #121 系统更新镜像 / SHA-256 完整性 fail-closed —— 集成测试（公有面）
//!
//! 本文件位于 `backend/tests/`（独立 crate `bingxi_backend` 之外），**只能访问 `pub` 项**。
//! 因此本文件覆盖 #121 信任模型中可从 crate 外部真实观测的公有契约：
//! - 校验值取源 host/URL 收窄判定（`is_official_digest_host` / `validate_official_digest_url`）；
//! - 字节下载 URL 判定（`validate_download_url`，默认无镜像态）；
//! - `UpdateError::IntegrityError` / `ChecksumUnavailable` → `AppError` 映射语义；
//! - `GitHubAsset.digest` 的 serde 反序列化契约（fail-closed 前提）；
//! - `UpdateConfig` / `MirrorOrder` 默认值（安全兜底）。
//!
//! # 与 `sha256_from_api_digest` / `resolve_expected_sha256` / `verify_sha256_matches` 的分工
//! 这三个纯函数为 `pub(crate)`（源码 `system_update_service.rs`），**集成测试 crate 无法访问**。
//! 它们已在源码内 `#[cfg(test)] mod integrity_tests` 被单元测试覆盖（决策树、剥前缀、大小写归一、
//! fail-closed、IntegrityError 语义）。本文件不重复、也不为此改源码可见性（红线：不改 backend/src）。
//! 本文件通过 `GitHubAsset.digest` 反序列化真实锁定"进入 resolve 的输入契约"这一可公有观测的部分。
//!
//! # 关于 `api.github.com` 的重要事实校正
//! 任务描述假设"官方域 github.com / objects.githubusercontent.com / api.github.com 均放行"。
//! 但源码 `OFFICIAL_DOWNLOAD_HOSTS = ["github.com", "objects.githubusercontent.com"]` 仅收
//! **资产下载官方域**（源码内注释：API `assets[].digest` 走 JSON 字段，不经此 host 判定）。
//! 因此 `is_official_digest_host("api.github.com")` 的真实返回是 `false`，本测试**按真实行为断言**，
//! 若按任务假设写成 `true` 将造成反向假失败。

use bingxi_backend::config::settings::{UpdateConfig, global_update_config};
use bingxi_backend::services::system_update_service::{
    GitHubAsset, UpdateError, is_official_digest_host, validate_asset_name, validate_download_url,
    validate_official_digest_url,
};
use bingxi_backend::utils::error::AppError;

// =====================================================
// A. is_official_digest_host（pub）：校验值取源 host 收窄到官方资产域
// =====================================================

/// 官方资产下载域放行，且 host 判定大小写不敏感（镜像/域名大小写绕过失效）。
#[test]
fn official_digest_host_accepts_official_asset_hosts_case_insensitive() {
    assert!(
        is_official_digest_host("github.com"),
        "github.com 必须为官方域"
    );
    assert!(
        is_official_digest_host("objects.githubusercontent.com"),
        "objects.githubusercontent.com 必须为官方域"
    );
    // eq_ignore_ascii_case：大小写变体同样放行（防止 host 大小写不一致绕过判定）
    assert!(
        is_official_digest_host("GitHub.com"),
        "host 判定应大小写不敏感"
    );
    assert!(
        is_official_digest_host("OBJECTS.GITHUBUSERCONTENT.COM"),
        "host 判定应大小写不敏感"
    );
}

/// api.github.com / 镜像 / 裸 IP / 内网元数据 IP / 前后缀伪装域一律拒（真实行为锁定）。
#[test]
fn official_digest_host_rejects_api_mirror_ip_and_lookalikes() {
    // 事实校正：源码白名单只收资产下载域，api.github.com 不在内 → 真实为 false。
    assert!(
        !is_official_digest_host("api.github.com"),
        "api.github.com 不属资产下载 host 白名单（API digest 走 JSON 字段不经此判定），真实应为 false"
    );
    // 镜像 host 绝不放行（#121 红线：严禁从镜像取校验基准，防连带投毒）。
    assert!(!is_official_digest_host("ghproxy.net"), "镜像 host 必须拒");
    assert!(
        !is_official_digest_host("mirror.example.com"),
        "任意镜像 host 必须拒"
    );
    // 裸 IP / 链路本地元数据 IP（SSRF 目标）必须拒。
    assert!(
        !is_official_digest_host("169.254.169.254"),
        "云元数据 IP 必须拒"
    );
    assert!(!is_official_digest_host("127.0.0.1"), "回环 IP 必须拒");
    // 后缀/前缀伪装域：攻击者常注册 github.com.evil.com / evilgithub.com 混过子串匹配。
    assert!(
        !is_official_digest_host("github.com.evil.com"),
        "后缀伪装域必须拒"
    );
    assert!(
        !is_official_digest_host("evilgithub.com"),
        "前缀伪装域必须拒"
    );
    // 空 host / 空串必须拒。
    assert!(!is_official_digest_host(""), "空 host 必须拒");
}

// =====================================================
// B. validate_official_digest_url（pub）：校验值取源 URL（https + 官方域收窄）
// =====================================================

/// 官方 https 校验值 URL 放行。
#[test]
fn validate_official_digest_url_accepts_official_https() {
    assert!(
        validate_official_digest_url("https://github.com/o/r/releases/download/v1/x.sha256")
            .is_ok(),
        "官方 github.com https 应放行"
    );
    assert!(
        validate_official_digest_url("https://objects.githubusercontent.com/x.sha256").is_ok(),
        "官方 objects.githubusercontent.com https 应放行"
    );
}

/// 镜像 https 取校验值必须被 IntegrityError 拒（信任模型核心：镜像只搬字节不供基准）。
#[test]
fn validate_official_digest_url_rejects_mirror_with_integrity_error() {
    let err = validate_official_digest_url("https://ghproxy.net/x.sha256")
        .expect_err("镜像取校验值必须被拒");
    assert!(
        matches!(err, UpdateError::IntegrityError(_)),
        "镜像 host 对校验值 URL 应为 IntegrityError，实得: {err:?}"
    );
}

/// 非 https（明文降级）→ NetworkError；非法/空 URL → NetworkError（parse 失败先于 host 判定）。
#[test]
fn validate_official_digest_url_rejects_non_https_and_malformed() {
    let insecure =
        validate_official_digest_url("http://github.com/x.sha256").expect_err("非 https 必须被拒");
    assert!(
        matches!(insecure, UpdateError::NetworkError(_)),
        "非 https 应为 NetworkError，实得: {insecure:?}"
    );
    let malformed = validate_official_digest_url("not-a-url").expect_err("非法 URL 必须被拒");
    assert!(
        matches!(malformed, UpdateError::NetworkError(_)),
        "非法 URL 应为 NetworkError，实得: {malformed:?}"
    );
    let empty = validate_official_digest_url("").expect_err("空串必须被拒");
    assert!(
        matches!(empty, UpdateError::NetworkError(_)),
        "空串应为 NetworkError，实得: {empty:?}"
    );
}

/// 官方域之外的 https host（含 api.github.com、内网/元数据 IP）→ IntegrityError（host 收窄拒）。
#[test]
fn validate_official_digest_url_rejects_non_official_https_hosts() {
    // https 但 host 非官方资产域 → IntegrityError（区别于 scheme 不符的 NetworkError）。
    let api = validate_official_digest_url("https://api.github.com/x.sha256")
        .expect_err("api.github.com 非资产下载官方域，应拒");
    assert!(
        matches!(api, UpdateError::IntegrityError(_)),
        "非官方 https host 应为 IntegrityError，实得: {api:?}"
    );
    let link_local = validate_official_digest_url("https://169.254.169.254/x.sha256")
        .expect_err("链路本地元数据 IP 应拒");
    assert!(
        matches!(link_local, UpdateError::IntegrityError(_)),
        "内网 IP host 应为 IntegrityError，实得: {link_local:?}"
    );
}

// =====================================================
// C. validate_download_url（pub）：字节下载 URL（https + 官方 ∪ 配置镜像）
//    默认态：本测试进程未加载配置（OnceLock 未 set），global_update_config() 回退
//    UpdateConfig::default()（mirrors 为空）→ 仅官方域放行。
// =====================================================

/// 默认（无镜像配置）态：官方 https 下载 URL 放行。
#[test]
fn validate_download_url_official_https_ok_in_default_config() {
    assert!(
        validate_download_url("https://github.com/57231307/1/releases/download/v1.0.0/pkg.tar.gz")
            .is_ok(),
        "官方 github.com https 下载应放行（默认配置）"
    );
    assert!(
        validate_download_url("https://objects.githubusercontent.com/assets/123").is_ok(),
        "官方 objects.githubusercontent.com https 下载应放行（默认配置）"
    );
}

/// 默认态：http 明文 / 非官方 host / 内网元数据 IP / 未配置镜像 host 均被 NetworkError 拒。
#[test]
fn validate_download_url_rejects_insecure_and_disallowed_hosts() {
    let http = validate_download_url("http://github.com/repo/releases")
        .expect_err("http 明文下载必须被拒");
    assert!(
        matches!(http, UpdateError::NetworkError(_)),
        "非 https 应为 NetworkError，实得: {http:?}"
    );

    let evil = validate_download_url("https://evil.com/exploit").expect_err("非允许域必须被拒");
    assert!(
        matches!(evil, UpdateError::NetworkError(_)),
        "非法 host 应为 NetworkError，实得: {evil:?}"
    );

    let metadata = validate_download_url("https://169.254.169.254/metadata")
        .expect_err("链路本地元数据 IP 必须被拒（SSRF 面）");
    assert!(
        matches!(metadata, UpdateError::NetworkError(_)),
        "内网 IP 应为 NetworkError，实得: {metadata:?}"
    );

    // 默认配置 mirrors 为空 → 镜像 host 不在允许列表，被拒（#121 信任模型：镜像须显式配置且经
    // validate_update_mirrors 的 https/非 IP/SSRF 启动校验后方可入列）。
    let mirror = validate_download_url("https://ghproxy.net/https://github.com/x.tar.gz")
        .expect_err("未配置镜像 host 必须被拒");
    assert!(
        matches!(mirror, UpdateError::NetworkError(_)),
        "未配置镜像 host 应为 NetworkError，实得: {mirror:?}"
    );
}

// =====================================================
// D. validate_asset_name（pub）：asset.name 路径穿越防护（下载落盘文件名，#121 链路一环）
// =====================================================

/// asset.name 合法值放行；路径穿越 / 绝对路径 / 隐藏文件被拒。
#[test]
fn validate_asset_name_guards_path_traversal() {
    assert!(
        validate_asset_name("release-2026.9.29.tar.gz").is_ok(),
        "合法 tar 包名应放行"
    );
    assert!(
        validate_asset_name("release-2026.9.29.tar.gz.sha256").is_ok(),
        "官方 .sha256 资产名应放行"
    );
    assert!(
        validate_asset_name("../../etc/cron.d/evil").is_err(),
        "路径穿越必须拒"
    );
    assert!(
        validate_asset_name("/etc/passwd").is_err(),
        "绝对路径必须拒"
    );
    assert!(validate_asset_name(".hidden").is_err(), "隐藏文件必须拒");
    assert!(validate_asset_name("").is_err(), "空 asset.name 必须拒");
}

// =====================================================
// E. UpdateError -> AppError 映射（pub From impl）：#121 新增两分支映射为 internal
// =====================================================

/// IntegrityError（SHA-256 不匹配，投毒/损坏）→ AppError::InternalError（护栏判定，不降级为业务/校验类）。
#[test]
fn integrity_error_maps_to_internal_app_error() {
    let app: AppError = UpdateError::IntegrityError("SHA-256 不匹配".to_string()).into();
    assert!(
        matches!(app, AppError::InternalError(_)),
        "IntegrityError 必须映射为 InternalError，实得: {app:?}"
    );
}

/// ChecksumUnavailable（两官方源皆无，fail-closed）→ AppError::InternalError（拒绝而非放行）。
#[test]
fn checksum_unavailable_maps_to_internal_app_error() {
    let app: AppError = UpdateError::ChecksumUnavailable("校验值不可用".to_string()).into();
    assert!(
        matches!(app, AppError::InternalError(_)),
        "ChecksumUnavailable 必须映射为 InternalError，实得: {app:?}"
    );
}

// =====================================================
// F. GitHubAsset.digest serde 契约（pub 字段）：#121 fail-closed 的输入前提
// =====================================================

/// release JSON 缺 digest 字段 → None（老 release 兼容；resolve 据此走 fail-closed）。
#[test]
fn github_asset_digest_missing_deserializes_to_none() {
    let json = serde_json::json!({
        "name": "release-2026.9.29.tar.gz",
        "browser_download_url": "https://github.com/o/r/releases/download/v1/release-2026.9.29.tar.gz",
        "size": 1234u64,
        "content_type": "application/gzip"
    });
    let asset: GitHubAsset =
        serde_json::from_value(json).expect("无 digest 字段的 release asset 必须可反序列化");
    assert_eq!(
        asset.digest, None,
        "缺 digest 字段应反序列化为 None（#[serde(default)] 兜底）"
    );
}

/// digest 原样保留为 Some(串)，serde 层**不做剥前缀/校验**（那是 pub(crate) sha256_from_api_digest 的职责）。
#[test]
fn github_asset_digest_is_preserved_verbatim_by_serde() {
    let raw = format!("sha256:{}", "a".repeat(64));
    let json = serde_json::json!({
        "name": "release.tar.gz",
        "browser_download_url": "https://github.com/o/r/releases/download/v1/release.tar.gz",
        "size": 1u64,
        "content_type": "application/gzip",
        "digest": raw
    });
    let asset: GitHubAsset = serde_json::from_value(json).expect("含 digest 的 asset 应可反序列化");
    assert_eq!(
        asset.digest.as_deref(),
        Some(raw.as_str()),
        "serde 只负责原样承载 API digest，剥前缀/合法性由 sha256_from_api_digest 判定"
    );
}

// =====================================================
// G. UpdateConfig / MirrorOrder / global_update_config 默认（安全兜底，pub）
// =====================================================

/// 默认配置即安全兜底：强制校验开、无镜像、官方优先（未加载配置时 global_update_config 回退此默认）。
#[test]
fn update_config_defaults_are_fail_closed() {
    let def = UpdateConfig::default();
    assert!(
        def.verify_digest,
        "默认必须强制 SHA-256 校验（fail-closed）"
    );
    assert!(def.mirrors.is_empty(), "默认不放行任何镜像");
    assert_eq!(
        def.mirror_order,
        bingxi_backend::config::settings::MirrorOrder::OfficialFirst,
        "默认官方优先，最大限度降低投毒面"
    );

    // 集成测试进程未调用 AppSettings::new()（未 set 全局单例）→ global_update_config 回退默认。
    let g = global_update_config();
    assert!(
        g.verify_digest && g.mirrors.is_empty(),
        "未初始化配置时 global_update_config 必须回退安全默认（仅官方域 + 强校验），实得: {g:?}"
    );
}
