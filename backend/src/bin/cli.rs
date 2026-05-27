//! ERP 命令行工具
//! 用途：系统运维和管理

use clap::{Parser, Subcommand};
use std::process::Command;

// 服务名称常量
const SERVICE_NAME: &str = "bingxi-backend";
const INSTALL_DIR: &str = "/opt/bingxi-erp";
const LOG_DIR: &str = "/opt/bingxi-erp/backend/logs";
const BACKUP_DIR: &str = "/opt/bingxi-erp/backups";

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
        /// 日志类型：backend/frontend/system
        #[arg(short, long, default_value = "backend")]
        log_type: String,
    },
    /// 备份数据
    Backup {
        /// 备份类型：database/files/all
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
    /// 系统升级
    Upgrade {
        /// 目标版本号
        #[arg(short, long)]
        version: Option<String>,
        /// 跳过备份
        #[arg(long)]
        no_backup: bool,
    },
    /// 清理缓存和旧文件
    Clean {
        /// 清理类型：logs/backups/all
        #[arg(short, long, default_value = "all")]
        clean_type: String,
    },
    /// 显示配置
    Config,
    /// 生成密码哈希
    HashPassword {
        /// 原始密码
        #[arg(short, long)]
        password: String,
    },
    /// 部署更新包
    Deploy {
        /// 更新包路径
        #[arg(short, long)]
        package: String,
    },
    /// 查看系统信息
    Info,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Status => cmd_status(),
        Commands::Start => cmd_start(),
        Commands::Stop => cmd_stop(),
        Commands::Restart => cmd_restart(),
        Commands::Logs { lines, follow, log_type } => cmd_logs(lines, follow, &log_type),
        Commands::Backup { backup_type } => cmd_backup(&backup_type),
        Commands::Restore { file } => cmd_restore(&file),
        Commands::Health => cmd_health().await,
        Commands::Upgrade { version, no_backup } => cmd_upgrade(version, no_backup).await,
        Commands::Clean { clean_type } => cmd_clean(&clean_type),
        Commands::Config => cmd_config(),
        Commands::HashPassword { password } => cmd_hash_password(&password),
        Commands::Deploy { package } => cmd_deploy(&package),
        Commands::Info => cmd_info(),
    }

    Ok(())
}

/// 执行系统命令并返回输出
fn run_cmd(cmd: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| format!("执行命令失败: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    
    if output.status.success() {
        Ok(stdout.trim().to_string())
    } else {
        Err(format!("{}\n{}", stdout, stderr).trim().to_string())
    }
}

/// 检查服务状态
fn is_service_active(service: &str) -> bool {
    run_cmd("systemctl", &["is-active", service])
        .map(|s| s == "active")
        .unwrap_or(false)
}

/// 打印状态图标
fn status_icon(active: bool) -> &'static str {
    if active { "[OK]" } else { "[STOPPED]" }
}

fn cmd_status() {
    println!("=== Bingxi ERP 服务状态 ===\n");
    
    // 后端服务
    let backend_active = is_service_active(SERVICE_NAME);
    println!("{} 后端服务 ({})", status_icon(backend_active), SERVICE_NAME);
    
    // Nginx
    let nginx_active = is_service_active("nginx");
    println!("{} Nginx 服务", status_icon(nginx_active));
    
    // Redis
    let redis_active = is_service_active("redis");
    println!("{} Redis 服务", status_icon(redis_active));
    
    // 检查端口
    println!("\n--- 端口监听状态 ---");
    if let Ok(output) = run_cmd("ss", &["-tlnp"]) {
        if output.contains(":8082") {
            println!("[OK] 端口 8082 (后端)");
        } else {
            println!("[STOPPED] 端口 8082 (后端)");
        }
        if output.contains(":80") {
            println!("[OK] 端口 80 (Nginx)");
        } else {
            println!("[STOPPED] 端口 80 (Nginx)");
        }
    }
    
    // 检查进程
    println!("\n--- 进程状态 ---");
    if let Ok(output) = run_cmd("pgrep", &["-f", "server"]) {
        if !output.is_empty() {
            println!("[OK] 后端进程运行中 (PID: {})", output.lines().next().unwrap_or("unknown"));
        } else {
            println!("[STOPPED] 后端进程未运行");
        }
    }
}

fn cmd_start() {
    println!("启动服务...\n");
    
    // 启动后端
    println!("启动后端服务...");
    match run_cmd("systemctl", &["start", SERVICE_NAME]) {
        Ok(_) => println!("[OK] 后端服务已启动"),
        Err(e) => println!("[ERROR] 启动后端失败: {}", e),
    }
    
    // 重新加载 Nginx
    println!("重新加载 Nginx...");
    match run_cmd("systemctl", &["reload", "nginx"]) {
        Ok(_) => println!("[OK] Nginx 已重新加载"),
        Err(e) => println!("[WARN] Nginx 重新加载失败: {}", e),
    }
    
    // 等待并检查状态
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    if is_service_active(SERVICE_NAME) {
        println!("\n[OK] 服务启动成功");
    } else {
        println!("\n[ERROR] 服务启动失败，请检查日志: bingxi logs");
    }
}

fn cmd_stop() {
    println!("停止服务...\n");
    
    match run_cmd("systemctl", &["stop", SERVICE_NAME]) {
        Ok(_) => println!("[OK] 后端服务已停止"),
        Err(e) => println!("[ERROR] 停止服务失败: {}", e),
    }
}

fn cmd_restart() {
    println!("重启服务...\n");
    
    // 停止
    println!("停止服务...");
    let _ = run_cmd("systemctl", &["stop", SERVICE_NAME]);
    std::thread::sleep(std::time::Duration::from_secs(1));
    
    // 启动
    println!("启动服务...");
    match run_cmd("systemctl", &["start", SERVICE_NAME]) {
        Ok(_) => println!("[OK] 服务已重启"),
        Err(e) => println!("[ERROR] 重启失败: {}", e),
    }
    
    // 重新加载 Nginx
    let _ = run_cmd("systemctl", &["reload", "nginx"]);
    
    // 等待并检查状态
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    if is_service_active(SERVICE_NAME) {
        println!("\n[OK] 服务重启成功");
    } else {
        println!("\n[ERROR] 服务重启失败，请检查日志: bingxi logs");
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
            let log_path = format!("{}/logs/error.log", INSTALL_DIR);
            let mut args = vec!["-n", &lines_str];
            if follow {
                args.push("-f");
            }
            args.push(&log_path);
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
            println!("可用类型: backend, frontend, system");
        }
    }
}

fn cmd_backup(backup_type: &str) {
    use std::time::SystemTime;
    
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let backup_dir = format!("{}/{}", BACKUP_DIR, timestamp);
    
    println!("=== 开始备份 ===\n");
    println!("备份目录: {}", backup_dir);
    
    // 创建备份目录
    let _ = run_cmd("mkdir", &["-p", &backup_dir]);
    
    // 备份数据库
    if backup_type == "database" || backup_type == "all" {
        println!("\n备份数据库...");
        let db_backup = format!("{}/database.sql", backup_dir);
        
        // 从配置文件读取数据库连接信息
        let db_host = std::env::var("DATABASE__HOST").unwrap_or_else(|_| "39.99.34.194".to_string());
        let db_user = std::env::var("DATABASE__USERNAME").unwrap_or_else(|_| "bingxi".to_string());
        let db_name = std::env::var("DATABASE__NAME").unwrap_or_else(|_| "bingxi".to_string());
        
        match run_cmd("pg_dump", &["-h", &db_host, "-U", &db_user, "-d", &db_name, "-f", &db_backup]) {
            Ok(_) => println!("[OK] 数据库备份完成"),
            Err(e) => println!("[ERROR] 数据库备份失败: {}", e),
        }
    }
    
    // 备份文件
    if backup_type == "files" || backup_type == "all" {
        println!("\n备份文件...");
        
        // 备份后端配置
        let _ = run_cmd("cp", &["-r", &format!("{}/backend/config.yaml", INSTALL_DIR), &backup_dir]);
        let _ = run_cmd("cp", &["-r", &format!("{}/backend/.env", INSTALL_DIR), &backup_dir]);
        
        // 备份前端
        let _ = run_cmd("cp", &["-r", &format!("{}/frontend/dist", INSTALL_DIR), &backup_dir]);
        
        // 备份部署配置
        let _ = run_cmd("cp", &["-r", "/etc/systemd/system/bingxi-backend.service", &backup_dir]);
        let _ = run_cmd("cp", &["-r", "/etc/nginx/sites-available/bingxi", &backup_dir]);
        
        println!("[OK] 文件备份完成");
    }
    
    // 压缩备份
    println!("\n压缩备份文件...");
    let tar_file = format!("{}/backup_{}.tar.gz", BACKUP_DIR, timestamp);
    let _ = run_cmd("tar", &["-czf", &tar_file, "-C", BACKUP_DIR, &timestamp.to_string()]);
    let _ = run_cmd("rm", &["-rf", &backup_dir]);
    
    println!("\n[OK] 备份完成");
    println!("备份文件: {}", tar_file);
}

fn cmd_restore(file: &str) {
    println!("=== 恢复数据 ===\n");
    println!("备份文件: {}", file);
    
    if !std::path::Path::new(file).exists() {
        println!("[ERROR] 文件不存在: {}", file);
        return;
    }
    
    // 解压备份
    let temp_dir = "/tmp/bingxi_restore";
    let _ = run_cmd("rm", &["-rf", temp_dir]);
    let _ = run_cmd("mkdir", &["-p", temp_dir]);
    
    println!("解压备份文件...");
    match run_cmd("tar", &["-xzf", file, "-C", temp_dir]) {
        Ok(_) => println!("[OK] 解压完成"),
        Err(e) => {
            println!("[ERROR] 解压失败: {}", e);
            return;
        }
    }
    
    // 恢复数据库
    let db_file = format!("{}/database.sql", temp_dir);
    if std::path::Path::new(&db_file).exists() {
        println!("\n恢复数据库...");
        let db_host = std::env::var("DATABASE__HOST").unwrap_or_else(|_| "39.99.34.194".to_string());
        let db_user = std::env::var("DATABASE__USERNAME").unwrap_or_else(|_| "bingxi".to_string());
        let db_name = std::env::var("DATABASE__NAME").unwrap_or_else(|_| "bingxi".to_string());
        
        match run_cmd("psql", &["-h", &db_host, "-U", &db_user, "-d", &db_name, "-f", &db_file]) {
            Ok(_) => println!("[OK] 数据库恢复完成"),
            Err(e) => println!("[ERROR] 数据库恢复失败: {}", e),
        }
    }
    
    // 恢复配置文件
    println!("\n恢复配置文件...");
    let config_files = ["config.yaml", ".env"];
    for config in &config_files {
        let src = format!("{}/{}", temp_dir, config);
        if std::path::Path::new(&src).exists() {
            let dst = format!("{}/backend/{}", INSTALL_DIR, config);
            let _ = run_cmd("cp", &[&src, &dst]);
            println!("[OK] 恢复: {}", config);
        }
    }
    
    // 清理临时文件
    let _ = run_cmd("rm", &["-rf", temp_dir]);
    
    println!("\n[OK] 恢复完成");
    println!("请重启服务: bingxi restart");
}

async fn cmd_health() {
    println!("=== 健康检查 ===\n");
    
    // 检查后端服务
    let backend_active = is_service_active(SERVICE_NAME);
    println!("{} 后端服务", status_icon(backend_active));
    
    // 检查 HTTP 服务
    println!("\n检查 HTTP 服务...");
    match run_cmd("curl", &["-s", "-o", "/dev/null", "-w", "%{http_code}", "http://127.0.0.1:8082/api/v1/erp/health"]) {
        Ok(code) => {
            if code == "200" {
                println!("[OK] HTTP 服务正常 (状态码: {})", code);
            } else {
                println!("[WARN] HTTP 服务异常 (状态码: {})", code);
            }
        }
        Err(e) => println!("[ERROR] HTTP 检查失败: {}", e),
    }
    
    // 检查数据库连接
    println!("\n检查数据库连接...");
    let db_host = std::env::var("DATABASE__HOST").unwrap_or_else(|_| "39.99.34.194".to_string());
    let db_user = std::env::var("DATABASE__USERNAME").unwrap_or_else(|_| "bingxi".to_string());
    let db_name = std::env::var("DATABASE__NAME").unwrap_or_else(|_| "bingxi".to_string());
    
    match run_cmd("psql", &["-h", &db_host, "-U", &db_user, "-d", &db_name, "-c", "SELECT 1;"]) {
        Ok(_) => println!("[OK] 数据库连接正常"),
        Err(e) => println!("[ERROR] 数据库连接失败: {}", e),
    }
    
    // 检查 Redis
    println!("\n检查 Redis 连接...");
    match run_cmd("redis-cli", &["ping"]) {
        Ok(output) => {
            if output.contains("PONG") {
                println!("[OK] Redis 连接正常");
            } else {
                println!("[WARN] Redis 连接异常: {}", output);
            }
        }
        Err(e) => println!("[ERROR] Redis 检查失败: {}", e),
    }
    
    // 检查磁盘空间
    println!("\n磁盘使用情况:");
    match run_cmd("df", &["-h", INSTALL_DIR]) {
        Ok(output) => println!("{}", output),
        Err(_) => println!("[WARN] 无法获取磁盘信息"),
    }
    
    // 检查日志目录
    println!("\n日志目录大小:");
    match run_cmd("du", &["-sh", LOG_DIR]) {
        Ok(output) => println!("{}", output),
        Err(_) => println!("[WARN] 无法获取日志目录信息"),
    }
}

async fn cmd_upgrade(version: Option<String>, no_backup: bool) {
    println!("=== 系统升级 ===\n");
    
    // 获取当前版本
    let current_version = env!("CARGO_PKG_VERSION");
    println!("当前版本: v{}", current_version);
    
    // 获取目标版本
    let target_version = match &version {
        Some(v) => {
            let v = if v.starts_with('v') { v.clone() } else { format!("v{}", v) };
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
                    println!("请手动指定版本: bingxi upgrade --version v2026.x.x.xxxx");
                    return;
                }
            }
        }
    };
    
    // 备份当前版本
    if !no_backup {
        println!("\n备份当前版本...");
        cmd_backup("all");
    }
    
    // 下载新版本
    println!("\n下载新版本...");
    let download_path = format!("/tmp/release-{}.tar.gz", target_version);
    
    if !download_release(&target_version, &download_path) {
        println!("[ERROR] 下载失败");
        println!("请手动下载并使用 deploy 命令部署");
        return;
    }
    
    // 部署
    println!("\n部署新版本...");
    deploy_release(&download_path);
    
    // 清理
    let _ = run_cmd("rm", &["-f", &download_path]);
    
    println!("\n[OK] 升级完成");
    println!("新版本: {}", target_version);
    println!("备份位置: {}", BACKUP_DIR);
}

/// 获取最新版本号
fn get_latest_version() -> Option<String> {
    // 尝试直接访问 GitHub API
    match run_cmd("curl", &[
        "-s", "-m", "10",
        "-H", "Accept: application/vnd.github.v3+json",
        "https://api.github.com/repos/57231307/1/releases/latest"
    ]) {
        Ok(output) => {
            // 解析 JSON 获取 tag_name
            if let Some(tag) = parse_json_field(&output, "tag_name") {
                return Some(tag);
            }
        }
        Err(_) => {}
    }
    
    // 尝试镜像源
    let mirrors = [
        "https://ghproxy.net",
        "https://github.moeyy.xyz",
        "https://mirror.ghproxy.com",
    ];
    
    for mirror in &mirrors {
        let url = format!("{}/https://api.github.com/repos/57231307/1/releases/latest", mirror);
        match run_cmd("curl", &["-s", "-m", "10", "-H", "Accept: application/vnd.github.v3+json", &url]) {
            Ok(output) => {
                if let Some(tag) = parse_json_field(&output, "tag_name") {
                    return Some(tag);
                }
            }
            Err(_) => continue,
        }
    }
    
    None
}

/// 简单解析 JSON 字段
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

/// 下载发布包
fn download_release(version: &str, path: &str) -> bool {
    let github_url = format!(
        "https://github.com/57231307/1/releases/download/{}/release-{}.tar.gz",
        version, version
    );
    
    // 尝试直连
    println!("尝试直连 GitHub...");
    if run_cmd("curl", &["-fsSL", "-m", "60", "-o", path, &github_url]).is_ok() {
        println!("[OK] 下载成功");
        return true;
    }
    
    // 尝试镜像源
    let mirrors = [
        "https://ghproxy.net",
        "https://github.moeyy.xyz",
        "https://mirror.ghproxy.com",
    ];
    
    for mirror in &mirrors {
        let mirror_url = format!("{}/{}", mirror, github_url);
        println!("尝试镜像: {}...", mirror);
        if run_cmd("curl", &["-fsSL", "-m", "60", "-o", path, &mirror_url]).is_ok() {
            println!("[OK] 从 {} 下载成功", mirror);
            return true;
        }
    }
    
    false
}

/// 部署发布包
fn deploy_release(package_path: &str) {
    println!("停止服务...");
    let _ = run_cmd("systemctl", &["stop", SERVICE_NAME]);
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    // 解压
    println!("解压更新包...");
    let _ = run_cmd("tar", &["-xzf", package_path, "-C", "/tmp"]);
    
    let extract_dir = "/tmp/bingxi-erp";
    
    // 备份旧文件
    println!("备份旧文件...");
    let _ = run_cmd("mv", &[
        &format!("{}/backend/server", INSTALL_DIR),
        &format!("{}/backend/server.old", INSTALL_DIR),
    ]);
    let _ = run_cmd("mv", &[
        &format!("{}/backend/bingxi", INSTALL_DIR),
        &format!("{}/backend/bingxi.old", INSTALL_DIR),
    ]);
    
    // 替换后端
    println!("更新后端...");
    let _ = run_cmd("cp", &[
        &format!("{}/backend/server", extract_dir),
        &format!("{}/backend/server", INSTALL_DIR),
    ]);
    let _ = run_cmd("cp", &[
        &format!("{}/backend/bingxi", extract_dir),
        &format!("{}/backend/bingxi", INSTALL_DIR),
    ]);
    let _ = run_cmd("chmod", &["+x", &format!("{}/backend/server", INSTALL_DIR)]);
    let _ = run_cmd("chmod", &["+x", &format!("{}/backend/bingxi", INSTALL_DIR)]);
    
    // 替换前端
    println!("更新前端...");
    let _ = run_cmd("rm", &["-rf", &format!("{}/frontend/dist", INSTALL_DIR)]);
    let _ = run_cmd("mv", &[
        &format!("{}/frontend/dist", extract_dir),
        &format!("{}/frontend/dist", INSTALL_DIR),
    ]);
    
    // 清理
    let _ = run_cmd("rm", &["-rf", extract_dir]);
    
    // 启动服务
    println!("启动服务...");
    let _ = run_cmd("systemctl", &["start", SERVICE_NAME]);
    
    // 等待服务启动
    std::thread::sleep(std::time::Duration::from_secs(3));
    
    // 检查状态
    if is_service_active(SERVICE_NAME) {
        println!("[OK] 服务启动成功");
    } else {
        println!("[ERROR] 服务启动失败，请检查日志");
    }
}

fn cmd_clean(clean_type: &str) {
    println!("=== 清理系统 ===\n");
    
    if clean_type == "logs" || clean_type == "all" {
        println!("清理旧日志 (30天前)...");
        let _ = run_cmd("find", &[LOG_DIR, "-name", "*.log", "-mtime", "+30", "-delete"]);
        println!("[OK] 日志清理完成");
    }
    
    if clean_type == "backups" || clean_type == "all" {
        println!("清理旧备份 (90天前)...");
        let _ = run_cmd("find", &[BACKUP_DIR, "-name", "*", "-mtime", "+90", "-delete"]);
        println!("[OK] 备份清理完成");
    }
    
    // 清理临时文件
    println!("清理临时文件...");
    let _ = run_cmd("rm", &["-rf", "/tmp/release-*.tar.gz"]);
    let _ = run_cmd("rm", &["-rf", "/tmp/bingxi-erp"]);
    
    println!("\n[OK] 清理完成");
}

fn cmd_config() {
    println!("=== 系统配置 ===\n");
    
    // 后端配置
    let config_file = format!("{}/backend/config.yaml", INSTALL_DIR);
    println!("--- {} ---", config_file);
    match std::fs::read_to_string(&config_file) {
        Ok(content) => println!("{}", content),
        Err(_) => println!("[WARN] 配置文件不存在"),
    }
    
    // 环境变量
    println!("\n--- 环境变量 ---");
    let env_file = "/etc/bingxi/.env";
    match std::fs::read_to_string(env_file) {
        Ok(content) => {
            // 隐藏敏感信息
            for line in content.lines() {
                if line.contains("PASSWORD") || line.contains("SECRET") {
                    let parts: Vec<&str> = line.splitn(2, '=').collect();
                    if parts.len() == 2 {
                        println!("{}=***", parts[0]);
                    }
                } else {
                    println!("{}", line);
                }
            }
        }
        Err(_) => println!("[WARN] 环境变量文件不存在"),
    }
    
    // systemd 服务配置
    println!("\n--- 服务配置 ---");
    let service_file = format!("/etc/systemd/system/{}.service", SERVICE_NAME);
    match std::fs::read_to_string(&service_file) {
        Ok(content) => println!("{}", content),
        Err(_) => println!("[WARN] 服务配置文件不存在"),
    }
}

fn cmd_hash_password(password: &str) {
    println!("=== 生成密码哈希 ===\n");
    
    // 使用 argon2 生成哈希
    // 注意：这里需要 argon2 crate，如果没有则使用系统命令
    match run_cmd("htpasswd", &["-nbBC", "10", "", password]) {
        Ok(hash) => {
            println!("哈希值: {}", hash);
            println!("\n[OK] 密码哈希生成成功");
        }
        Err(_) => {
            // 尝试使用 python
            let python_cmd = format!(
                "import hashlib; print(hashlib.sha256('{}'.encode()).hexdigest())",
                password
            );
            match run_cmd("python3", &["-c", &python_cmd]) {
                Ok(hash) => {
                    println!("SHA256 哈希: {}", hash);
                    println!("\n[WARN] 使用 SHA256 哈希 (生产环境建议使用 argon2)");
                }
                Err(e) => println!("[ERROR] 生成哈希失败: {}", e),
            }
        }
    }
}

fn cmd_deploy(package: &str) {
    println!("=== 部署更新包 ===\n");
    println!("更新包: {}", package);
    
    if !std::path::Path::new(package).exists() {
        println!("[ERROR] 更新包不存在: {}", package);
        return;
    }
    
    deploy_release(package);
    
    println!("\n[OK] 部署完成");
}

fn cmd_info() {
    println!("=== Bingxi ERP 系统信息 ===\n");
    
    // 版本信息
    println!("版本: v{}", env!("CARGO_PKG_VERSION"));
    
    // 安装目录
    println!("安装目录: {}", INSTALL_DIR);
    
    // 服务状态
    println!("服务状态: {}", if is_service_active(SERVICE_NAME) { "运行中" } else { "已停止" });
    
    // 系统信息
    println!("\n--- 系统信息 ---");
    if let Ok(output) = run_cmd("uname", &["-a"]) {
        println!("系统: {}", output);
    }
    
    // 磁盘使用
    println!("\n--- 磁盘使用 ---");
    if let Ok(output) = run_cmd("df", &["-h", INSTALL_DIR]) {
        println!("{}", output);
    }
    
    // 内存使用
    println!("\n--- 内存使用 ---");
    if let Ok(output) = run_cmd("free", &["-h"]) {
        println!("{}", output);
    }
    
    // 运行时间
    println!("\n--- 服务运行时间 ---");
    match run_cmd("systemctl", &["show", SERVICE_NAME, "--property=ActiveEnterTimestamp"]) {
        Ok(output) => println!("{}", output),
        Err(_) => println!("无法获取运行时间"),
    }
}
