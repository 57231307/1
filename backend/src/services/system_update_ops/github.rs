//! GitHub 远程更新子模块（system_update_ops/github）
//!
//! 从原 `system_update_service.rs` 迁移 8 个方法：
//! - `check_for_updates`：查询 GitHub Releases 最新版本（pub，handler 调用）
//! - `fetch_latest_release`：调用 GitHub API `/repos/{owner}/{repo}/releases/latest`（私有）
//! - `compare_versions`：比较 current < latest（`pub(crate)`，供 status 子模块 `check_local_updates`
//!  + facade 测试调用）
//! - `compare_versions_for_sort`：版本号排序比较（`pub(crate)`，供 status 子模块 `list_local_releases` 调用）//! - `download_update`：下载 GitHub Release asset（pub，handler + `download_and_update` 调用）
//! - `find_release_asset`：在 Release 中查找匹配 asset（私有，关联函数）
//! - `build_safe_download_client`：构建 SSRF 防御下载客户端（私有，关联函数）
//! - `download_and_update`：下载并应用更新（pub，handler 调用）
//!
//! 跨模块依赖：
//! - `check_for_updates` 调用 `status::get_current_version`（pub）
//! - `compare_versions` / `compare_versions_for_sort` 调用 facade 归一函数 `normalize_versions_for_compare`（`pub(crate)`，内部含 `parse_version` 与三段↔四段 MD 反解兜底）
//! - `download_update` 调用 `apply::log_update`（`pub(crate)`）+ facade 纯函数
//!  `validate_asset_name` / `validate_download_url`（`pub(crate)`）
//! - `download_and_update` 调用 `apply::apply_update`（pub）+ `apply::log_update`（`pub(crate)`）
//! - `fetch_latest_release` 使用 facade 常量 `GITHUB_API_URL` / `GITHUB_REPO`（`pub(crate)`）

use crate::config::settings::{
    DEFAULT_RELEASE_MIRRORS, MirrorOrder, UpdateConfig, global_update_config,
};
use crate::services::system_update_service::{GITHUB_API_URL, GITHUB_REPO};
use crate::services::system_update_service::{
    GitHubAsset, GitHubRelease, SystemUpdateService, UpdateCheckResult, UpdateError,
    find_official_md5_asset, find_official_sha256_asset, normalize_versions_for_compare,
    resolve_expected_sha256, sha256_hex_of_file, validate_asset_name, validate_download_url,
    validate_official_digest_url, verify_sha256_matches,
};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

impl SystemUpdateService {
    pub async fn check_for_updates(&self) -> UpdateCheckResult {
        let current_version = self.get_current_version();

        match self.fetch_latest_release().await {
            Ok(release) => {
                let latest_version = release.tag_name.trim_start_matches('v').to_string();
                let has_update = self.compare_versions(&current_version, &latest_version);

                // 按当前版本 tag 查询对应 Release（查不到返回 None，不冒充）
                let current_release_info = match self.fetch_release_by_tag(&current_version).await {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::warn!(
                            "[system_update] fetch_release_by_tag(current={}) 失败: {}",
                            current_version,
                            e
                        );
                        None
                    }
                };

                UpdateCheckResult {
                    has_update,
                    current_version,
                    latest_version,
                    release_info: Some(release),
                    current_release_info,
                    error: None,
                }
            }
            Err(e) => UpdateCheckResult {
                has_update: false,
                current_version: current_version.clone(),
                latest_version: current_version,
                release_info: None,
                current_release_info: None,
                error: Some(e.to_string()),
            },
        }
    }

    async fn fetch_latest_release(&self) -> Result<GitHubRelease, UpdateError> {
        let url = format!("{}/repos/{}/releases/latest", GITHUB_API_URL, GITHUB_REPO);

        // M-1 修复（v9 复审）：对齐 download_update 的 SSRF 防护
        // 原 L1 修复仅添加重定向限制，未用 resolve_to_addrs 防 DNS Rebinding
        // 攻击者可在 DNS 解析后修改记录指向内网 IP（DNS Rebinding TOCTOU）
        let (api_host, api_safe_addrs) = crate::utils::ssrf_guard::validate_url_and_resolve(&url)
            .map_err(|e| {
            UpdateError::NetworkError(format!("GitHub API URL SSRF 校验失败: {}", e))
        })?;

        let client = reqwest::Client::builder()
            .user_agent("BingxiManagementPlatform/1.0")
            .redirect(reqwest::redirect::Policy::limited(3))
            .resolve_to_addrs(&api_host, &api_safe_addrs)
            .build()
            .map_err(|e| UpdateError::NetworkError(e.to_string()))?;

        // V15 P1 20.1-A：注入 traceparent 到出站 HTTP 请求，跨服务调用链追踪
        let traceparent = crate::observability::trace_context::traceparent_from_current_span();
        let response = client
            .get(&url)
            .header(
                crate::observability::trace_context::TRACEPARENT_HEADER,
                traceparent,
            )
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

    /// 按 tag 精确查询 GitHub Release（复用与 `fetch_latest_release` 同款 SSRF 防护）。
    ///
    /// - 成功（HTTP 200）：返回 `Ok(Some(release))`
    /// - 未找到（HTTP 404，历史版本无 tag）：返回 `Ok(None)`（诚实空态，严禁拿新版本冒充）
    /// - 其他网络/解析错误：返回 `Err`
    pub(crate) async fn fetch_release_by_tag(
        &self,
        version: &str,
    ) -> Result<Option<GitHubRelease>, UpdateError> {
        let url = format!(
            "{}/repos/{}/releases/tags/v{}",
            GITHUB_API_URL, GITHUB_REPO, version
        );

        let (api_host, api_safe_addrs) = crate::utils::ssrf_guard::validate_url_and_resolve(&url)
            .map_err(|e| {
            UpdateError::NetworkError(format!("GitHub API tag-release URL SSRF 校验失败: {}", e))
        })?;

        let client = reqwest::Client::builder()
            .user_agent("BingxiManagementPlatform/1.0")
            .redirect(reqwest::redirect::Policy::limited(3))
            .resolve_to_addrs(&api_host, &api_safe_addrs)
            .build()
            .map_err(|e| UpdateError::NetworkError(e.to_string()))?;

        let traceparent = crate::observability::trace_context::traceparent_from_current_span();
        let response = client
            .get(&url)
            .header(
                crate::observability::trace_context::TRACEPARENT_HEADER,
                traceparent,
            )
            .send()
            .await
            .map_err(|e| UpdateError::NetworkError(e.to_string()))?;

        // 404 表示该 tag 不存在对应 release（历史/开发构建） → 诚实返回 None
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            tracing::info!(
                "[system_update] fetch_release_by_tag: tag v{} 无对应 GitHub Release",
                version
            );
            return Ok(None);
        }

        if !response.status().is_success() {
            return Err(UpdateError::NetworkError(format!(
                "GitHub API tag-release 返回错误状态: {}",
                response.status()
            )));
        }

        let release: GitHubRelease = response
            .json()
            .await
            .map_err(|e| UpdateError::NetworkError(e.to_string()))?;

        Ok(Some(release))
    }

    pub fn compare_versions(&self, current: &str, latest: &str) -> bool {
        // 任务 #116：改走共享归一路径，消除三段 current vs 四段 tag 的跨格式假阴性
        // （同段数原样比 / 3↔4 才 MD 反解 / 不可判定退回 element-wise 并 warn）；
        // 与 compare_versions_for_sort 同源，避免逻辑重复。
        let (current_parts, latest_parts) = normalize_versions_for_compare(current, latest);

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

    pub fn compare_versions_for_sort(&self, a: &str, b: &str) -> std::cmp::Ordering {
        // 任务 #116：与 compare_versions 走同一归一路径（normalize_versions_for_compare），
        // 保证排序与"是否最新"判定对三段/四段混排的口径一致。
        let (a_parts, b_parts) = normalize_versions_for_compare(a, b);

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

        let release = check_result
            .release_info
            .ok_or_else(|| UpdateError::NetworkError("无法获取发布信息".to_string()))?;

        let asset = Self::find_release_asset(&release, asset_name)?;

        self.log_update(&format!("开始下载更新包: {}", asset.name));

        // M-2 修复（v9 复审）：校验 asset.name 防止路径穿越
        validate_asset_name(&asset.name)?;

        let download_dir = self.app_dir.join("downloads");
        if !download_dir.exists() {
            fs::create_dir_all(&download_dir)?;
        }
        let download_path = download_dir.join(&asset.name);

        // 任务 #121：读取进程级更新配置，按 mirror_order 生成候选下载 URL（官方永远列入并兜底）。
        let cfg = global_update_config();
        let candidates = Self::build_download_candidates(&asset.browser_download_url, &cfg);
        tracing::info!(
            "[system_update] 更新包 '{}' 生成 {} 个下载候选（mirror_order={:?}，官方域永远列入并兜底）",
            asset.name,
            candidates.len(),
            cfg.mirror_order
        );

        // 逐个候选尝试：任一成功即停止；全部失败返回最后错误（每步 warn，不静默）。
        let mut last_err: Option<UpdateError> = None;
        let mut downloaded = false;
        for (idx, cand_url) in candidates.iter().enumerate() {
            match self.try_download_candidate(cand_url, &download_path).await {
                Ok(()) => {
                    tracing::info!(
                        "[system_update] 下载候选[{}/{}] 成功: {}",
                        idx + 1,
                        candidates.len(),
                        cand_url
                    );
                    downloaded = true;
                    break;
                }
                Err(e) => {
                    tracing::warn!(
                        "[system_update] 下载候选[{}/{}] 失败（{}）: {}",
                        idx + 1,
                        candidates.len(),
                        cand_url,
                        e
                    );
                    // 清理该候选写入的半成品，避免残损文件影响后续候选与校验
                    if let Err(rm) = fs::remove_file(&download_path)
                        && rm.kind() != std::io::ErrorKind::NotFound
                    {
                        tracing::warn!("[system_update] 清理失败候选残件出错: {}", rm);
                    }
                    last_err = Some(e);
                }
            }
        }

        if !downloaded {
            let msg = "所有下载候选（含官方域）均失败";
            self.log_update(msg);
            return Err(last_err.unwrap_or_else(|| UpdateError::NetworkError(msg.to_string())));
        }

        // 任务 #121：字节到手后取【官方】校验值 + 本地 SHA-256 重算比对（fail-closed）。
        // 校验值只从官方域取（CI .sha256 资产优先，其次 API assets[].digest）；镜像仅搬 tar 字节。
        if let Err(e) = self
            .verify_downloaded_integrity(asset, &release.assets, &download_path, &cfg)
            .await
        {
            // 校验不通过 / 不可得 → 删除已下文件 + 返回错误（绝不 apply）
            if let Err(rm) = fs::remove_file(&download_path)
                && rm.kind() != std::io::ErrorKind::NotFound
            {
                tracing::warn!("[system_update] 校验失败后清理下载文件出错: {}", rm);
            }
            self.log_update(&format!("完整性校验未通过，已删除下载文件: {}", e));
            return Err(e);
        }

        self.log_update(&format!("更新包下载并通过完整性校验: {:?}", download_path));
        Ok(download_path)
    }

    /// 任务 #121：按 `mirror_order` 生成候选下载 URL 列表。
    /// - 官方 `browser_download_url` 永远列入且作最终兜底；
    /// - 镜像改写形如 `{mirror}/{githubAssetUrl}`（**仅搬大 tar 字节，绝不用于取校验值**）；
    /// - 镜像 base 集合 = 运维显式 `cfg.mirrors` ∪（`cfg.use_default_mirrors==true` 时并入的
    ///   内置默认 [`DEFAULT_RELEASE_MIRRORS`]），按出现顺序去重（尾斜杠归一）；
    /// - `MirrorFirst`：镜像优先、官方兜底末位；`OfficialFirst`（默认）：官方优先、镜像兜底。
    fn build_download_candidates(official_url: &str, cfg: &UpdateConfig) -> Vec<String> {
        // 镜像 base 归一：trim + 去尾斜杠 + 去空 + 精确去重（运维清单在前，内置默认在后）。
        // 内置默认镜像只在下载期作为候选按序尝试、失败优雅跳到下一候选/官方，绝不参与启动
        // fail-fast（`validate_update_mirrors` 只校验运维 `cfg.mirrors`）。
        let mut mirror_bases: Vec<String> = Vec::new();
        {
            let push_unique = |bases: &mut Vec<String>, raw: &str| {
                let base = raw.trim().trim_end_matches('/');
                if base.is_empty() {
                    return;
                }
                if !bases.iter().any(|b| b == base) {
                    bases.push(base.to_string());
                }
            };
            for raw in &cfg.mirrors {
                push_unique(&mut mirror_bases, raw);
            }
            if cfg.use_default_mirrors {
                for raw in DEFAULT_RELEASE_MIRRORS {
                    push_unique(&mut mirror_bases, raw);
                }
            }
        }

        let mirror_urls: Vec<String> = mirror_bases
            .iter()
            .map(|m| format!("{}/{}", m, official_url))
            .collect();

        let mut candidates = Vec::new();
        match cfg.mirror_order {
            MirrorOrder::MirrorFirst => {
                candidates.extend(mirror_urls);
                candidates.push(official_url.to_string());
            }
            MirrorOrder::OfficialFirst => {
                candidates.push(official_url.to_string());
                candidates.extend(mirror_urls);
            }
        }
        candidates
    }

    /// 对单个候选 URL 流式下载到 `download_path`（host 过下载白名单 + SSRF + 重定向终点复核）。
    async fn try_download_candidate(
        &self,
        url: &str,
        download_path: &Path,
    ) -> Result<(), UpdateError> {
        let client = Self::build_safe_download_client(url)?;

        let traceparent = crate::observability::trace_context::traceparent_from_current_span();
        let mut response = client
            .get(url)
            .header(
                crate::observability::trace_context::TRACEPARENT_HEADER,
                traceparent,
            )
            .send()
            .await
            .map_err(|e| UpdateError::NetworkError(e.to_string()))?;

        // TS-S-7：二次校验最终跳转后的 URL 域名（镜像若 302 到别处，终点仍须在允许域内）
        let final_url = response.url().clone();
        validate_download_url(final_url.as_str())?;

        let mut file = fs::File::create(download_path)?;
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| UpdateError::NetworkError(e.to_string()))?
        {
            io::copy(&mut chunk.as_ref(), &mut file)?;
        }
        Ok(())
    }

    /// 任务 #121：取【官方】SHA-256 校验基准 + 本地重算比对。
    /// 优先级：① CI 上传的官方 `.sha256` 资产内容 ② GitHub API `assets[].digest` 兜底。
    /// - 两源皆无 → `Err(ChecksumUnavailable)`（`verify_digest=true` 时 fail-closed 拒绝 apply）；
    /// - 取到但不匹配 → `Err(IntegrityError)`（无论 verify_digest 均拒绝，投毒零容忍）。
    /// - MD5 资产仅作补充记录（不抗碰撞、非门槛；仓内无 md5 crate，不本地重算）。
    async fn verify_downloaded_integrity(
        &self,
        asset: &GitHubAsset,
        assets: &[GitHubAsset],
        download_path: &Path,
        cfg: &UpdateConfig,
    ) -> Result<(), UpdateError> {
        // MD5：补充记录（明确注释：不抗碰撞，不作为独立安全门槛）
        if let Some(md5_asset) = find_official_md5_asset(assets, &asset.name) {
            tracing::info!(
                "[system_update] 存在官方 MD5 资产 '{}'（仅补充记录；SHA-256 为权威安全判据，MD5 不抗碰撞不作独立门槛）",
                md5_asset.name
            );
        }

        // ① 官方 .sha256 资产内容（host 须官方域，只走官方直连取回）
        let official_sha256_content = match find_official_sha256_asset(assets, &asset.name) {
            Some(sha_asset) => {
                match Self::fetch_official_digest_content(&sha_asset.browser_download_url).await {
                    Ok(content) => Some(content),
                    Err(e) => {
                        tracing::warn!(
                            "[system_update] 官方 .sha256 资产取回失败，回退 API assets[].digest: {}",
                            e
                        );
                        None
                    }
                }
            }
            None => {
                tracing::info!(
                    "[system_update] release 无官方 .sha256 资产，尝试 API assets[].digest 兜底"
                );
                None
            }
        };

        let expected = match resolve_expected_sha256(
            official_sha256_content.as_deref(),
            asset.digest.as_deref(),
        ) {
            Ok(hex) => hex,
            Err(e) => {
                if cfg.verify_digest {
                    // fail-closed：两官方校验源皆不可得 → 拒绝（严禁因取不到校验值而放行）
                    tracing::error!("[system_update] fail-closed：{}（verify_digest=true）", e);
                    return Err(e);
                }
                // 显式逃生阀（仅隔离调试）：verify_digest=false 且无校验源 → 告警放行，不静默
                tracing::warn!(
                    "[system_update] update.verify_digest=false 且无官方校验源，跳过校验（仅调试用途，生产严禁）: {}",
                    e
                );
                return Ok(());
            }
        };

        // 本地重算 SHA-256 并与官方基准比对（不匹配必拒，无论 verify_digest）
        let actual = sha256_hex_of_file(download_path)?;
        tracing::info!(
            "[system_update] SHA-256 校验：官方基准 {}，本地重算 {}",
            expected,
            actual
        );
        verify_sha256_matches(&expected, &actual)
    }

    /// 只从【官方域】取回校验值文件内容（.sha256）。
    /// host 必须过 `validate_official_digest_url`（收窄官方域，严禁从镜像取校验基准）+ SSRF；
    /// 重定向终点二次复核仍在官方域，否则拒绝。
    async fn fetch_official_digest_content(url: &str) -> Result<String, UpdateError> {
        // 校验值 URL 必须 https + 官方域（github.com / objects.githubusercontent.com）
        validate_official_digest_url(url)?;

        let (host, safe_addrs) = crate::utils::ssrf_guard::validate_url_and_resolve(url)
            .map_err(|e| UpdateError::NetworkError(format!("校验值 URL SSRF 校验失败: {}", e)))?;

        let client = reqwest::Client::builder()
            .user_agent("BingxiManagementPlatform/1.0")
            .redirect(reqwest::redirect::Policy::limited(3))
            .resolve_to_addrs(&host, &safe_addrs)
            .build()
            .map_err(|e| UpdateError::NetworkError(e.to_string()))?;

        let traceparent = crate::observability::trace_context::traceparent_from_current_span();
        let response = client
            .get(url)
            .header(
                crate::observability::trace_context::TRACEPARENT_HEADER,
                traceparent,
            )
            .send()
            .await
            .map_err(|e| UpdateError::NetworkError(e.to_string()))?;

        // 终点复核：校验值绝不能来自被重定向到的非官方域
        let final_url = response.url().clone();
        validate_official_digest_url(final_url.as_str())?;

        if !response.status().is_success() {
            return Err(UpdateError::NetworkError(format!(
                "官方校验值资产返回错误状态: {}",
                response.status()
            )));
        }

        response
            .text()
            .await
            .map_err(|e| UpdateError::NetworkError(e.to_string()))
    }

    /// 在发布信息中查找匹配的资源（按名称匹配或按扩展名自动选择 .zip/.tar.gz）
    fn find_release_asset<'a>(
        release: &'a GitHubRelease,
        asset_name: Option<&str>,
    ) -> Result<&'a GitHubAsset, UpdateError> {
        if let Some(name) = asset_name {
            release
                .assets
                .iter()
                .find(|a| a.name.contains(name))
                .ok_or_else(|| UpdateError::NetworkError(format!("找不到资源: {}", name)))
        } else {
            release
                .assets
                .iter()
                .find(|a| a.name.ends_with(".zip") || a.name.ends_with(".tar.gz"))
                .ok_or_else(|| UpdateError::NetworkError("找不到更新包".to_string()))
        }
    }

    /// 构建 SSRF 防御的下载客户端（URL 校验 + DNS Rebinding 防御 + 重定向限制）
    /// TS-S-7 安全加固：校验下载域名防止 SSRF / 中间人攻击。；M1 修复（v8 复审）：DNS Rebinding 防御，用 resolve_to_addrs 固定连接到已校验 IP。
    fn build_safe_download_client(url: &str) -> Result<reqwest::Client, UpdateError> {
        validate_download_url(url)?;
        let (dl_host, dl_safe_addrs) = crate::utils::ssrf_guard::validate_url_and_resolve(url)
            .map_err(|e| UpdateError::NetworkError(format!("下载 URL SSRF 校验失败: {}", e)))?;
        reqwest::Client::builder()
            .user_agent("BingxiManagementPlatform/1.0")
            .redirect(reqwest::redirect::Policy::limited(3))
            .resolve_to_addrs(&dl_host, &dl_safe_addrs)
            .build()
            .map_err(|e| UpdateError::NetworkError(e.to_string()))
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

// =====================================================
// 任务 #121：多镜像候选下载 URL 生成单测（官方永远列入且兜底）
// =====================================================
#[cfg(test)]
mod download_candidate_tests {
    use super::*;

    /// 构造测试配置。`use_default_mirrors` 默认置 false，使既有镜像用例只对显式镜像集合敏感、
    /// 与内置 [`DEFAULT_RELEASE_MIRRORS`] 解耦（默认档行为另由专用用例覆盖）。
    fn cfg(order: MirrorOrder, mirrors: &[&str]) -> UpdateConfig {
        UpdateConfig {
            mirrors: mirrors.iter().map(|s| s.to_string()).collect(),
            use_default_mirrors: false,
            mirror_order: order,
            verify_digest: true,
            connect_timeout_secs: 15,
            read_timeout_secs: 180,
        }
    }

    const OFFICIAL: &str = "https://github.com/57231307/1/releases/download/v1/release-1.tar.gz";

    #[test]
    fn no_mirrors_yields_only_official() {
        // 产品默认档为 use_default_mirrors=true；此用例断言的是"关闭内置默认 + 空运维清单"时
        // 候选仅剩官方（运维显式配置语义不变），与默认档行为解耦。
        let c = cfg(MirrorOrder::OfficialFirst, &[]);
        let got = SystemUpdateService::build_download_candidates(OFFICIAL, &c);
        assert_eq!(
            got,
            vec![OFFICIAL.to_string()],
            "关闭默认镜像且空清单时候选应仅官方"
        );
    }

    /// 默认档（`use_default_mirrors=true` + 空运维清单）：候选并入内置默认镜像，官方仍永远兜底。
    #[test]
    fn default_mirrors_on_includes_defaults_with_official_fallback() {
        let mut c = cfg(MirrorOrder::MirrorFirst, &[]);
        c.use_default_mirrors = true;
        let got = SystemUpdateService::build_download_candidates(OFFICIAL, &c);

        // 每个内置默认镜像派生一个候选（MirrorFirst：镜像在前）。
        for m in DEFAULT_RELEASE_MIRRORS {
            let expected = format!("{}/{}", m.trim_end_matches('/'), OFFICIAL);
            assert!(
                got.iter().any(|u| u == &expected),
                "默认镜像 {} 应进入候选",
                m
            );
        }
        assert_eq!(
            *got.last().unwrap(),
            OFFICIAL,
            "官方永远兜底：默认档 MirrorFirst 时官方排在末位"
        );
        assert_eq!(
            got.len(),
            DEFAULT_RELEASE_MIRRORS.len() + 1,
            "空运维清单 + 默认开启 = 内置默认数 + 官方兜底"
        );
    }

    /// 运维显式镜像与内置默认重叠时须去重，避免同一候选重复尝试。
    #[test]
    fn default_mirrors_on_dedupes_overlapping_config_mirror() {
        let dup = DEFAULT_RELEASE_MIRRORS[0];
        let mut c = cfg(
            MirrorOrder::OfficialFirst,
            &[dup, "https://ops-only.example.com"],
        );
        c.use_default_mirrors = true;
        let got = SystemUpdateService::build_download_candidates(OFFICIAL, &c);
        let dup_candidate = format!("{}/{}", dup.trim_end_matches('/'), OFFICIAL);
        let dup_count = got.iter().filter(|u| **u == dup_candidate).count();
        assert_eq!(dup_count, 1, "重叠镜像候选必须去重，实得 {dup_count} 次");
        // 排序口径判据：OfficialFirst 的成文契约是**官方优先首位、镜像兜底**
        // （本文件 build_download_candidates 的文档注释与实现：先 push official
        // 再 extend mirrors）。末位兜底是 **MirrorFirst** 分支的口径，由
        // mirror_first_places_official_last_as_fallback 用例钉住。
        // 本条断 got[0]==OFFICIAL，并**保留"官方必在候选"判据 + 数量正形约束**。
        assert_eq!(
            *got.first().unwrap(),
            OFFICIAL,
            "OfficialFirst 时官方必须排首位（镜像作兜底列其后）"
        );
        assert!(
            got.iter().any(|u| u == OFFICIAL),
            "官方永远必须列入候选（任何档位不得剔除）"
        );
        // 数量正形：官方 1 + 去重后镜像集合（运维 dup 已被默认吸收、ops-only 净增 1，
        // 加其余内置默认）= DEFAULT_RELEASE_MIRRORS.len() + 2
        assert_eq!(
            got.len(),
            DEFAULT_RELEASE_MIRRORS.len() + 2,
            "候选总数=官方1+去重镜像(内置{dm}去1重+运维净增1)，实得 {}（去重回潮或清单丢失都会在这里显形）",
            got.len(),
            dm = DEFAULT_RELEASE_MIRRORS.len()
        );
    }

    #[test]
    fn official_first_places_official_leading_and_mirrors_after() {
        let c = cfg(
            MirrorOrder::OfficialFirst,
            &["https://m1.example.com", "https://m2.example.com/"],
        );
        let got = SystemUpdateService::build_download_candidates(OFFICIAL, &c);
        assert_eq!(got.len(), 3);
        assert_eq!(got[0], OFFICIAL, "官方优先须排首位");
        assert_eq!(got[1], format!("https://m1.example.com/{OFFICIAL}"));
        // 镜像尾斜杠被 trim，避免拼接双斜杠
        assert_eq!(got[2], format!("https://m2.example.com/{OFFICIAL}"));
    }

    #[test]
    fn mirror_first_places_official_last_as_fallback() {
        let c = cfg(MirrorOrder::MirrorFirst, &["https://m1.example.com"]);
        let got = SystemUpdateService::build_download_candidates(OFFICIAL, &c);
        assert_eq!(got.len(), 2);
        assert_eq!(
            got[0],
            format!("https://m1.example.com/{OFFICIAL}"),
            "镜像优先"
        );
        assert_eq!(
            *got.last().unwrap(),
            OFFICIAL,
            "官方永远兜底：MirrorFirst 时官方排最后"
        );
    }

    #[test]
    fn official_never_omitted_regardless_of_order() {
        for order in [MirrorOrder::OfficialFirst, MirrorOrder::MirrorFirst] {
            let c = cfg(order, &["https://m1.example.com", "https://m2.example.com"]);
            let got = SystemUpdateService::build_download_candidates(OFFICIAL, &c);
            assert!(
                got.iter().any(|u| u == OFFICIAL),
                "{order:?} 下官方域必须在候选中"
            );
        }
    }
}
