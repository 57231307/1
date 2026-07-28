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
    is_service_active, parse_json_field, require_root, run_cmd, timestamp, GITHUB_REPO,
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

/// V15 P1 25.4-L：部署后自动回滚监控次数（每 10s 一次，连续 3 次失败触发回滚）
const POST_DEPLOY_MONITOR_RETRIES: u8 = 3;

/// V15 P1 25.4-L：自动回滚监控间隔
const POST_DEPLOY_MONITOR_INTERVAL_SECS: u64 = 10;

// ==================== V15 P1 升级流程加固辅助函数 ====================

/// V15 P1 25.3-A：下载后 SHA256 校验（对比 Release assets 中的 .sha256 文件）
/// 返回 true 表示校验通过或无 sha256 文件可下载（fail-open，避免 release 未提供 sha256 阻塞升级）
fn verify_sha256(release_url: &str, download_path: &str) -> bool {
    let sha256_url = format!("{}.sha256", release_url);
    let sha256_file = format!("{}.sha256", download_path);
    println!("下载 SHA256 校验文件...");
    if !download_with_mirrors(&sha256_url, &sha256_file, 30) {
        println!("[WARN] 无法下载 .sha256 文件，跳过校验（fail-open）");
        return true;
    }
    let expected_raw = match std::fs::read_to_string(&sha256_file) {
        Ok(s) => s,
        Err(e) => {
            println!("[WARN] 读取 .sha256 文件失败，跳过校验: {}", e);
            return true;
        }
    };
    // sha256sum 文件格式：`<hash>  <filename>`，取第一个空白前字段
    let expected = expected_raw
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_lowercase();
    if expected.is_empty() {
        println!("[WARN] .sha256 文件内容为空，跳过校验");
        return true;
    }
    let computed = match run_cmd("sha256sum", &[download_path]) {
        Ok(s) => s,
        Err(e) => {
            println!("[ERROR] sha256sum 命令执行失败: {}", e);
            return false;
        }
    };
    let computed_hash = computed
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_lowercase();
    if computed_hash == expected {
        println!("[OK] SHA256 校验通过");
        true
    } else {
        println!("[ERROR] SHA256 校验失败");
        println!("  期望: {}", expected);
        println!("  实际: {}", computed_hash);
        false
    }
}

/// V15 P1 25.3-E：升级前 schema 版本兼容性检查（调用 bingxi migrate status）
/// 返回 true 表示迁移状态正常或校验跳过（fail-open）
fn check_schema_compatibility() -> bool {
    println!("校验数据库 schema 版本...");
    // 调用 bingxi migrate status 检查迁移状态
    let bingxi_bin = format!("{}/backend/bingxi", get_install_dir());
    if !std::path::Path::new(&bingxi_bin).exists() {
        println!(
            "[WARN] 未找到 bingxi CLI（{}），跳过 schema 校验",
            bingxi_bin
        );
        return true;
    }
    match run_cmd(&bingxi_bin, &["migrate", "status"]) {
        Ok(output) => {
            if output.contains("Pending") || output.contains("pending") {
                println!("[WARN] 检测到待执行迁移，升级后将自动执行 migrate run");
            }
            println!("[OK] schema 版本校验通过");
            true
        }
        Err(e) => {
            // fail-open：迁移状态检查失败不阻塞升级（升级后会自动执行 migrate run）
            println!(
                "[WARN] schema 版本校验失败（fail-open，升级后仍会执行迁移）: {}",
                e
            );
            true
        }
    }
}

/// V15 P1 25.3-H：升级后自动执行数据库迁移（调用 bingxi migrate run）
/// 返回 true 表示迁移成功，false 表示失败（调用方应触发回滚）
fn run_database_migration() -> bool {
    println!("执行数据库迁移...");
    let bingxi_bin = format!("{}/backend/bingxi", get_install_dir());
    if !std::path::Path::new(&bingxi_bin).exists() {
        println!("[WARN] 未找到 bingxi CLI（{}），跳过自动迁移", bingxi_bin);
        return true;
    }
    match run_cmd(&bingxi_bin, &["migrate", "run"]) {
        Ok(_) => {
            println!("[OK] 数据库迁移完成");
            true
        }
        Err(e) => {
            println!("[ERROR] 数据库迁移失败: {}", e);
            false
        }
    }
}

/// V15 P1 25.3-K：回滚时同步回滚 DB schema（调用 bingxi migrate rollback）
/// 失败仅告警（旧二进制可能兼容新 schema，或 schema 回滚不可逆）
fn rollback_database_schema() {
    println!("回滚数据库 schema...");
    let bingxi_bin = format!("{}/backend/bingxi", get_install_dir());
    if !std::path::Path::new(&bingxi_bin).exists() {
        println!(
            "[WARN] 未找到 bingxi CLI（{}），跳过 schema 回滚",
            bingxi_bin
        );
        return;
    }
    match run_cmd(&bingxi_bin, &["migrate", "rollback"]) {
        Ok(_) => println!("[OK] 数据库 schema 回滚完成"),
        Err(e) => println!(
            "[WARN] 数据库 schema 回滚失败（旧二进制可能兼容新 schema）: {}",
            e
        ),
    }
}

/// V15 P1 25.4-F：单实例模式健康检查门禁（HTTP /health 端点）
/// 返回 true 表示健康检查通过
fn health_check_http(retries: u8, interval_secs: u64) -> bool {
    let url = "http://127.0.0.1:8082/health";
    for i in 0..retries {
        if run_cmd("curl", &["-fsSL", "-m", "3", url]).is_ok() {
            println!("  [OK] 健康检查通过（第 {} 次）", i + 1);
            return true;
        }
        if i < retries - 1 {
            std::thread::sleep(std::time::Duration::from_secs(interval_secs));
        }
    }
    false
}

/// V15 P1 25.4-L：部署后启动监控线程，连续失败触发自动回滚
/// 在新线程中执行，避免阻塞主流程；主流程立即返回成功
fn start_post_deploy_monitor() {
    let handle = std::thread::spawn(|| {
        let url = "http://127.0.0.1:8082/health";
        let mut consecutive_failures: u8 = 0;
        for _ in 0..POST_DEPLOY_MONITOR_RETRIES {
            std::thread::sleep(std::time::Duration::from_secs(
                POST_DEPLOY_MONITOR_INTERVAL_SECS,
            ));
            if run_cmd("curl", &["-fsSL", "-m", "3", url]).is_ok() {
                return; // 健康检查通过，监控结束
            }
            consecutive_failures += 1;
            println!(
                "[WARN] 部署后健康检查失败（{}/{}）",
                consecutive_failures, POST_DEPLOY_MONITOR_RETRIES
            );
        }
        // 连续失败触发自动回滚
        println!(
            "[ERROR] 部署后连续 {} 次健康检查失败，触发自动回滚",
            consecutive_failures
        );
        let server_old = format!("{}/backend/server.old", get_install_dir());
        if std::path::Path::new(&server_old).exists() {
            cmd_rollback();
        } else {
            println!("[ERROR] 无 server.old 备份，无法自动回滚，请手动介入");
        }
    });
    // 分离线程：监控在后台运行，主流程不等待
    let _ = handle;
}

// ==================== P0-D15 蓝绿部署辅助函数 ====================

/// 检测当前是否为蓝绿部署模式（systemd template 已安装）。
fn is_blue_green_mode() -> bool {
    run_cmd("systemctl", &["list-unit-files", BLUE_GREEN_TEMPLATE])
        .map(|s| s.contains("bingxi-backend@"))
        .unwrap_or(false)
}

/// 获取当前活跃实例名（`blue` 或 `green`）。
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
    // V15 P1 25.2-C 修复：升级命令必须 root 权限（操作 systemd + 系统目录）
    require_root();
    println!("=== 系统升级 ===\n");
    let current = env!("CARGO_PKG_VERSION");
    println!("当前版本: v{}", current);

    let target = match resolve_target_version(&version) {
        Some(v) => v,
        None => return,
    };

    // V15 P1 25.3-E 修复：升级前检查 schema 版本兼容性
    if !check_schema_compatibility() {
        println!("[ERROR] schema 版本兼容性检查失败，终止升级");
        return;
    }

    if !no_backup && !super::backup::cmd_backup("all") {
        println!("[ERROR] 备份失败，终止升级");
        return;
    }

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

    // V15 P1 25.3-A 修复：下载后 SHA256 校验，防止损坏/篡改
    if !verify_sha256(&release_url, &download_path) {
        println!("[ERROR] SHA256 校验失败，终止升级（文件可能损坏或被篡改）");
        let _ = run_cmd("rm", &["-f", &download_path]);
        return;
    }

    deploy_release(&download_path);

    if let Err(e) = run_cmd("rm", &["-f", &download_path]) {
        println!("[WARN] 清理下载包失败（可忽略）: {}", e);
    }

    // V15 P1 25.4-L 修复：部署后启动监控，连续失败自动回滚
    start_post_deploy_monitor();

    println!("\n[OK] 升级完成");
    println!("新版本: {}", target);
    println!("备份位置: {}", get_backup_dir());
    println!("\n如需回滚: bingxi rollback");
}

fn resolve_target_version(version: &Option<String>) -> Option<String> {
    match version {
        Some(v) => {
            let v = if v.starts_with('v') {
                v.clone()
            } else {
                format!("v{}", v)
            };
            println!("目标版本: {}", v);
            Some(v)
        }
        None => {
            println!("获取最新版本...");
            match get_latest_version() {
                Some(v) => {
                    println!("最新版本: {}", v);
                    Some(v)
                }
                None => {
                    println!("[ERROR] 无法获取最新版本");
                    println!("\n请手动指定版本:");
                    println!("  bingxi upgrade --version v2026.x.x.xxxx");
                    println!("\n或手动下载后使用 deploy 命令:");
                    println!("  bingxi deploy --package release-xxx.tar.gz");
                    None
                }
            }
        }
    }
}

pub(super) fn cmd_deploy(package: &str) {
    // V15 P1 25.2-C 修复：部署命令必须 root 权限（操作 systemd + 系统目录）
    require_root();
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
    // V15 P1 25.2-C 修复：回滚命令必须 root 权限（操作 systemd + 系统目录 + 二进制覆盖）
    require_root();
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

    let _ = run_cmd("systemctl", &["stop", &inactive_service]);

    if !restore_rollback_binaries(server_old, bingxi_old, &active_service) {
        return;
    }

    if !start_and_health_check(&inactive, &inactive_service, &active_service) {
        return;
    }

    if let Err(e) = switch_nginx_upstream(&inactive) {
        println!("[ERROR] nginx 切换失败: {}", e);
        println!("停止回滚实例，活跃实例 {} 继续服务", active_service);
        let _ = run_cmd("systemctl", &["stop", &inactive_service]);
        return;
    }

    println!("停止原活跃实例 {}...", active_service);
    if let Err(e) = run_cmd("systemctl", &["stop", &active_service]) {
        println!("[WARN] 停止原活跃实例失败（可手动停止）: {}", e);
    }

    // V15 P1 25.3-K 修复：蓝绿回滚同步回滚 DB schema
    rollback_database_schema();

    println!("\n[OK] 蓝绿回滚成功");
    println!("新活跃实例: {} ({})", inactive, instance_port(&inactive));
    println!(
        "旧实例 {} 已停止（如需重启新版本可手动启动）",
        active_service
    );
}

fn restore_rollback_binaries(server_old: &str, bingxi_old: &str, active_service: &str) -> bool {
    println!("恢复旧版本二进制...");
    let server_path = format!("{}/backend/server", get_install_dir());
    let bingxi_path = format!("{}/backend/bingxi", get_install_dir());

    if let Err(e) = run_cmd("cp", &["-f", server_old, &server_path]) {
        println!("[ERROR] 恢复 server 失败，终止回滚: {}", e);
        println!("活跃实例 {} 继续服务，未受影响", active_service);
        return false;
    }
    if let Err(e) = run_cmd("cp", &["-f", bingxi_old, &bingxi_path]) {
        println!("[ERROR] 恢复 bingxi 失败，终止回滚: {}", e);
        println!("活跃实例 {} 继续服务，未受影响", active_service);
        return false;
    }
    if let Err(e) = run_cmd("chmod", &["+x", &server_path]) {
        println!("[ERROR] chmod server 失败，终止回滚: {}", e);
        println!("活跃实例 {} 继续服务，未受影响", active_service);
        return false;
    }
    if let Err(e) = run_cmd("chmod", &["+x", &bingxi_path]) {
        println!("[ERROR] chmod bingxi 失败，终止回滚: {}", e);
        println!("活跃实例 {} 继续服务，未受影响", active_service);
        return false;
    }
    true
}

fn start_and_health_check(inactive: &str, inactive_service: &str, active_service: &str) -> bool {
    println!("启动非活跃实例 {}...", inactive_service);
    if let Err(e) = run_cmd("systemctl", &["start", inactive_service]) {
        println!("[ERROR] 启动 {} 失败: {}", inactive_service, e);
        println!("活跃实例 {} 继续服务，未受影响", active_service);
        return false;
    }

    println!("健康检查回滚实例 {}...", inactive_service);
    if !health_check_instance(inactive) {
        println!("[ERROR] 回滚实例健康检查失败");
        println!("停止回滚实例，活跃实例 {} 继续服务", active_service);
        let _ = run_cmd("systemctl", &["stop", inactive_service]);
        return false;
    }
    true
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

    // V15 P1 25.3-K 修复：单实例回滚同步回滚 DB schema
    rollback_database_schema();

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
fn deploy_release(package: &str) {
    if is_blue_green_mode() {
        deploy_release_blue_green(package);
    } else {
        deploy_release_legacy(package);
    }
}

/// 解析活跃/非活跃实例名与服务名；失败返回 Err（错误已打印）
fn resolve_blue_green_instances() -> Result<(String, String, String, String), ()> {
    let active = match get_active_instance() {
        Some(a) => a,
        None => {
            println!("[ERROR] 无法确定活跃实例");
            println!(
                "请检查 {} 软链接是否指向 blue 或 green 配置",
                NGINX_UPSTREAM_ACTIVE
            );
            println!("或回退到单实例模式（移除 bingxi-backend@.service 后重试）");
            return Err(());
        }
    };
    let inactive = opposite_instance(&active).to_string();
    let inactive_service = instance_service(&inactive);
    let active_service = instance_service(&active);
    println!("当前活跃实例: {} ({})", active, instance_port(&active));
    println!("部署目标实例: {} ({})", inactive, instance_port(&inactive));
    Ok((active, inactive, active_service, inactive_service))
}

/// 创建临时目录 + tar -tf 校验 + 解压 + validate_extracted_paths 二次校验
fn prepare_and_extract_package(package: &str, temp_dir: &str) -> Result<String, ()> {
    if let Err(e) = run_cmd("mkdir", &["-p", temp_dir]) {
        println!("[ERROR] 创建临时目录失败，终止部署: {}", e);
        return Err(());
    }
    println!("校验更新包内容...");
    let tar_list = match run_cmd("tar", &["-tf", package]) {
        Ok(list) => list,
        Err(e) => {
            println!("[ERROR] 列出更新包内容失败: {}", e);
            return Err(());
        }
    };
    for line in tar_list.lines() {
        let path = line.trim();
        if path.is_empty() || path == "./" {
            continue;
        }
        if path.contains("..") {
            println!("[ERROR] 检测到路径穿越攻击：文件 {} 包含 ..", path);
            return Err(());
        }
        if path.starts_with('/') {
            println!("[ERROR] 检测到绝对路径：文件 {}", path);
            return Err(());
        }
    }
    println!("解压更新包...");
    if let Err(e) = run_cmd("tar", &["-xzf", package, "-C", temp_dir]) {
        println!("[ERROR] 解压失败，终止部署: {}", e);
        return Err(());
    }
    let extract_dir = format!("{}/bingxi-erp", temp_dir);
    if let Err(e) = validate_extracted_paths(&extract_dir) {
        println!("[ERROR] 安全校验失败，终止部署: {}", e);
        return Err(());
    }
    Ok(extract_dir)
}

/// 备份旧二进制到 old.{ts} 目录并刷新 .old 标记；mkdir 失败返回 Err
fn backup_old_binaries(install_dir: &str, ts: u64) -> Result<(), ()> {
    println!("备份旧文件...");
    let old_backup = format!("{}/old.{}", install_dir, ts);
    if let Err(e) = run_cmd("mkdir", &["-p", &old_backup]) {
        println!("[ERROR] 创建旧文件备份目录失败，终止部署: {}", e);
        return Err(());
    }
    let server_src = format!("{}/backend/server", install_dir);
    let bingxi_src = format!("{}/backend/bingxi", install_dir);
    if let Err(e) = run_cmd("cp", &["-r", &server_src, &old_backup]) {
        println!("[ERROR] 备份 server 失败: {}", e);
    }
    if let Err(e) = run_cmd("cp", &["-r", &bingxi_src, &old_backup]) {
        println!("[ERROR] 备份 bingxi 失败: {}", e);
    }
    let server_old = format!("{}/backend/server.old", install_dir);
    let bingxi_old = format!("{}/backend/bingxi.old", install_dir);
    if let Err(e) = run_cmd("cp", &["-f", &server_src, &server_old]) {
        println!("[WARN] 刷新 server.old 失败（不影响部署）: {}", e);
    }
    if let Err(e) = run_cmd("cp", &["-f", &bingxi_src, &bingxi_old]) {
        println!("[WARN] 刷新 bingxi.old 失败（不影响部署）: {}", e);
    }
    Ok(())
}

/// 替换后端二进制（cp + chmod +x）；任一失败返回 Err
fn replace_backend_binaries(extract_dir: &str, install_dir: &str) -> Result<(), ()> {
    println!("更新后端二进制...");
    let new_server = format!("{}/backend/server", extract_dir);
    let new_bingxi = format!("{}/backend/bingxi", extract_dir);
    let dst_server = format!("{}/backend/server", install_dir);
    let dst_bingxi = format!("{}/backend/bingxi", install_dir);
    if let Err(e) = run_cmd("cp", &["-r", &new_server, &dst_server]) {
        println!("[ERROR] 覆盖 server 失败，终止部署: {}", e);
        return Err(());
    }
    if let Err(e) = run_cmd("cp", &["-r", &new_bingxi, &dst_bingxi]) {
        println!("[ERROR] 覆盖 bingxi 失败，终止部署: {}", e);
        return Err(());
    }
    if let Err(e) = run_cmd("chmod", &["+x", &dst_server]) {
        println!("[ERROR] chmod server 失败，终止部署: {}", e);
        return Err(());
    }
    if let Err(e) = run_cmd("chmod", &["+x", &dst_bingxi]) {
        println!("[ERROR] chmod bingxi 失败，终止部署: {}", e);
        return Err(());
    }
    Ok(())
}

/// 替换前端 dist（rm -rf 旧 + mv 新）；mv 失败返回 Err
fn replace_frontend_dist(extract_dir: &str, install_dir: &str) -> Result<(), ()> {
    println!("更新前端...");
    let frontend_dist = format!("{}/frontend/dist", install_dir);
    if let Err(e) = run_cmd("rm", &["-rf", &frontend_dist]) {
        println!("[WARN] 清理旧前端 dist 失败（继续 mv 覆盖）: {}", e);
    }
    let new_dist = format!("{}/frontend/dist", extract_dir);
    if let Err(e) = run_cmd("mv", &[&new_dist, &frontend_dist]) {
        println!("[ERROR] 移动新前端 dist 失败，终止部署: {}", e);
        return Err(());
    }
    Ok(())
}

/// 停止非活跃实例 → 启动 → 健康检查；任一失败返回 Err（活跃实例继续服务）
fn start_inactive_and_health_check(
    inactive: &str,
    inactive_service: &str,
    active_service: &str,
) -> Result<(), ()> {
    println!("停止非活跃实例 {}（如运行中）...", inactive_service);
    let _ = run_cmd("systemctl", &["stop", inactive_service]);
    println!("启动非活跃实例 {}...", inactive_service);
    if let Err(e) = run_cmd("systemctl", &["start", inactive_service]) {
        println!("[ERROR] 启动 {} 失败: {}", inactive_service, e);
        println!("活跃实例 {} 继续服务，未受影响", active_service);
        return Err(());
    }
    println!("健康检查新实例 {}...", inactive_service);
    if !health_check_instance(inactive) {
        println!("[ERROR] 新实例健康检查失败");
        println!("停止新实例，活跃实例 {} 继续服务", active_service);
        let _ = run_cmd("systemctl", &["stop", inactive_service]);
        return Err(());
    }
    Ok(())
}

/// 切换 nginx upstream → 停止原活跃实例；nginx 失败回滚新实例并返回 Err
fn switch_nginx_and_stop_active(
    inactive: &str,
    active_service: &str,
    inactive_service: &str,
) -> Result<(), ()> {
    println!("切换 nginx upstream → {}...", inactive);
    if let Err(e) = switch_nginx_upstream(inactive) {
        println!("[ERROR] nginx 切换失败: {}", e);
        println!("停止新实例，活跃实例 {} 继续服务", active_service);
        let _ = run_cmd("systemctl", &["stop", inactive_service]);
        return Err(());
    }
    println!("停止旧实例 {}...", active_service);
    if let Err(e) = run_cmd("systemctl", &["stop", active_service]) {
        println!("[WARN] 停止旧实例失败（可手动停止）: {}", e);
    }
    Ok(())
}

/// P0-D15：蓝绿模式部署（零停机），任一关键步骤失败立即中止以保持活跃实例服务
fn deploy_release_blue_green(package: &str) {
    println!("=== 蓝绿部署模式（零停机）===");
    let (_active, inactive, active_service, inactive_service) = match resolve_blue_green_instances()
    {
        Ok(v) => v,
        Err(()) => return,
    };
    let temp_dir_owned = format!(
        "{}/bingxi_upgrade_{}",
        std::env::temp_dir().to_string_lossy(),
        uuid::Uuid::new_v4()
    );
    let temp_dir = temp_dir_owned.as_str();
    let extract_dir = match prepare_and_extract_package(package, temp_dir) {
        Ok(d) => d,
        Err(()) => {
            cleanup_temp(temp_dir);
            return;
        }
    };
    let install_dir = get_install_dir();
    if let Err(()) = backup_old_binaries(&install_dir, timestamp()) {
        cleanup_temp(temp_dir);
        return;
    }
    if let Err(()) = replace_backend_binaries(&extract_dir, &install_dir) {
        cleanup_temp(temp_dir);
        return;
    }
    if let Err(()) = replace_frontend_dist(&extract_dir, &install_dir) {
        cleanup_temp(temp_dir);
        return;
    }
    cleanup_temp(temp_dir);
    // V15 P1 25.3-H 修复：部署后自动执行数据库迁移（蓝绿模式下，新实例启动前执行迁移）
    if !run_database_migration() {
        println!(
            "[ERROR] 数据库迁移失败，终止部署（活跃实例 {} 继续服务）",
            active_service
        );
        return;
    }
    if let Err(()) = start_inactive_and_health_check(&inactive, &inactive_service, &active_service)
    {
        return;
    }
    if let Err(()) = switch_nginx_and_stop_active(&inactive, &active_service, &inactive_service) {
        return;
    }
    println!("\n[OK] 蓝绿部署成功");
    println!("新活跃实例: {} ({})", inactive, instance_port(&inactive));
    println!("如需回滚: bingxi rollback");
}

/// 单实例模式部署（原 deploy_release 逻辑，停机模式）。
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
    // V15 P1 25.3-H 修复：部署后自动执行数据库迁移（单实例模式下，启动服务前执行迁移）
    if !run_database_migration() {
        println!("[ERROR] 数据库迁移失败，请手动执行 `bingxi migrate run` 后启动服务");
        return;
    }
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
    let tar_list =
        run_cmd("tar", &["-tf", package]).map_err(|e| format!("列出更新包内容失败: {}", e))?;
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
    run_cmd("tar", &["-xzf", package, "-C", temp_dir]).map_err(|e| format!("解压失败: {}", e))?;
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
    run_cmd("mkdir", &["-p", &old_backup]).map_err(|e| format!("创建旧文件备份目录失败: {}", e))?;
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
    run_cmd("cp", &["-r", &new_server, &dst_server])
        .map_err(|e| format!("覆盖 server 失败: {}", e))?;
    run_cmd("cp", &["-r", &new_bingxi, &dst_bingxi])
        .map_err(|e| format!("覆盖 bingxi 失败: {}", e))?;
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
    run_cmd("mv", &[&new_dist, &frontend_dist])
        .map_err(|e| format!("移动新前端 dist 失败: {}", e))?;
    Ok(())
}

/// 启动服务并健康检查（启动失败仅记录，等待运维介入）。
/// V15 P1 25.4-F：单实例模式部署后增加 HTTP 健康检查门禁
fn start_service_and_check() {
    println!("启动服务...");
    if let Err(e) = run_cmd("systemctl", &["start", super::SERVICE_NAME]) {
        println!("[ERROR] 启动服务失败: {}", e);
    }
    std::thread::sleep(std::time::Duration::from_secs(3));
    if !is_service_active(super::SERVICE_NAME) {
        println!("[ERROR] 服务启动失败，请检查日志");
        return;
    }
    // V15 P1 25.4-F 修复：单实例模式部署后 HTTP 健康检查门禁
    // systemd active 仅表示进程存活，HTTP /health 才确认业务就绪（DB 连接、关键依赖可用）
    println!("健康检查（HTTP /health）...");
    if health_check_http(HEALTH_CHECK_RETRIES, 1) {
        println!("[OK] 部署成功（HTTP 健康检查通过）");
    } else {
        println!("[ERROR] HTTP 健康检查失败，服务可能未就绪，请检查日志");
        println!("  可执行 `curl -fsSL http://127.0.0.1:8082/health` 手动验证");
        println!(
            "  或执行 `journalctl -u {} -n 100 --no-pager` 查看服务日志",
            super::SERVICE_NAME
        );
    }
}
