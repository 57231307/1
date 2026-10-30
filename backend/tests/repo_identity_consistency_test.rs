//! 更新通道仓身份一致性与缺省值契约。
//!
//! 功能：钉住自更新使用的 `owner/name`，确认缺省值就是本仓库真实身份、确认旧值不会回潮、
//! 确认生效值与运行时可见的远端地址同源，并双向证明覆盖解析既收合法值也点名拒绝非法值。
//!
//! 调用方：`cargo test` 集成测试档，CI 与本地同一条命令触发，不依赖工作流定义。
//!
//! 入参：环境变量 `GITHUB_REPOSITORY`（CI 检出时由 runner 注入，值形如 `owner/name`）优先，
//! 缺失时回落到 `git config --get remote.origin.url`（本地检出的 origin 地址，https 与 ssh
//! 两种形态都支持）；被测面另收 `UPDATE__GITHUB_REPO` 覆盖值的解析结果。
//!
//! 传给谁：断言直接打在 `bingxi_backend::services::system_update_service` 的公有面
//! `GITHUB_REPO_OWNER` / `GITHUB_REPO_NAME` / `resolve_github_repo` 上，不经任何中间层。
//!
//! 存什么/存哪里：本文件是只读判定，不落任何业务数据，也不改仓库远端配置。
//!
//! 判定边界：两个参照源都取不到时判失败而非跳过，因为跳过会让这条锁在缺 git 的构建环境里
//! 变成恒真；覆盖值为空串也判失败，空串属"设了但没填"，必须点名而不是当作未设。

use bingxi_backend::services::system_update_service::{
    GITHUB_REPO_NAME, GITHUB_REPO_OWNER, resolve_github_repo,
};

const EXPECTED_DEFAULT: &str = "BingXi-ERP/bingxi-erp";
/// 改名前的旧 owner 与已废弃的旧仓名组合，出现即回归。
const STALE_IDENTITY: &str = "57231307/1";

/// 把 https 与 ssh 两种 origin 地址统一归一成小写 `owner/name`。
/// 调用方：本文件的一致性断言。入参：`git config --get remote.origin.url` 的原始值。
/// 传给谁：只在本文件内比较用，不回写任何配置。存什么/存哪里：不存。
fn normalize_remote_identity(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_end_matches('/').trim_end_matches(".git");
    let tail = trimmed.rsplit(['/', ':']).next().unwrap_or(trimmed);
    // owner 与 name 是尾部两段，ssh 形态 `git@host:owner/name.git` 与 https 形态同构
    let head_len = trimmed.len() - tail.len();
    let owner = trimmed[..head_len]
        .rsplit(['/', ':'])
        .next()
        .filter(|s| !s.is_empty() && !s.contains('@'))?;
    if tail.is_empty() {
        return None;
    }
    Some(format!("{owner}/{tail}").to_lowercase())
}

/// 取运行时可见的远端身份参照面。
/// 调用方：本文件的一致性断言。入参：无，读环境变量与 git 配置。
/// 传给谁：返回值只用于比较。存什么/存哪里：不存。
/// 两个参照源都取不到时返回 None，由调用方判失败。
fn runtime_remote_identity() -> Option<String> {
    if let Ok(repo) = std::env::var("GITHUB_REPOSITORY") {
        let normalized = normalize_remote_identity(&repo);
        if normalized.is_some() {
            return normalized;
        }
    }
    let output = std::process::Command::new("git")
        .args(["config", "--get", "remote.origin.url"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    normalize_remote_identity(&String::from_utf8_lossy(&output.stdout))
}

#[test]
fn default_identity_matches_expected_repo() {
    let default = resolve_github_repo(None).expect("缺省仓身份不应被拒绝");
    assert_eq!(
        default, EXPECTED_DEFAULT,
        "自更新缺省仓身份应为 {EXPECTED_DEFAULT}，实际 {default}"
    );
    assert_eq!(
        format!("{GITHUB_REPO_OWNER}/{GITHUB_REPO_NAME}"),
        EXPECTED_DEFAULT,
        "owner 与 name 两段常量拼起来应与生效缺省一致"
    );
}

#[test]
fn stale_identity_never_leaks_back() {
    let default = resolve_github_repo(None).expect("缺省仓身份不应被拒绝");
    assert_ne!(
        default, STALE_IDENTITY,
        "旧身份 {STALE_IDENTITY} 回潮会让更新检查指向已改名前的仓库"
    );
    assert_ne!(
        GITHUB_REPO_NAME, "1",
        "仓名常量为占位数字即说明真源又被改回旧值"
    );
}

#[test]
fn default_identity_is_consistent_with_runtime_remote() {
    let observed = runtime_remote_identity().unwrap_or_else(|| {
        panic!(
            "一致性判定拿不到参照面：环境变量 GITHUB_REPOSITORY 未设，\
             且 git config --get remote.origin.url 也无可用值。此档不做静默跳过。"
        )
    });
    let default = resolve_github_repo(None)
        .expect("缺省仓身份不应被拒绝")
        .to_lowercase();
    assert_eq!(
        default, observed,
        "代码里的缺省仓身份与检出的远端地址不同源，代码 {default} 远端 {observed}"
    );
}

#[test]
fn valid_override_is_honored() {
    for candidate in [
        "acme-corp/bingxi-erp",
        "Acme2/bingxi_erp",
        "acme/bingxi-erp.v2",
        EXPECTED_DEFAULT,
    ] {
        let got = resolve_github_repo(Some(candidate))
            .unwrap_or_else(|e| panic!("合法覆盖 {candidate} 应被接受，实际被拒：{e}"));
        assert_eq!(got, candidate);
    }
}

#[test]
fn malformed_override_is_rejected_by_name() {
    for candidate in [
        "",
        "   ",
        "no-slash-here",
        "/only-name",
        "owner/",
        "a/b/c",
        "two words/name",
        "中文/name",
        "-lead/name",
        "trail-/name",
        "double--hy/name",
        "owner/.name",
        "owner/name.",
        "owner/name with space",
    ] {
        let outcome = resolve_github_repo(Some(candidate));
        assert!(
            outcome.is_err(),
            "非法覆盖 {candidate:?} 必须被点名拒绝，实际返回 {outcome:?}"
        );
        let reason = outcome.unwrap_err();
        assert!(
            reason.contains("UPDATE__GITHUB_REPO"),
            "拒绝理由应含被拒的环境变量名，便于运维定位，实际 `{reason}`"
        );
    }
}

#[test]
fn blank_override_is_not_treated_as_unset() {
    // 空串属"设了但没填"，与 None 必须走不同分支，否则运维误清空后会让更新悄悄指回缺省仓。
    assert!(
        resolve_github_repo(Some("")).is_err(),
        "空覆盖值应报错而不是回落缺省"
    );
    assert_eq!(
        resolve_github_repo(None).expect("未设覆盖时缺省应可用"),
        EXPECTED_DEFAULT
    );
}
