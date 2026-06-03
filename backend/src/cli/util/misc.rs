//! 杂项子命令实现：Clean / Config / Info

use super::{get_install_dir, get_log_dir, get_backup_dir, is_service_active, run_cmd, SERVICE_NAME};

pub(super) fn cmd_clean(clean_type: &str) {
    println!("=== 清理系统 ===\n");

    if clean_type == "logs" || clean_type == "all" {
        println!("清理旧日志 (30天前)...");
        let _ = run_cmd(
            "find",
            &[
                &get_log_dir(),
                "-name",
                "*.log*",
                "-mtime",
                "+30",
                "-delete",
            ],
        );
        println!("[OK] 日志清理完成");
    }

    if clean_type == "backups" || clean_type == "all" {
        println!("清理旧备份 (90天前)...");
        let _ = run_cmd(
            "find",
            &[
                &get_backup_dir(),
                "-name",
                "backup_*",
                "-mtime",
                "+90",
                "-delete",
            ],
        );
        println!("[OK] 备份清理完成");
    }

    if clean_type == "temp" || clean_type == "all" {
        println!("清理临时文件...");
        let _ = run_cmd("rm", &["-rf", "/tmp/release-*.tar.gz"]);
        let _ = run_cmd("rm", &["-rf", "/tmp/bingxi-erp"]);
        println!("[OK] 临时文件清理完成");
    }

    println!("\n[OK] 清理完成");
}

pub(super) fn cmd_config() {
    println!("=== 系统配置 ===\n");

    // 后端配置
    let config_file = format!("{}/backend/config.yaml", get_install_dir());
    println!("--- {} ---", config_file);
    match std::fs::read_to_string(&config_file) {
        Ok(content) => println!("{}", content),
        Err(_) => println!("[WARN] 文件不存在"),
    }

    // 环境变量 (隐藏敏感信息)
    println!("\n--- 环境变量 ---");
    let env_file = "/etc/bingxi/.env";
    match std::fs::read_to_string(env_file) {
        Ok(content) => {
            for line in content.lines() {
                if line.contains("PASSWORD") || line.contains("SECRET") || line.contains("KEY") {
                    let parts: Vec<&str> = line.splitn(2, '=').collect();
                    if parts.len() == 2 {
                        println!("{}=***", parts[0]);
                    }
                } else {
                    println!("{}", line);
                }
            }
        }
        Err(_) => println!("[WARN] 文件不存在"),
    }

    // 服务配置
    println!("\n--- 服务配置 ---");
    let service_file = format!("/etc/systemd/system/{}.service", SERVICE_NAME);
    match std::fs::read_to_string(&service_file) {
        Ok(content) => println!("{}", content),
        Err(_) => println!("[WARN] 文件不存在"),
    }
}

pub(super) fn cmd_info() {
    println!("=== Bingxi ERP 系统信息 ===\n");

    println!("CLI 版本: v{}", env!("CARGO_PKG_VERSION"));
    println!("安装目录: {}", get_install_dir());
    println!(
        "服务状态: {}",
        if is_service_active(SERVICE_NAME) {
            "运行中"
        } else {
            "已停止"
        }
    );

    println!("\n--- 系统信息 ---");
    if let Ok(uname) = run_cmd("uname", &["-a"]) {
        println!("{}", uname);
    }

    println!("\n--- 磁盘使用 ---");
    if let Ok(df) = run_cmd("df", &["-h", &get_install_dir()]) {
        println!("{}", df);
    }

    println!("\n--- 内存使用 ---");
    if let Ok(free) = run_cmd("free", &["-h"]) {
        println!("{}", free);
    }

    println!("\n--- 服务运行时间 ---");
    match run_cmd(
        "systemctl",
        &["show", SERVICE_NAME, "--property=ActiveEnterTimestamp"],
    ) {
        Ok(ts) => println!("{}", ts),
        Err(_) => println!("无法获取"),
    }
}
