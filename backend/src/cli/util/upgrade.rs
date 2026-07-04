//! 升级 / 部署 / 回滚子命令实现：Upgrade / Deploy / Rollback
//!
//! 同时承载两个内部辅助函数 `get_latest_version`、`deploy_release`，
//! 它们由本文件内的 `cmd_upgrade` / `cmd_deploy` 使用。
//!
//! 批次 92 P3-8：原 20 处 `let _ = run_cmd(...)` 静默吞错已全部改为
//! `if let Err(e) = ... { println!("[ERROR]/[WARN] ...") }` 模式：
//! - 关键路径（stop/start/mv/cp/chmod/mkdir）失败记录 [ERROR]
//! - 清理路径（rm -rf temp）失败记录 [WARN]

use super::{
    build_release_url, download_with_mirrors, fetch_with_mirrors, get_backup_dir, get_install_dir,
    is_service_active, parse_json_field, run_cmd, timestamp, GITHUB_REPO,
};

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
        super::backup::cmd_backup("all");
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
    if let Err(e) = run_cmd("mv", &[&server_old, &server_path]) {
        println!("[ERROR] 恢复 server 失败，终止回滚: {}", e);
        return;
    }
    if let Err(e) = run_cmd("mv", &[&bingxi_old, &bingxi_path]) {
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

/// 部署发布包
fn deploy_release(package: &str) {
    println!("停止服务...");
    if let Err(e) = run_cmd("systemctl", &["stop", super::SERVICE_NAME]) {
        println!("[ERROR] 停止服务失败（继续部署）: {}", e);
    }
    std::thread::sleep(std::time::Duration::from_secs(2));

    println!("解压更新包...");
    if let Err(e) = run_cmd("tar", &["-xzf", package, "-C", "/tmp"]) {
        println!("[ERROR] 解压失败，终止部署: {}", e);
        return;
    }

    let extract_dir = "/tmp/bingxi-erp";
    let install_dir = get_install_dir();

    // 备份旧文件
    println!("备份旧文件...");
    let ts = timestamp();
    let old_backup = format!("{}/old.{}", install_dir, ts);
    if let Err(e) = run_cmd("mkdir", &["-p", &old_backup]) {
        println!("[ERROR] 创建旧文件备份目录失败，终止部署: {}", e);
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

    // 更新后端
    println!("更新后端...");
    let new_server = format!("{}/backend/server", extract_dir);
    let new_bingxi = format!("{}/backend/bingxi", extract_dir);
    let dst_server = format!("{}/backend/server", install_dir);
    let dst_bingxi = format!("{}/backend/bingxi", install_dir);

    // 批次 95 P3-13 修复：覆盖二进制 + chmod 为关键路径，失败立即中止部署
    // （避免启动残缺版本；服务保持停止状态等待运维介入）
    if let Err(e) = run_cmd("cp", &["-r", &new_server, &dst_server]) {
        println!("[ERROR] 覆盖 server 失败，终止部署: {}", e);
        return;
    }
    if let Err(e) = run_cmd("cp", &["-r", &new_bingxi, &dst_bingxi]) {
        println!("[ERROR] 覆盖 bingxi 失败，终止部署: {}", e);
        return;
    }
    if let Err(e) = run_cmd("chmod", &["+x", &dst_server]) {
        println!("[ERROR] chmod server 失败，终止部署: {}", e);
        return;
    }
    if let Err(e) = run_cmd("chmod", &["+x", &dst_bingxi]) {
        println!("[ERROR] chmod bingxi 失败，终止部署: {}", e);
        return;
    }

    // 更新前端
    println!("更新前端...");
    let frontend_dist = format!("{}/frontend/dist", install_dir);
    if let Err(e) = run_cmd("rm", &["-rf", &frontend_dist]) {
        println!("[WARN] 清理旧前端 dist 失败（继续 mv 覆盖）: {}", e);
    }
    let new_dist = format!("{}/frontend/dist", extract_dir);
    // 批次 95 P3-13 修复：移动前端 dist 为关键路径，失败立即中止部署（避免前端缺失上线）
    if let Err(e) = run_cmd("mv", &[&new_dist, &frontend_dist]) {
        println!("[ERROR] 移动新前端 dist 失败，终止部署: {}", e);
        return;
    }

    // 清理解压目录（非关键路径）
    if let Err(e) = run_cmd("rm", &["-rf", extract_dir]) {
        println!("[WARN] 清理解压目录失败（可忽略）: {}", e);
    }

    // 启动
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
