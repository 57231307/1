//! Bingxi ERP 命令行工具
//! 用途：系统运维、升级、备份和管理

use clap::{Parser, Subcommand};
use std::process::Command;

// ==================== 配置常量 ====================

/// 服务名称 (systemd service name)
const SERVICE_NAME: &str = "bingxi";

/// 获取安装目录 (支持环境变量覆盖)
fn get_install_dir() -> String {
    std::env::var("BINGXI_INSTALL_DIR").unwrap_or_else(|_| "/opt/bingxi-erp".to_string())
}

/// 获取日志目录 (支持环境变量覆盖)
fn get_log_dir() -> String {
    std::env::var("BINGXI_LOG_DIR").unwrap_or_else(|_| format!("{}/logs", get_install_dir()))
}

/// 获取备份目录 (支持环境变量覆盖)
fn get_backup_dir() -> String {
    std::env::var("BINGXI_BACKUP_DIR").unwrap_or_else(|_| format!("{}/backups", get_install_dir()))
}

/// 必需的环境变量读取助手：缺失或为空时打印明确错误并退出
fn require_env(key: &str, hint: &str) -> String {
    match std::env::var(key) {
        Ok(v) if !v.trim().is_empty() => v,
        _ => {
            eprintln!("❌ 错误：缺少必需的环境变量 {}", key);
            eprintln!("提示：{}", hint);
            eprintln!("请在 /etc/bingxi/.env 或当前 shell 中设置后重试。");
            std::process::exit(1);
        }
    }
}

/// GitHub 仓库
const GITHUB_REPO: &str = "57231307/1";

/// 国内 GitHub 加速镜像源 (按优先级排序)
const GITHUB_MIRRORS: &[&str] = &[
    "https://ghfast.top",         // FastGit 加速
    "https://ghproxy.net",        // GitHub Proxy
    "https://github.moeyy.xyz",   // Moeyy 加速
    "https://mirror.ghproxy.com", // 镜像加速
    "https://gh-proxy.com",       // 代理加速
    "https://ghps.cc",            // 加速代理
];

/// GitHub API 加速镜像
const GITHUB_API_MIRRORS: &[&str] = &[
    "https://ghfast.top",
    "https://ghproxy.net",
    "https://github.moeyy.xyz",
];

// ==================== CLI 定义 ====================

#[derive(Parser)]
#[command(name = "bingxi")]
#[command(author = "Bingxi Team")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Bingxi ERP 系统命令行工具", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 查看服务状态
    Status,

    /// 启动服务
    Start,

    /// 停止服务
    Stop,

    /// 重启服务
    Restart,

    /// 查看日志
    Logs {
        /// 日志行数
        #[arg(short, long, default_value = "100")]
        lines: u16,

        /// 实时跟踪
        #[arg(short, long)]
        follow: bool,

        /// 日志类型: backend / frontend / system
        #[arg(short, long, default_value = "backend")]
        log_type: String,
    },

    /// 备份数据
    Backup {
        /// 备份类型: database / files / all
        #[arg(short, long, default_value = "all")]
        backup_type: String,
    },

    /// 恢复数据
    Restore {
        /// 备份文件路径
        #[arg(short, long)]
        file: String,
    },

    /// 健康检查
    Health,

    /// 系统升级 (从 GitHub Release 下载)
    Upgrade {
        /// 目标版本号 (不指定则升级到最新)
        #[arg(short, long)]
        version: Option<String>,

        /// 跳过备份
        #[arg(long)]
        no_backup: bool,
    },

    /// 部署本地更新包
    Deploy {
        /// 更新包路径 (.tar.gz)
        #[arg(short, long)]
        package: String,
    },

    /// 清理缓存和旧文件
    Clean {
        /// 清理类型: logs / backups / temp / all
        #[arg(short, long, default_value = "all")]
        clean_type: String,
    },

    /// 显示配置信息
    Config,

    /// 生成密码哈希
    HashPassword {
        /// 原始密码
        #[arg(short, long)]
        password: String,
    },

    /// 查看系统信息
    Info,

    /// 回滚到上一个版本
    Rollback,
}

// ==================== 主入口 ====================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Status => cmd_status(),
        Commands::Start => cmd_start(),
        Commands::Stop => cmd_stop(),
        Commands::Restart => cmd_restart(),
        Commands::Logs {
            lines,
            follow,
            log_type,
        } => cmd_logs(lines, follow, &log_type),
        Commands::Backup { backup_type } => cmd_backup(&backup_type),
        Commands::Restore { file } => cmd_restore(&file),
        Commands::Health => cmd_health(),
        Commands::Upgrade { version, no_backup } => cmd_upgrade(version, no_backup),
        Commands::Deploy { package } => cmd_deploy(&package),
        Commands::Clean { clean_type } => cmd_clean(&clean_type),
        Commands::Config => cmd_config(),
        Commands::HashPassword { password } => cmd_hash_password(&password),
        Commands::Info => cmd_info(),
        Commands::Rollback => cmd_rollback(),
    }

    Ok(())
}

// ==================== 工具函数 ====================

/// 执行系统命令
fn run_cmd(cmd: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| format!("执行失败: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(stdout.trim().to_string())
    } else {
        Err(format!("{}\n{}", stdout, stderr).trim().to_string())
    }
}

/// 检查服务是否活跃
fn is_service_active(service: &str) -> bool {
    run_cmd("systemctl", &["is-active", service])
        .map(|s| s == "active")
        .unwrap_or(false)
}

/// 状态图标
fn status_icon(ok: bool) -> &'static str {
    if ok {
        "[OK]"
    } else {
        "[STOPPED]"
    }
}

/// 构建 GitHub Release 下载链接
fn build_release_url(version: &str) -> String {
    format!(
        "https://github.com/{}/releases/download/{}/release-{}.tar.gz",
        GITHUB_REPO, version, version
    )
}

/// 构建 GitHub API 链接
#[allow(dead_code)]
fn build_api_url() -> String {
    format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    )
}

/// 带镜像源的下载 (自动尝试多个镜像)
fn download_with_mirrors(url: &str, output: &str, timeout: u32) -> bool {
    // 1. 尝试直连
    println!("  尝试直连 GitHub...");
    if run_cmd(
        "curl",
        &["-fsSL", "-m", &timeout.to_string(), "-o", output, url],
    )
    .is_ok()
    {
        println!("  [OK] 直连下载成功");
        return true;
    }

    // 2. 尝试镜像源
    for mirror in GITHUB_MIRRORS {
        let mirror_url = format!("{}/{}", mirror, url);
        println!("  尝试镜像: {}...", mirror);
        if run_cmd(
            "curl",
            &[
                "-fsSL",
                "-m",
                &timeout.to_string(),
                "-o",
                output,
                &mirror_url,
            ],
        )
        .is_ok()
        {
            println!("  [OK] 从 {} 下载成功", mirror);
            return true;
        }
    }

    false
}

/// 带镜像源的 API 请求
fn fetch_with_mirrors(api_path: &str, timeout: u32) -> Option<String> {
    let full_url = format!("https://api.github.com/{}", api_path);

    // 1. 尝试直连
    if let Ok(output) = run_cmd(
        "curl",
        &[
            "-s",
            "-m",
            &timeout.to_string(),
            "-H",
            "Accept: application/vnd.github.v3+json",
            &full_url,
        ],
    ) {
        if output.contains("tag_name") {
            return Some(output);
        }
    }

    // 2. 尝试镜像源
    for mirror in GITHUB_API_MIRRORS {
        let mirror_url = format!("{}/{}", mirror, full_url);
        if let Ok(output) = run_cmd(
            "curl",
            &[
                "-s",
                "-m",
                &timeout.to_string(),
                "-H",
                "Accept: application/vnd.github.v3+json",
                &mirror_url,
            ],
        ) {
            if output.contains("tag_name") {
                return Some(output);
            }
        }
    }

    None
}

/// 解析 JSON 字段值
fn parse_json_field(json: &str, field: &str) -> Option<String> {
    let pattern = format!("\"{}\":\"", field);
    if let Some(start) = json.find(&pattern) {
        let value_start = start + pattern.len();
        if let Some(end) = json[value_start..].find('"') {
            return Some(json[value_start..value_start + end].to_string());
        }
    }
    None
}

/// 获取当前时间戳
fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

// ==================== 命令实现 ====================

fn cmd_status() {
    println!("=== Bingxi ERP 服务状态 ===\n");

    // 后端服务
    let backend_ok = is_service_active(SERVICE_NAME);
    println!("{} 后端服务 ({})", status_icon(backend_ok), SERVICE_NAME);

    // Nginx
    let nginx_ok = is_service_active("nginx");
    println!("{} Nginx 服务", status_icon(nginx_ok));

    // Redis
    let redis_ok = is_service_active("redis");
    println!("{} Redis 服务", status_icon(redis_ok));

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

fn cmd_start() {
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

fn cmd_stop() {
    println!("=== 停止服务 ===\n");

    match run_cmd("systemctl", &["stop", SERVICE_NAME]) {
        Ok(_) => println!("[OK] 后端已停止"),
        Err(e) => println!("[ERROR] 停止失败: {}", e),
    }
}

fn cmd_restart() {
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

fn cmd_logs(lines: u16, follow: bool, log_type: &str) {
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

fn cmd_backup(backup_type: &str) {
    let ts = timestamp();
    let backup_dir = format!("{}/{}", get_backup_dir(), ts);

    println!("=== 开始备份 ===\n");
    println!("备份目录: {}", backup_dir);

    let _ = run_cmd("mkdir", &["-p", &backup_dir]);

    // 备份数据库
    if backup_type == "database" || backup_type == "all" {
        println!("\n备份数据库...");
        let db_file = format!("{}/database.sql", backup_dir);
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
            "pg_dump",
            &[
                "-h", &db_host, "-U", &db_user, "-d", &db_name, "-f", &db_file,
            ],
        ) {
            Ok(_) => println!("[OK] 数据库备份完成"),
            Err(e) => println!("[ERROR] 数据库备份失败: {}", e),
        }
    }

    // 备份文件
    if backup_type == "files" || backup_type == "all" {
        println!("\n备份配置文件...");
        let config_dir = format!("{}/backend/config.yaml", get_install_dir());
        let env_file = "/etc/bingxi/.env";
        let service_file = format!("/etc/systemd/system/{}.service", SERVICE_NAME);

        let _ = run_cmd("cp", &["-r", &config_dir, &backup_dir]);
        let _ = run_cmd("cp", &["-r", env_file, &backup_dir]);
        let _ = run_cmd("cp", &["-r", &service_file, &backup_dir]);

        println!("[OK] 配置文件备份完成");
    }

    // 压缩
    println!("\n压缩备份...");
    let tar_file = format!("{}/backup_{}.tar.gz", get_backup_dir(), ts);
    let _ = run_cmd(
        "tar",
        &["-czf", &tar_file, "-C", &get_backup_dir(), &ts.to_string()],
    );
    let _ = run_cmd("rm", &["-rf", &backup_dir]);

    println!("\n[OK] 备份完成: {}", tar_file);
}

fn cmd_restore(file: &str) {
    println!("=== 恢复数据 ===\n");
    println!("备份文件: {}", file);

    if !std::path::Path::new(file).exists() {
        println!("[ERROR] 文件不存在: {}", file);
        return;
    }

    let temp_dir = "/tmp/bingxi_restore";
    let _ = run_cmd("rm", &["-rf", temp_dir]);
    let _ = run_cmd("mkdir", &["-p", temp_dir]);

    println!("解压备份...");
    if let Err(e) = run_cmd("tar", &["-xzf", file, "-C", temp_dir]) {
        println!("[ERROR] 解压失败: {}", e);
        return;
    }

    // 恢复数据库
    let db_file = format!("{}/database.sql", temp_dir);
    if std::path::Path::new(&db_file).exists() {
        println!("\n恢复数据库...");
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
                "-h", &db_host, "-U", &db_user, "-d", &db_name, "-f", &db_file,
            ],
        ) {
            Ok(_) => println!("[OK] 数据库恢复完成"),
            Err(e) => println!("[ERROR] 数据库恢复失败: {}", e),
        }
    }

    // 恢复配置
    println!("\n恢复配置文件...");
    for name in &["config.yaml", ".env"] {
        let src = format!("{}/{}", temp_dir, name);
        if std::path::Path::new(&src).exists() {
            let dst = if *name == ".env" {
                "/etc/bingxi/.env".to_string()
            } else {
                format!("{}/backend/{}", get_install_dir(), name)
            };
            let _ = run_cmd("cp", &[&src, &dst]);
            println!("[OK] 恢复: {}", name);
        }
    }

    let _ = run_cmd("rm", &["-rf", temp_dir]);

    println!("\n[OK] 恢复完成，请重启服务: bingxi restart");
}

fn cmd_health() {
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

    // Redis 检查
    println!("\n检查 Redis...");
    match run_cmd("redis-cli", &["ping"]) {
        Ok(out) => println!(
            "{} Redis: {}",
            if out.contains("PONG") {
                "[OK]"
            } else {
                "[WARN]"
            },
            out
        ),
        Err(e) => println!("[ERROR] Redis 检查失败: {}", e),
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

fn cmd_upgrade(version: Option<String>, no_backup: bool) {
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
        cmd_backup("all");
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

    // 清理
    let _ = run_cmd("rm", &["-f", &download_path]);

    println!("\n[OK] 升级完成");
    println!("新版本: {}", target);
    println!("备份位置: {}", get_backup_dir());
    println!("\n如需回滚: bingxi rollback");
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
    let _ = run_cmd("systemctl", &["stop", SERVICE_NAME]);
    std::thread::sleep(std::time::Duration::from_secs(2));

    println!("解压更新包...");
    let _ = run_cmd("tar", &["-xzf", package, "-C", "/tmp"]);

    let extract_dir = "/tmp/bingxi-erp";

    // 备份旧文件
    println!("备份旧文件...");
    let ts = timestamp();
    let old_backup = format!("{}/old.{}", get_install_dir(), ts);
    let _ = run_cmd("mkdir", &["-p", &old_backup]);
    let _ = run_cmd(
        "cp",
        &[
            "-r",
            &format!("{}/backend/server", get_install_dir()),
            &old_backup,
        ],
    );
    let _ = run_cmd(
        "cp",
        &[
            "-r",
            &format!("{}/backend/bingxi", get_install_dir()),
            &old_backup,
        ],
    );

    // 更新后端
    println!("更新后端...");
    let _ = run_cmd(
        "cp",
        &[
            &format!("{}/backend/server", extract_dir),
            &format!("{}/backend/server", get_install_dir()),
        ],
    );
    let _ = run_cmd(
        "cp",
        &[
            &format!("{}/backend/bingxi", extract_dir),
            &format!("{}/backend/bingxi", get_install_dir()),
        ],
    );
    let _ = run_cmd(
        "chmod",
        &["+x", &format!("{}/backend/server", get_install_dir())],
    );
    let _ = run_cmd(
        "chmod",
        &["+x", &format!("{}/backend/bingxi", get_install_dir())],
    );

    // 更新前端
    println!("更新前端...");
    let _ = run_cmd(
        "rm",
        &["-rf", &format!("{}/frontend/dist", get_install_dir())],
    );
    let _ = run_cmd(
        "mv",
        &[
            &format!("{}/frontend/dist", extract_dir),
            &format!("{}/frontend/dist", get_install_dir()),
        ],
    );

    // 清理
    let _ = run_cmd("rm", &["-rf", extract_dir]);

    // 启动
    println!("启动服务...");
    let _ = run_cmd("systemctl", &["start", SERVICE_NAME]);

    std::thread::sleep(std::time::Duration::from_secs(3));

    if is_service_active(SERVICE_NAME) {
        println!("[OK] 部署成功");
    } else {
        println!("[ERROR] 服务启动失败，请检查日志");
    }
}

fn cmd_deploy(package: &str) {
    println!("=== 部署更新包 ===\n");
    println!("更新包: {}", package);

    if !std::path::Path::new(package).exists() {
        println!("[ERROR] 文件不存在: {}", package);
        return;
    }

    deploy_release(package);

    println!("\n[OK] 部署完成");
}

fn cmd_rollback() {
    println!("=== 回滚版本 ===\n");

    let server_old = format!("{}/backend/server.old", get_install_dir());
    let bingxi_old = format!("{}/backend/bingxi.old", get_install_dir());

    if !std::path::Path::new(&server_old).exists() {
        println!("[ERROR] 未找到旧版本文件");
        println!("请确认之前执行过升级操作");
        return;
    }

    println!("停止服务...");
    let _ = run_cmd("systemctl", &["stop", SERVICE_NAME]);
    std::thread::sleep(std::time::Duration::from_secs(2));

    println!("恢复旧版本...");
    let _ = run_cmd(
        "mv",
        &[
            &server_old,
            &format!("{}/backend/server", get_install_dir()),
        ],
    );
    let _ = run_cmd(
        "mv",
        &[
            &bingxi_old,
            &format!("{}/backend/bingxi", get_install_dir()),
        ],
    );
    let _ = run_cmd(
        "chmod",
        &["+x", &format!("{}/backend/server", get_install_dir())],
    );
    let _ = run_cmd(
        "chmod",
        &["+x", &format!("{}/backend/bingxi", get_install_dir())],
    );

    println!("启动服务...");
    let _ = run_cmd("systemctl", &["start", SERVICE_NAME]);

    std::thread::sleep(std::time::Duration::from_secs(3));

    if is_service_active(SERVICE_NAME) {
        println!("\n[OK] 回滚成功");
    } else {
        println!("\n[ERROR] 回滚后服务启动失败，请检查日志");
    }
}

fn cmd_clean(clean_type: &str) {
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

fn cmd_config() {
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

fn cmd_hash_password(password: &str) {
    println!("=== 生成密码哈希 ===\n");

    // 使用 Python 生成哈希
    let escaped_password = password.replace('"', "\\\"");
    let python_code = format!(
        r#"
import hashlib
import base64
import os
try:
    from argon2 import PasswordHasher
    ph = PasswordHasher()
    hash = ph.hash("{}")
    print("Argon2 哈希:", hash)
except ImportError:
    salt = os.urandom(32)
    hash = hashlib.pbkdf2_hmac('sha256', '{}'.encode(), salt, 100000)
    print("PBKDF2 哈希:", base64.b64encode(salt + hash).decode())
"#,
        escaped_password, escaped_password
    );

    match run_cmd("python3", &["-c", &python_code]) {
        Ok(hash) => println!("{}", hash),
        Err(e) => println!("[ERROR] 生成失败: {}", e),
    }
}

fn cmd_info() {
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
