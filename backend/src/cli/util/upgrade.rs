//! 升级 / 部署 / 回滚子命令实现：Upgrade / Deploy / Rollback
//!
//! 同时承载两个内部辅助函数 `get_latest_version`、`deploy_release`，
//! 它们由本文件内的 `cmd_upgrade` / `cmd_deploy` 使用。
//!
//! 批次 92 P3-8：原 20 处 `let _ = run_cmd(...)` 静默吞错已全部改为
//! `if let Err(e) = ... { println!("[ERROR]/[WARN] ...") }` 模式：
//! - 关键路径（stop/start/mv/cp/chmod/mkdir）失败记录 [ERROR]
//! - 清理路径（rm -rf temp）失败记录 [WARN]
//!
//! P0-D15（Batch 488）：新增蓝绿部署零停机升级模式。
//! - 检测 systemd template `bingxi-backend@.service` 是否已安装；
//! - 已安装 → 走 `deploy_release_blue_green` / `cmd_rollback_blue_green`（零停机）；
//! - 未安装 → 回退到原 `deploy_release_legacy` / `cmd_rollback_legacy`（停机模式）。
//! 蓝绿切换通过 nginx upstream include + `ln -sf` + `nginx -s reload` 实现，
//! 活跃/非活跃实例由 `/etc/nginx/bingxi-upstream.active.conf` 软链接判定。

use super::{
    build_release_url, download_with_mirrors, fetch_with_mirrors, get_backup_dir, get_install_dir,
    is_service_active, parse_json_field, run_cmd, timestamp, GITHUB_REPO,
};

// 批次 322 v9 复审低危修复：路径校验逻辑已抽取到共享模块 `utils::path_validator`，
// 此处复用，避免与 backup.rs 重复维护。测试覆盖见 path_validator 模块。
use crate::utils::path_validator::validate_extracted_paths;

// ==================== P0-D15 蓝绿部署常量 ====================

/// systemd template 服务前缀（实例名形如 `bingxi-backend@blue` / `bingxi-backend@green`）
const BLUE_GREEN_TEMPLATE: &str = "bingxi-backend@.service";

/// 蓝实例名
const BLUE_INSTANCE: &str = "blue";

/// 绿实例名
const GREEN_INSTANCE: &str = "green";

/// 蓝实例监听端口（与 deploy/instances/blue.env 一致）
const BLUE_PORT: &str = "8082";

/// 绿实例监听端口（与 deploy/instances/green.env 一致）
const GREEN_PORT: &str = "8083";

/// nginx upstream 活跃实例软链接路径（由 nginx.conf include）
const NGINX_UPSTREAM_ACTIVE: &str = "/etc/nginx/bingxi-upstream.active.conf";

/// 健康检查路径（公开路由，无需认证；routes/system.rs::health()）
const HEALTH_PATH: &str = "/api/v1/erp/health/readiness";

/// 健康检查重试次数（每次间隔 1 秒）
const HEALTH_CHECK_RETRIES: u8 = 15;

// ==================== P0-D15 蓝绿部署辅助函数 ====================

/// 检测当前是否为蓝绿部署模式（systemd template 已安装）。
fn is_blue_green_mode() -> bool {
    run_cmd(
        "systemctl",
        &["list-unit-files", BLUE_GREEN_TEMPLATE],
    )
    .map(|s| s.contains("bingxi-backend@"))
    .unwrap_or(false)
}

/// 获取当前活跃实例名（`blue` 或 `green`）。
///
/// 通过读取 `/etc/nginx/bingxi-upstream.active.conf` 软链接目标判定。
/// 返回 `None` 表示软链接缺失或目标无法识别。
fn get_active_instance() -> Option<String> {
    let target = run_cmd("readlink", &["-f", NGINX_UPSTREAM_ACTIVE]).ok()?;
    let t = target.trim();
    if t.ends_with("bingxi-upstream-blue.conf") || t.contains("blue") {
        Some(BLUE_INSTANCE.to_string())
    } else if t.ends_with("bingxi-upstream-green.conf") || t.contains("green") {
        Some(GREEN_INSTANCE.to_string())
    } else {
        None
    }
}

/// 实例名 → systemd 服务名（如 `blue` → `bingxi-backend@blue`）。
fn instance_service(name: &str) -> String {
    format!("bingxi-backend@{}", name)
}

/// 实例名 → 监听端口。
fn instance_port(name: &str) -> &'static str {
    if name == BLUE_INSTANCE {
        BLUE_PORT
    } else {
        GREEN_PORT
    }
}

/// 实例名 → 对侧实例名（blue↔green）。
fn opposite_instance(name: &str) -> &'static str {
    if name == BLUE_INSTANCE {
        GREEN_INSTANCE
    } else {
        BLUE_INSTANCE
    }
}

/// 对指定实例执行健康检查（GET `/api/v1/erp/health/readiness`）。
///
/// 重试 `HEALTH_CHECK_RETRIES` 次，每次间隔 1 秒；任一次成功即返回 `true`。
fn health_check_instance(instance: &str) -> bool {
    let port = instance_port(instance);
    let url = format!("http://127.0.0.1:{}{}", port, HEALTH_PATH);
    for i in 0..HEALTH_CHECK_RETRIES {
        if run_cmd("curl", &["-fsSL", "-m", "3", &url]).is_ok() {
            println!("  [OK] 健康检查通过（第 {} 次）", i + 1);
            return true;
        }
        if i < HEALTH_CHECK_RETRIES - 1 {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
    false
}

/// 切换 nginx upstream 到指定实例（软链接 + nginx -t + nginx -s reload）。
fn switch_nginx_upstream(target: &str) -> Result<(), String> {
    let upstream_file = format!("/etc/nginx/bingxi-upstream-{}.conf", target);
    if !std::path::Path::new(&upstream_file).exists() {
        return Err(format!(
            "upstream 配置文件不存在: {}（请确认 deploy/nginx-upstream-{}.conf 已部署）",
            upstream_file, target
        ));
    }
    run_cmd("ln", &["-sf", &upstream_file, NGINX_UPSTREAM_ACTIVE])?;
    run_cmd("nginx", &["-t"])?;
    run_cmd("nginx", &["-s", "reload"])?;
    Ok(())
}

/// 清理临时目录（非关键路径，失败仅 warn）。
fn cleanup_temp(temp_dir: &str) {
    if let Err(e) = run_cmd("rm", &["-rf", temp_dir]) {
        println!("[WARN] 清理临时目录失败（可忽略）: {}", e);
    }
}

pub(super) fn cmd_upgrade(version: Option<String>, no_backup: bool) {
    println!("=== 系统升级 ===\n");

    let current = env!("CARGO_PKG_VERSION");
    println!("当前版本: v{}", current);

    // 获取目标版本
    let target = match &version {
        Some(v) => {
            let v = if v.starts_with('v') {
                v.clone()
            } else {
                format!("v{}", v)
            };
            println!("目标版本: {}", v);
            v
        }
        None => {
            println!("获取最新版本...");
            match get_latest_version() {
                Some(v) => {
                    println!("最新版本: {}", v);
                    v
                }
                None => {
                    println!("[ERROR] 无法获取最新版本");
                    println!("\n请手动指定版本:");
                    println!("  bingxi upgrade --version v2026.x.x.xxxx");
                    println!("\n或手动下载后使用 deploy 命令:");
                    println!("  bingxi deploy --package release-xxx.tar.gz");
                    return;
                }
            }
        }
    };

    // 备份
    if !no_backup {
        println!("\n备份当前版本...");
        // L4 修复：检查备份结果，失败则中止升级
        if !super::backup::cmd_backup("all") {
            println!("[ERROR] 备份失败，终止升级");
            return;
        }
    }

    // 下载
    println!("\n下载新版本...");
    let download_path = format!("/tmp/release-{}.tar.gz", target);
    let release_url = build_release_url(&target);

    if !download_with_mirrors(&release_url, &download_path, 120) {
        println!("\n[ERROR] 下载失败");
        println!("\n请手动下载:");
        println!(
            "  curl -fsSL -o /tmp/release-{}.tar.gz {}",
            target, release_url
        );
        println!("\n然后执行:");
        println!("  bingxi deploy --package /tmp/release-{}.tar.gz", target);
        return;
    }

    // 部署
    println!("\n部署新版本...");
    deploy_release(&download_path);

    // 清理下载包（非关键路径）
    if let Err(e) = run_cmd("rm", &["-f", &download_path]) {
        println!("[WARN] 清理下载包失败（可忽略）: {}", e);
    }

    println!("\n[OK] 升级完成");
    println!("新版本: {}", target);
    println!("备份位置: {}", get_backup_dir());
    println!("\n如需回滚: bingxi rollback");
}

pub(super) fn cmd_deploy(package: &str) {
    println!("=== 部署更新包 ===\n");
    println!("更新包: {}", package);

    if !std::path::Path::new(package).exists() {
        println!("[ERROR] 文件不存在: {}", package);
        return;
    }

    deploy_release(package);

    println!("\n[OK] 部署完成");
}

pub(super) fn cmd_rollback() {
    println!("=== 回滚版本 ===\n");

    let server_old = format!("{}/backend/server.old", get_install_dir());
    let bingxi_old = format!("{}/backend/bingxi.old", get_install_dir());

    if !std::path::Path::new(&server_old).exists() {
        println!("[ERROR] 未找到旧版本文件");
        println!("请确认之前执行过升级操作");
        return;
    }

    // P0-D15：检测蓝绿部署模式后分发
    if is_blue_green_mode() {
        cmd_rollback_blue_green(&server_old, &bingxi_old);
    } else {
        cmd_rollback_legacy(&server_old, &bingxi_old);
    }
}

/// P0-D15：蓝绿模式回滚（零停机）。
///
/// 流程：
/// 1. 识别当前活跃实例（运行新版本/有问题的版本）与非活跃实例；
/// 2. 停止非活跃实例（如运行中）；
/// 3. 将 `server.old` / `bingxi.old` 恢复到 `/opt/bingxi-erp/backend/` 路径；
/// 4. 启动非活跃实例（加载旧版本二进制）；
/// 5. 健康检查非活跃实例；
/// 6. 切换 nginx upstream 到非活跃实例（零停机）；
/// 7. 停止原活跃实例（新版本/有问题的版本）。
///
/// 任一关键步骤失败立即中止，活跃实例继续服务，避免回滚中途宕机。
fn cmd_rollback_blue_green(server_old: &str, bingxi_old: &str) {
    println!("=== 蓝绿回滚模式（零停机）===");

    let active = match get_active_instance() {
        Some(a) => a,
        None => {
            println!("[ERROR] 无法确定活跃实例");
            println!(
                "请检查 {} 软链接是否指向 blue 或 green 配置",
                NGINX_UPSTREAM_ACTIVE
            );
            println!("或回退到单实例模式（移除 bingxi-backend@.service 后重试）");
            return;
        }
    };
    let inactive = opposite_instance(&active).to_string();
    let inactive_service = instance_service(&inactive);
    let active_service = instance_service(&active);

    println!("当前活跃实例: {} ({})", active, instance_port(&active));
    println!("回滚目标实例: {} ({})", inactive, instance_port(&inactive));

    // 1. 停止非活跃实例（如运行中，确保干净启动）
    println!("停止非活跃实例 {}（如运行中）...", inactive_service);
    let _ = run_cmd("systemctl", &["stop", &inactive_service]);

    // 2. 恢复旧版本二进制（关键路径，任一失败立即中止）
    println!("恢复旧版本二进制...");
    let server_path = format!("{}/backend/server", get_install_dir());
    let bingxi_path = format!("{}/backend/bingxi", get_install_dir());

    if let Err(e) = run_cmd("cp", &["-f", server_old, &server_path]) {
        println!("[ERROR] 恢复 server 失败，终止回滚: {}", e);
        println!("活跃实例 {} 继续服务，未受影响", active_service);
        return;
    }
    if let Err(e) = run_cmd("cp", &["-f", bingxi_old, &bingxi_path]) {
        println!("[ERROR] 恢复 bingxi 失败，终止回滚: {}", e);
        println!("活跃实例 {} 继续服务，未受影响", active_service);
        return;
    }
    if let Err(e) = run_cmd("chmod", &["+x", &server_path]) {
        println!("[ERROR] chmod server 失败，终止回滚: {}", e);
        println!("活跃实例 {} 继续服务，未受影响", active_service);
        return;
    }
    if let Err(e) = run_cmd("chmod", &["+x", &bingxi_path]) {
        println!("[ERROR] chmod bingxi 失败，终止回滚: {}", e);
        println!("活跃实例 {} 继续服务，未受影响", active_service);
        return;
    }

    // 3. 启动非活跃实例（加载旧版本二进制）
    println!("启动非活跃实例 {}...", inactive_service);
    if let Err(e) = run_cmd("systemctl", &["start", &inactive_service]) {
        println!("[ERROR] 启动 {} 失败: {}", inactive_service, e);
        println!("活跃实例 {} 继续服务，未受影响", active_service);
        return;
    }

    // 4. 健康检查非活跃实例
    println!("健康检查回滚实例 {}...", inactive_service);
    if !health_check_instance(&inactive) {
        println!("[ERROR] 回滚实例健康检查失败");
        println!("停止回滚实例，活跃实例 {} 继续服务", active_service);
        let _ = run_cmd("systemctl", &["stop", &inactive_service]);
        return;
    }

    // 5. 切换 nginx upstream（零停机）
    println!("切换 nginx upstream → {}...", inactive);
    if let Err(e) = switch_nginx_upstream(&inactive) {
        println!("[ERROR] nginx 切换失败: {}", e);
        println!("停止回滚实例，活跃实例 {} 继续服务", active_service);
        let _ = run_cmd("systemctl", &["stop", &inactive_service]);
        return;
    }

    // 6. 停止原活跃实例（新版本/有问题的版本）
    println!("停止原活跃实例 {}...", active_service);
    if let Err(e) = run_cmd("systemctl", &["stop", &active_service]) {
        println!("[WARN] 停止原活跃实例失败（可手动停止）: {}", e);
    }

    println!("\n[OK] 蓝绿回滚成功");
    println!("新活跃实例: {} ({})", inactive, instance_port(&inactive));
    println!("旧实例 {} 已停止（如需重启新版本可手动启动）", active_service);
}

/// 单实例模式回滚（原 cmd_rollback 逻辑，停机模式）。
fn cmd_rollback_legacy(server_old: &str, bingxi_old: &str) {
    println!("停止服务...");
    if let Err(e) = run_cmd("systemctl", &["stop", super::SERVICE_NAME]) {
        println!("[ERROR] 停止服务失败（继续回滚）: {}", e);
    }
    std::thread::sleep(std::time::Duration::from_secs(2));

    println!("恢复旧版本...");
    let server_path = format!("{}/backend/server", get_install_dir());
    let bingxi_path = format!("{}/backend/bingxi", get_install_dir());

    // 批次 95 P3-13 修复：恢复旧版本为关键路径，任一步失败立即中止回滚
    // （避免后续 chmod/start 对缺失文件误操作）
    if let Err(e) = run_cmd("mv", &[server_old, &server_path]) {
        println!("[ERROR] 恢复 server 失败，终止回滚: {}", e);
        return;
    }
    if let Err(e) = run_cmd("mv", &[bingxi_old, &bingxi_path]) {
        println!("[ERROR] 恢复 bingxi 失败，终止回滚: {}", e);
        return;
    }
    if let Err(e) = run_cmd("chmod", &["+x", &server_path]) {
        println!("[ERROR] chmod server 失败，终止回滚: {}", e);
        return;
    }
    if let Err(e) = run_cmd("chmod", &["+x", &bingxi_path]) {
        println!("[ERROR] chmod bingxi 失败，终止回滚: {}", e);
        return;
    }

    println!("启动服务...");
    if let Err(e) = run_cmd("systemctl", &["start", super::SERVICE_NAME]) {
        println!("[ERROR] 启动服务失败: {}", e);
    }

    std::thread::sleep(std::time::Duration::from_secs(3));

    if is_service_active(super::SERVICE_NAME) {
        println!("\n[OK] 回滚成功");
    } else {
        println!("\n[ERROR] 回滚后服务启动失败，请检查日志");
    }
}

/// 获取最新版本号
fn get_latest_version() -> Option<String> {
    let api_path = format!("repos/{}/releases/latest", GITHUB_REPO);

    if let Some(json) = fetch_with_mirrors(&api_path, 15) {
        return parse_json_field(&json, "tag_name");
    }

    None
}

// 批次 322 v9 复审低危修复：`validate_extracted_path` 已抽取到共享模块
// `utils::path_validator::validate_extracted_paths`，此处不再重复维护。

/// 部署发布包。
///
/// P0-D15：根据 systemd template 是否已安装，自动分发到蓝绿零停机模式或单实例停机模式。
fn deploy_release(package: &str) {
    if is_blue_green_mode() {
        deploy_release_blue_green(package);
    } else {
        deploy_release_legacy(package);
    }
}

/// P0-D15：蓝绿模式部署（零停机）。
///
/// 流程：
/// 1. 识别当前活跃实例与非活跃实例；
/// 2. 校验 + 解压 + 安全检查发布包；
/// 3. 备份当前二进制到 `old.{timestamp}` 目录与 `server.old` / `bingxi.old` 标记文件；
/// 4. 替换 `/opt/bingxi-erp/backend/{server,bingxi}` 二进制（活跃实例继续服务，Linux 允许替换 in-use 二进制）；
/// 5. 替换前端 dist；
/// 6. 停止非活跃实例（如运行中）→ 启动非活跃实例（加载新二进制）；
/// 7. 健康检查非活跃实例（最长 15 秒重试）；
/// 8. 切换 nginx upstream 到非活跃实例（`ln -sf` + `nginx -s reload`，零停机）；
/// 9. 停止原活跃实例。
///
/// 任一关键步骤失败立即中止，活跃实例继续服务，避免部署中途宕机。
fn deploy_release_blue_green(package: &str) {
    println!("=== 蓝绿部署模式（零停机）===");

    // 1. 确定活跃/非活跃实例
    let active = match get_active_instance() {
        Some(a) => a,
        None => {
            println!("[ERROR] 无法确定活跃实例");
            println!(
                "请检查 {} 软链接是否指向 blue 或 green 配置",
                NGINX_UPSTREAM_ACTIVE
            );
            println!("或回退到单实例模式（移除 bingxi-backend@.service 后重试）");
            return;
        }
    };
    let inactive = opposite_instance(&active).to_string();
    let inactive_service = instance_service(&inactive);
    let active_service = instance_service(&active);

    println!("当前活跃实例: {} ({})", active, instance_port(&active));
    println!("部署目标实例: {} ({})", inactive, instance_port(&inactive));

    // 2. 解压 + 安全校验（与 legacy 共用逻辑，H-1 修复 UUID 随机目录 + 二次校验）
    let temp_dir_owned = format!(
        "{}/bingxi_upgrade_{}",
        std::env::temp_dir().to_string_lossy(),
        uuid::Uuid::new_v4()
    );
    let temp_dir = temp_dir_owned.as_str();

    if let Err(e) = run_cmd("mkdir", &["-p", temp_dir]) {
        println!("[ERROR] 创建临时目录失败，终止部署: {}", e);
        return;
    }

    println!("校验更新包内容...");
    let tar_list = match run_cmd("tar", &["-tf", package]) {
        Ok(list) => list,
        Err(e) => {
            println!("[ERROR] 列出更新包内容失败: {}", e);
            cleanup_temp(temp_dir);
            return;
        }
    };

    for line in tar_list.lines() {
        let path = line.trim();
        if path.is_empty() || path == "./" {
            continue;
        }
        if path.contains("..") {
            println!("[ERROR] 检测到路径穿越攻击：文件 {} 包含 ..", path);
            cleanup_temp(temp_dir);
            return;
        }
        if path.starts_with('/') {
            println!("[ERROR] 检测到绝对路径：文件 {}", path);
            cleanup_temp(temp_dir);
            return;
        }
    }

    println!("解压更新包...");
    if let Err(e) = run_cmd("tar", &["-xzf", package, "-C", temp_dir]) {
        println!("[ERROR] 解压失败，终止部署: {}", e);
        cleanup_temp(temp_dir);
        return;
    }

    let extract_dir = format!("{}/bingxi-erp", temp_dir);
    if let Err(e) = validate_extracted_paths(&extract_dir) {
        println!("[ERROR] 安全校验失败，终止部署: {}", e);
        cleanup_temp(temp_dir);
        return;
    }

    let install_dir = get_install_dir();

    // 3. 备份旧二进制（带时间戳目录 + .old 标记供 rollback）
    println!("备份旧文件...");
    let ts = timestamp();
    let old_backup = format!("{}/old.{}", install_dir, ts);
    if let Err(e) = run_cmd("mkdir", &["-p", &old_backup]) {
        println!("[ERROR] 创建旧文件备份目录失败，终止部署: {}", e);
        cleanup_temp(temp_dir);
        return;
    }
    let server_src = format!("{}/backend/server", install_dir);
    let bingxi_src = format!("{}/backend/bingxi", install_dir);
    if let Err(e) = run_cmd("cp", &["-r", &server_src, &old_backup]) {
        println!("[ERROR] 备份 server 失败: {}", e);
    }
    if let Err(e) = run_cmd("cp", &["-r", &bingxi_src, &old_backup]) {
        println!("[ERROR] 备份 bingxi 失败: {}", e);
    }
    // 同步刷新 .old 标记文件（供 rollback 直接 cp）
    let server_old = format!("{}/backend/server.old", install_dir);
    let bingxi_old = format!("{}/backend/bingxi.old", install_dir);
    if let Err(e) = run_cmd("cp", &["-f", &server_src, &server_old]) {
        println!("[WARN] 刷新 server.old 失败（不影响部署）: {}", e);
    }
    if let Err(e) = run_cmd("cp", &["-f", &bingxi_src, &bingxi_old]) {
        println!("[WARN] 刷新 bingxi.old 失败（不影响部署）: {}", e);
    }

    // 4. 替换后端二进制（活跃实例继续服务，Linux 允许替换 in-use 二进制）
    println!("更新后端二进制...");
    let new_server = format!("{}/backend/server", extract_dir);
    let new_bingxi = format!("{}/backend/bingxi", extract_dir);
    let dst_server = format!("{}/backend/server", install_dir);
    let dst_bingxi = format!("{}/backend/bingxi", install_dir);

    if let Err(e) = run_cmd("cp", &["-r", &new_server, &dst_server]) {
        println!("[ERROR] 覆盖 server 失败，终止部署: {}", e);
        cleanup_temp(temp_dir);
        return;
    }
    if let Err(e) = run_cmd("cp", &["-r", &new_bingxi, &dst_bingxi]) {
        println!("[ERROR] 覆盖 bingxi 失败，终止部署: {}", e);
        cleanup_temp(temp_dir);
        return;
    }
    if let Err(e) = run_cmd("chmod", &["+x", &dst_server]) {
        println!("[ERROR] chmod server 失败，终止部署: {}", e);
        cleanup_temp(temp_dir);
        return;
    }
    if let Err(e) = run_cmd("chmod", &["+x", &dst_bingxi]) {
        println!("[ERROR] chmod bingxi 失败，终止部署: {}", e);
        cleanup_temp(temp_dir);
        return;
    }

    // 5. 更新前端
    println!("更新前端...");
    let frontend_dist = format!("{}/frontend/dist", install_dir);
    if let Err(e) = run_cmd("rm", &["-rf", &frontend_dist]) {
        println!("[WARN] 清理旧前端 dist 失败（继续 mv 覆盖）: {}", e);
    }
    let new_dist = format!("{}/frontend/dist", extract_dir);
    if let Err(e) = run_cmd("mv", &[&new_dist, &frontend_dist]) {
        println!("[ERROR] 移动新前端 dist 失败，终止部署: {}", e);
        cleanup_temp(temp_dir);
        return;
    }

    cleanup_temp(temp_dir);

    // 6. 停止非活跃实例（如运行中）→ 启动非活跃实例
    println!("停止非活跃实例 {}（如运行中）...", inactive_service);
    let _ = run_cmd("systemctl", &["stop", &inactive_service]);

    println!("启动非活跃实例 {}...", inactive_service);
    if let Err(e) = run_cmd("systemctl", &["start", &inactive_service]) {
        println!("[ERROR] 启动 {} 失败: {}", inactive_service, e);
        println!("活跃实例 {} 继续服务，未受影响", active_service);
        return;
    }

    // 7. 健康检查（最长 15 秒重试）
    println!("健康检查新实例 {}...", inactive_service);
    if !health_check_instance(&inactive) {
        println!("[ERROR] 新实例健康检查失败");
        println!("停止新实例，活跃实例 {} 继续服务", active_service);
        let _ = run_cmd("systemctl", &["stop", &inactive_service]);
        return;
    }

    // 8. 切换 nginx upstream（零停机）
    println!("切换 nginx upstream → {}...", inactive);
    if let Err(e) = switch_nginx_upstream(&inactive) {
        println!("[ERROR] nginx 切换失败: {}", e);
        println!("停止新实例，活跃实例 {} 继续服务", active_service);
        let _ = run_cmd("systemctl", &["stop", &inactive_service]);
        return;
    }

    // 9. 停止旧实例
    println!("停止旧实例 {}...", active_service);
    if let Err(e) = run_cmd("systemctl", &["stop", &active_service]) {
        println!("[WARN] 停止旧实例失败（可手动停止）: {}", e);
    }

    println!("\n[OK] 蓝绿部署成功");
    println!("新活跃实例: {} ({})", inactive, instance_port(&inactive));
    println!("如需回滚: bingxi rollback");
}

/// 单实例模式部署（原 deploy_release 逻辑，停机模式）。
///
/// 保留作为非蓝绿部署环境的回退路径。
fn deploy_release_legacy(package: &str) {
    println!("停止服务...");
    stop_service_for_legacy_deploy();
    std::thread::sleep(std::time::Duration::from_secs(2));

    let temp_dir = match prepare_random_temp_dir() {
        Ok(d) => d,
        Err(e) => {
            println!("[ERROR] 创建临时目录失败，终止部署: {}", e);
            return;
        }
    };

    // H-1 修复（v9 复审）：UUID 随机目录 + 先 tar -tf 校验再解压 + 二次校验，防止 Tar Slip 与符号链接竞争
    if let Err(e) = validate_tar_contents(package) {
        println!("[ERROR] {}", e);
        cleanup_temp(&temp_dir);
        return;
    }

    let extract_dir = match extract_package_and_validate(package, &temp_dir) {
        Ok(d) => d,
        Err(e) => {
            println!("[ERROR] {}", e);
            cleanup_temp(&temp_dir);
            return;
        }
    };

    let install_dir = get_install_dir();
    if let Err(e) = backup_old_files_legacy(&install_dir) {
        println!("[ERROR] {}", e);
        cleanup_temp(&temp_dir);
        return;
    }
    if let Err(e) = copy_new_backend(&extract_dir, &install_dir) {
        println!("[ERROR] {}", e);
        cleanup_temp(&temp_dir);
        return;
    }
    if let Err(e) = copy_new_frontend(&extract_dir, &install_dir) {
        println!("[ERROR] {}", e);
        cleanup_temp(&temp_dir);
        return;
    }
    cleanup_temp(&temp_dir);
    start_service_and_check();
}

/// 停止 systemd 服务（非关键路径，失败仅记录继续部署）。
fn stop_service_for_legacy_deploy() {
    if let Err(e) = run_cmd("systemctl", &["stop", super::SERVICE_NAME]) {
        println!("[ERROR] 停止服务失败（继续部署）: {}", e);
    }
}

/// 创建 UUID 随机临时目录（关键路径，失败终止部署）。
fn prepare_random_temp_dir() -> Result<String, String> {
    let temp_dir = format!(
        "{}/bingxi_upgrade_{}",
        std::env::temp_dir().to_string_lossy(),
        uuid::Uuid::new_v4()
    );
    run_cmd("mkdir", &["-p", &temp_dir]).map_err(|e| format!("创建临时目录失败: {}", e))?;
    Ok(temp_dir)
}

/// 先列出 tar 内容并校验路径，防止恶意文件在校验前写入磁盘（Tar Slip 防护）。
fn validate_tar_contents(package: &str) -> Result<(), String> {
    println!("校验更新包内容...");
    let tar_list = run_cmd("tar", &["-tf", package])
        .map_err(|e| format!("列出更新包内容失败: {}", e))?;
    for line in tar_list.lines() {
        let path = line.trim();
        if path.is_empty() || path == "./" {
            continue;
        }
        if path.contains("..") {
            return Err(format!("检测到路径穿越攻击：文件 {} 包含 ..", path));
        }
        if path.starts_with('/') {
            return Err(format!("检测到绝对路径：文件 {}", path));
        }
    }
    Ok(())
}

/// 解压到随机临时目录并做二次校验（canonicalize 解析符号链接，双重防护）。
fn extract_package_and_validate(package: &str, temp_dir: &str) -> Result<String, String> {
    println!("解压更新包...");
    run_cmd("tar", &["-xzf", package, "-C", temp_dir])
        .map_err(|e| format!("解压失败: {}", e))?;
    // 批次 322 v9 复审低危修复：改用共享模块 utils::path_validator::validate_extracted_paths
    let extract_dir = format!("{}/bingxi-erp", temp_dir);
    validate_extracted_paths(&extract_dir).map_err(|e| format!("安全校验失败: {}", e))?;
    Ok(extract_dir)
}

/// 备份旧后端文件到 old.{ts} 目录（非关键路径，单文件失败仅记录）。
fn backup_old_files_legacy(install_dir: &str) -> Result<(), String> {
    println!("备份旧文件...");
    let ts = timestamp();
    let old_backup = format!("{}/old.{}", install_dir, ts);
    run_cmd("mkdir", &["-p", &old_backup])
        .map_err(|e| format!("创建旧文件备份目录失败: {}", e))?;
    let server_src = format!("{}/backend/server", install_dir);
    let bingxi_src = format!("{}/backend/bingxi", install_dir);
    if let Err(e) = run_cmd("cp", &["-r", &server_src, &old_backup]) {
        println!("[ERROR] 备份 server 失败: {}", e);
    }
    if let Err(e) = run_cmd("cp", &["-r", &bingxi_src, &old_backup]) {
        println!("[ERROR] 备份 bingxi 失败: {}", e);
    }
    Ok(())
}

/// 覆盖后端二进制并 chmod（批次 95 P3-13：关键路径，失败立即中止部署）。
fn copy_new_backend(extract_dir: &str, install_dir: &str) -> Result<(), String> {
    println!("更新后端...");
    let new_server = format!("{}/backend/server", extract_dir);
    let new_bingxi = format!("{}/backend/bingxi", extract_dir);
    let dst_server = format!("{}/backend/server", install_dir);
    let dst_bingxi = format!("{}/backend/bingxi", install_dir);
    run_cmd("cp", &["-r", &new_server, &dst_server]).map_err(|e| format!("覆盖 server 失败: {}", e))?;
    run_cmd("cp", &["-r", &new_bingxi, &dst_bingxi]).map_err(|e| format!("覆盖 bingxi 失败: {}", e))?;
    run_cmd("chmod", &["+x", &dst_server]).map_err(|e| format!("chmod server 失败: {}", e))?;
    run_cmd("chmod", &["+x", &dst_bingxi]).map_err(|e| format!("chmod bingxi 失败: {}", e))?;
    Ok(())
}

/// 移动新前端 dist（批次 95 P3-13：关键路径，失败立即中止部署，避免前端缺失上线）。
fn copy_new_frontend(extract_dir: &str, install_dir: &str) -> Result<(), String> {
    println!("更新前端...");
    let frontend_dist = format!("{}/frontend/dist", install_dir);
    if let Err(e) = run_cmd("rm", &["-rf", &frontend_dist]) {
        println!("[WARN] 清理旧前端 dist 失败（继续 mv 覆盖）: {}", e);
    }
    let new_dist = format!("{}/frontend/dist", extract_dir);
    run_cmd("mv", &[&new_dist, &frontend_dist]).map_err(|e| format!("移动新前端 dist 失败: {}", e))?;
    Ok(())
}

/// 启动服务并健康检查（启动失败仅记录，等待运维介入）。
fn start_service_and_check() {
    println!("启动服务...");
    if let Err(e) = run_cmd("systemctl", &["start", super::SERVICE_NAME]) {
        println!("[ERROR] 启动服务失败: {}", e);
    }
    std::thread::sleep(std::time::Duration::from_secs(3));
    if is_service_active(super::SERVICE_NAME) {
        println!("[OK] 部署成功");
    } else {
        println!("[ERROR] 服务启动失败，请检查日志");
    }
}
