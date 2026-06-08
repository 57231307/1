//! 服务管理子命令实现：Status / Start / Stop / Restart / Logs / Health

use std::process::Command;

use super::{
    get_install_dir, get_log_dir, is_service_active, require_env, run_cmd, status_icon,
    SERVICE_NAME,
};

pub(super) fn cmd_status() {
    println!("=== Bingxi ERP 服务状态 ===\n");

    // 后端服务
    let backend_ok = is_service_active(SERVICE_NAME);
    println!("{} 后端服务 ({})", status_icon(backend_ok), SERVICE_NAME);

    // Nginx
    let nginx_ok = is_service_active("nginx");
    println!("{} Nginx 服务", status_icon(nginx_ok));

    // 端口监听
    println!("\n--- 端口监听 ---");
    if let Ok(ss) = run_cmd("ss", &["-tlnp"]) {
        println!(
            "{} 端口 8082 (后端)",
            if ss.contains(":8082") {
                "[OK]"
            } else {
                "[STOPPED]"
            }
        );
        println!(
            "{} 端口 80 (HTTP)",
            if ss.contains(":80 ") {
                "[OK]"
            } else {
                "[STOPPED]"
            }
        );
    }

    // 进程信息
    println!("\n--- 进程信息 ---");
    if let Ok(pid) = run_cmd("pgrep", &["-f", "server"]) {
        if !pid.is_empty() {
            println!("[OK] 后端进程 PID: {}", pid.lines().next().unwrap_or("?"));
        } else {
            println!("[STOPPED] 后端进程未运行");
        }
    }
}

pub(super) fn cmd_start() {
    println!("=== 启动服务 ===\n");

    println!("启动后端...");
    match run_cmd("systemctl", &["start", SERVICE_NAME]) {
        Ok(_) => println!("[OK] 后端已启动"),
        Err(e) => println!("[ERROR] 启动失败: {}", e),
    }

    println!("重载 Nginx...");
    match run_cmd("systemctl", &["reload", "nginx"]) {
        Ok(_) => println!("[OK] Nginx 已重载"),
        Err(e) => println!("[WARN] Nginx 重载失败: {}", e),
    }

    std::thread::sleep(std::time::Duration::from_secs(2));

    if is_service_active(SERVICE_NAME) {
        println!("\n[OK] 服务启动成功");
    } else {
        println!("\n[ERROR] 服务启动失败，查看日志: bingxi logs");
    }
}

pub(super) fn cmd_stop() {
    println!("=== 停止服务 ===\n");

    match run_cmd("systemctl", &["stop", SERVICE_NAME]) {
        Ok(_) => println!("[OK] 后端已停止"),
        Err(e) => println!("[ERROR] 停止失败: {}", e),
    }
}

pub(super) fn cmd_restart() {
    println!("=== 重启服务 ===\n");

    println!("停止服务...");
    let _ = run_cmd("systemctl", &["stop", SERVICE_NAME]);
    std::thread::sleep(std::time::Duration::from_secs(1));

    println!("启动服务...");
    match run_cmd("systemctl", &["start", SERVICE_NAME]) {
        Ok(_) => println!("[OK] 服务已重启"),
        Err(e) => println!("[ERROR] 重启失败: {}", e),
    }

    let _ = run_cmd("systemctl", &["reload", "nginx"]);

    std::thread::sleep(std::time::Duration::from_secs(2));

    if is_service_active(SERVICE_NAME) {
        println!("\n[OK] 重启成功");
    } else {
        println!("\n[ERROR] 重启失败，查看日志: bingxi logs");
    }
}

pub(super) fn cmd_logs(lines: u16, follow: bool, log_type: &str) {
    let lines_str = lines.to_string();

    match log_type {
        "backend" => {
            let mut args = vec!["-u", SERVICE_NAME, "-n", &lines_str];
            if follow {
                args.push("-f");
            }
            let _ = Command::new("journalctl")
                .args(&args)
                .stdin(std::process::Stdio::inherit())
                .status();
        }
        "frontend" => {
            let path = format!("{}/frontend/logs/error.log", get_install_dir());
            let mut args = vec!["-n", &lines_str];
            if follow {
                args.push("-f");
            }
            args.push(&path);
            let _ = Command::new("tail")
                .args(&args)
                .stdin(std::process::Stdio::inherit())
                .status();
        }
        "system" => {
            let mut args = vec!["-n", &lines_str];
            if follow {
                args.push("-f");
            }
            let _ = Command::new("journalctl")
                .args(&args)
                .stdin(std::process::Stdio::inherit())
                .status();
        }
        _ => {
            println!("[ERROR] 未知日志类型: {}", log_type);
            println!("可用: backend, frontend, system");
        }
    }
}

pub(super) fn cmd_health() {
    println!("=== 健康检查 ===\n");

    // 服务状态
    let backend_ok = is_service_active(SERVICE_NAME);
    println!("{} 后端服务", status_icon(backend_ok));

    // HTTP 检查
    println!("\n检查 HTTP 接口...");
    match run_cmd(
        "curl",
        &[
            "-s",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            "http://127.0.0.1:8082/api/v1/erp/health",
        ],
    ) {
        Ok(code) => println!(
            "{} HTTP 状态码: {}",
            if code == "200" { "[OK]" } else { "[WARN]" },
            code
        ),
        Err(e) => println!("[ERROR] HTTP 检查失败: {}", e),
    }

    // 数据库检查
    println!("\n检查数据库...");
    let db_host = require_env(
        "DATABASE__HOST",
        "请设置数据库主机地址，例如 export DATABASE__HOST=127.0.0.1",
    );
    let db_user = require_env(
        "DATABASE__USERNAME",
        "请设置数据库用户名，例如 export DATABASE__USERNAME=postgres",
    );
    let db_name = require_env(
        "DATABASE__NAME",
        "请设置数据库名称，例如 export DATABASE__NAME=bingxi_erp",
    );

    match run_cmd(
        "psql",
        &[
            "-h",
            &db_host,
            "-U",
            &db_user,
            "-d",
            &db_name,
            "-c",
            "SELECT 1;",
        ],
    ) {
        Ok(_) => println!("[OK] 数据库连接正常"),
        Err(e) => println!("[ERROR] 数据库连接失败: {}", e),
    }

    // 磁盘空间
    println!("\n--- 磁盘使用 ---");
    if let Ok(df) = run_cmd("df", &["-h", &get_install_dir()]) {
        println!("{}", df);
    }

    // 日志大小
    println!("\n--- 日志大小 ---");
    if let Ok(du) = run_cmd("du", &["-sh", &get_log_dir()]) {
        println!("{}", du);
    }
}
