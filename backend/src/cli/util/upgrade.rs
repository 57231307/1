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

// 批次 322 v9 复审低危修复：路径校验逻辑已抽取到共享模块 `utils::path_validator`，
// 此处复用，避免与 backup.rs 重复维护。测试覆盖见 path_validator 模块。
use crate::utils::path_validator::validate_extracted_paths;

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

// 批次 322 v9 复审低危修复：`validate_extracted_path` 已抽取到共享模块
// `utils::path_validator::validate_extracted_paths`，此处不再重复维护。

/// 部署发布包
fn deploy_release(package: &str) {
    println!("停止服务...");
    if let Err(e) = run_cmd("systemctl", &["stop", super::SERVICE_NAME]) {
        println!("[ERROR] 停止服务失败（继续部署）: {}", e);
    }
    std::thread::sleep(std::time::Duration::from_secs(2));

    // H-1 修复（v9 复审）：对齐 backup.rs M4 方案 — UUID 随机目录 + 先 tar -tf 校验再解压 + 二次校验
    // 原方案解压到固定路径 /tmp，存在符号链接竞争（TOCTOU）和校验范围不足问题：
    // 1. 固定路径 /tmp/bingxi-erp 可被攻击者预先创建符号链接指向 /etc
    // 2. 校验仅覆盖 /tmp/bingxi-erp 子目录，tar 可解压出 /tmp 其他文件绕过校验
    // 3. 先解压后校验，恶意文件在校验前已写入磁盘
    let temp_dir_owned = format!(
        "{}/bingxi_upgrade_{}",
        std::env::temp_dir().to_string_lossy(),
        uuid::Uuid::new_v4()
    );
    let temp_dir = temp_dir_owned.as_str();

    // 创建随机临时目录（关键路径）
    if let Err(e) = run_cmd("mkdir", &["-p", temp_dir]) {
        println!("[ERROR] 创建临时目录失败，终止部署: {}", e);
        return;
    }

    // 1. 先列出 tar 内容并校验路径，防止恶意文件在校验前已写入磁盘
    println!("校验更新包内容...");
    let tar_list = match run_cmd("tar", &["-tf", package]) {
        Ok(list) => list,
        Err(e) => {
            println!("[ERROR] 列出更新包内容失败: {}", e);
            if let Err(e) = run_cmd("rm", &["-rf", temp_dir]) {
                println!("[WARN] 清理临时目录失败（可忽略）: {}", e);
            }
            return;
        }
    };

    // 解压前校验：检查每个路径不包含 .. 和不以 / 开头（防止 Tar Slip 路径穿越）
    for line in tar_list.lines() {
        let path = line.trim();
        if path.is_empty() || path == "./" {
            continue;
        }
        if path.contains("..") {
            println!("[ERROR] 检测到路径穿越攻击：文件 {} 包含 ..", path);
            if let Err(e) = run_cmd("rm", &["-rf", temp_dir]) {
                println!("[WARN] 清理临时目录失败（可忽略）: {}", e);
            }
            return;
        }
        if path.starts_with('/') {
            println!("[ERROR] 检测到绝对路径：文件 {}", path);
            if let Err(e) = run_cmd("rm", &["-rf", temp_dir]) {
                println!("[WARN] 清理临时目录失败（可忽略）: {}", e);
            }
            return;
        }
    }

    // 2. 解压到随机临时目录
    println!("解压更新包...");
    if let Err(e) = run_cmd("tar", &["-xzf", package, "-C", temp_dir]) {
        println!("[ERROR] 解压失败，终止部署: {}", e);
        if let Err(e) = run_cmd("rm", &["-rf", temp_dir]) {
            println!("[WARN] 清理临时目录失败（可忽略）: {}", e);
        }
        return;
    }

    // 3. 解压后二次校验（canonicalize 解析符号链接），双重防护
    // 批次 322 v9 复审低危修复：改用共享模块 utils::path_validator::validate_extracted_paths
    let extract_dir = format!("{}/bingxi-erp", temp_dir);
    if let Err(e) = validate_extracted_paths(&extract_dir) {
        println!("[ERROR] 安全校验失败，终止部署: {}", e);
        if let Err(e) = run_cmd("rm", &["-rf", temp_dir]) {
            println!("[WARN] 清理临时目录失败（可忽略）: {}", e);
        }
        return;
    }
    let install_dir = get_install_dir();

    // 备份旧文件
    println!("备份旧文件...");
    let ts = timestamp();
    let old_backup = format!("{}/old.{}", install_dir, ts);
    if let Err(e) = run_cmd("mkdir", &["-p", &old_backup]) {
        println!("[ERROR] 创建旧文件备份目录失败，终止部署: {}", e);
        if let Err(e) = run_cmd("rm", &["-rf", temp_dir]) {
            println!("[WARN] 清理临时目录失败（可忽略）: {}", e);
        }
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
        if let Err(e) = run_cmd("rm", &["-rf", temp_dir]) {
            println!("[WARN] 清理临时目录失败（可忽略）: {}", e);
        }
        return;
    }
    if let Err(e) = run_cmd("cp", &["-r", &new_bingxi, &dst_bingxi]) {
        println!("[ERROR] 覆盖 bingxi 失败，终止部署: {}", e);
        if let Err(e) = run_cmd("rm", &["-rf", temp_dir]) {
            println!("[WARN] 清理临时目录失败（可忽略）: {}", e);
        }
        return;
    }
    if let Err(e) = run_cmd("chmod", &["+x", &dst_server]) {
        println!("[ERROR] chmod server 失败，终止部署: {}", e);
        if let Err(e) = run_cmd("rm", &["-rf", temp_dir]) {
            println!("[WARN] 清理临时目录失败（可忽略）: {}", e);
        }
        return;
    }
    if let Err(e) = run_cmd("chmod", &["+x", &dst_bingxi]) {
        println!("[ERROR] chmod bingxi 失败，终止部署: {}", e);
        if let Err(e) = run_cmd("rm", &["-rf", temp_dir]) {
            println!("[WARN] 清理临时目录失败（可忽略）: {}", e);
        }
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
        if let Err(e) = run_cmd("rm", &["-rf", temp_dir]) {
            println!("[WARN] 清理临时目录失败（可忽略）: {}", e);
        }
        return;
    }

    // 清理临时目录（非关键路径）
    if let Err(e) = run_cmd("rm", &["-rf", temp_dir]) {
        println!("[WARN] 清理临时目录失败（可忽略）: {}", e);
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
