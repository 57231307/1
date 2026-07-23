//! GitHub 远程更新子模块（system_update_ops/github）
//!
//! 从原 `system_update_service.rs` 迁移 8 个方法：
//! - `check_for_updates`：查询 GitHub Releases 最新版本（pub，handler 调用）
//! - `fetch_latest_release`：调用 GitHub API `/repos/{owner}/{repo}/releases/latest`（私有）
//! - `compare_versions`：比较 current < latest（`pub(crate)`，供 status 子模块 `check_local_updates`
//!  + facade 测试调用）
//! - `compare_versions_for_sort`：版本号排序比较（`pub(crate)`，供 status 子模块 `list_local_releases` 调用）
//! - `download_update`：下载 GitHub Release asset（pub，handler + `download_and_update` 调用）
//! - `find_release_asset`：在 Release 中查找匹配 asset（私有，关联函数）
//! - `build_safe_download_client`：构建 SSRF 防御下载客户端（私有，关联函数）
//! - `download_and_update`：下载并应用更新（pub，handler 调用）
//!
//! 跨模块依赖：
//! - `check_for_updates` 调用 `status::get_current_version`（pub）
//! - `compare_versions` / `compare_versions_for_sort` 调用 facade 纯函数 `parse_version`（`pub(crate)`）
//! - `download_update` 调用 `apply::log_update`（`pub(crate)`）+ facade 纯函数
//!  `validate_asset_name` / `validate_download_url`（`pub(crate)`）
//! - `download_and_update` 调用 `apply::apply_update`（pub）+ `apply::log_update`（`pub(crate)`）
//! - `fetch_latest_release` 使用 facade 常量 `GITHUB_API_URL` / `GITHUB_REPO`（`pub(crate)`）

use crate::services::system_update_service::{
    parse_version, validate_asset_name, validate_download_url, GitHubAsset, GitHubRelease,
    SystemUpdateService, UpdateCheckResult, UpdateError,
};
use crate::services::system_update_service::{GITHUB_API_URL, GITHUB_REPO};
use std::fs;
use std::io;
use std::path::PathBuf;

impl SystemUpdateService {
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
            Err(e) => UpdateCheckResult {
                has_update: false,
                current_version: current_version.clone(),
                latest_version: current_version,
                release_info: None,
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
            .map_err(|e| UpdateError::NetworkError(format!("GitHub API URL SSRF 校验失败: {}", e)))?;

        let client = reqwest::Client::builder()
            .user_agent("BingxiERP/1.0")
            .redirect(reqwest::redirect::Policy::limited(3))
            .resolve_to_addrs(&api_host, &api_safe_addrs)
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

    pub(crate) fn compare_versions(&self, current: &str, latest: &str) -> bool {
        // 批次 322 v9 复审低危修复：parse_version 抽取为共享函数，消除与
        // compare_versions_for_sort 的逻辑重复
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

    pub(crate) fn compare_versions_for_sort(&self, a: &str, b: &str) -> std::cmp::Ordering {
        // 批次 322 v9 复审低危修复：parse_version 抽取为共享函数，消除与
        // compare_versions 的逻辑重复
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

        let client = Self::build_safe_download_client(&asset.browser_download_url)?;

        let mut response = client
            .get(&asset.browser_download_url)
            .send()
            .await
            .map_err(|e| UpdateError::NetworkError(e.to_string()))?;

        // TS-S-7：二次校验最终跳转后的 URL 域名
        let final_url = response.url();
        validate_download_url(final_url.as_str())?;

        let mut file = fs::File::create(&download_path)?;
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| UpdateError::NetworkError(e.to_string()))?
        {
            io::copy(&mut chunk.as_ref(), &mut file)?;
        }

        self.log_update(&format!("更新包下载完成: {:?}", download_path));
        Ok(download_path)
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
    ///
    /// TS-S-7 安全加固：校验下载域名防止 SSRF / 中间人攻击。
    /// M1 修复（v8 复审）：DNS Rebinding 防御，用 resolve_to_addrs 固定连接到已校验 IP。
    fn build_safe_download_client(url: &str) -> Result<reqwest::Client, UpdateError> {
        validate_download_url(url)?;
        let (dl_host, dl_safe_addrs) = crate::utils::ssrf_guard::validate_url_and_resolve(url)
            .map_err(|e| UpdateError::NetworkError(format!("下载 URL SSRF 校验失败: {}", e)))?;
        reqwest::Client::builder()
            .user_agent("BingxiERP/1.0")
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
