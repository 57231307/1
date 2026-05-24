//! ERP 命令行工具
//! 用途：系统运维和管理

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "bingxi")]
#[command(author = "Bingxi Team")]
#[command(version = "2.0.0")]
#[command(about = "ERP 系统命令行工具", long_about = None)]
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
        #[arg(short, long, default_value = "100")]
        lines: u16,
        #[arg(short, long)]
        follow: bool,
        #[arg(short, long, help = "日志类型：backend/frontend/nginx")]
        log_type: Option<String>,
    },
    /// 备份数据
    Backup {
        #[arg(short, long, help = "备份类型：database/files/all")]
        backup_type: Option<String>,
    },
    /// 恢复数据
    Restore {
        #[arg(short, long, help = "备份文件路径")]
        file: String,
    },
    /// 健康检查
    Health,
    /// 数据库迁移
    Migrate {
        #[arg(short, long, default_value = "up")]
        direction: String,
    },
    /// 版本回滚
    Rollback {
        #[arg(short, long, help = "目标版本号")]
        version: Option<String>,
    },
    /// 系统升级
    Upgrade {
        #[arg(short, long, help = "目标版本号，不指定则升级到最新版")]
        version: Option<String>,
        #[arg(short, long)]
        no_backup: bool,
    },
    /// 清理缓存
    Clean,
    /// 显示配置
    Config,
    /// 生成密码哈希
    HashPassword {
        #[arg(short, long)]
        password: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Status => cmd_status().await?,
        Commands::Start => cmd_start().await?,
        Commands::Stop => cmd_stop().await?,
        Commands::Restart => cmd_restart().await?,
        Commands::Logs { lines, follow, log_type } => {
            cmd_logs(lines, follow, log_type).await?
        }
        Commands::Backup { backup_type } => cmd_backup(backup_type).await?,
        Commands::Restore { file } => cmd_restore(&file).await?,
        Commands::Health => cmd_health().await?,
        Commands::Migrate { direction } => cmd_migrate(&direction).await?,
        Commands::Rollback { version } => cmd_rollback(version).await?,
        Commands::Upgrade { version, no_backup } => cmd_upgrade(version, no_backup).await?,
        Commands::Clean => cmd_clean().await?,
        Commands::Config => cmd_config().await?,
        Commands::HashPassword { password } => cmd_hash_password(&password).await?,
    }

    Ok(())
}

async fn cmd_status() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;
    
    println!("🔍 检查服务状态...\n");
    
    // 检查后端服务
    match Command::new("systemctl").args(["is-active", "bingxi"]).output() {
        Ok(output) => {
            let status = String::from_utf8_lossy(&output.stdout);
            let status = status.trim();
            if status == "active" {
                println!("✅ 后端服务：运行中");
            } else {
                println!("❌ 后端服务：已停止");
            }
        }
        Err(_) => println!("❌ 后端服务：未安装"),
    }
    
    // 检查 Nginx
    match Command::new("systemctl").args(["is-active", "nginx"]).output() {
        Ok(output) => {
            let status = String::from_utf8_lossy(&output.stdout);
            let status = status.trim();
            if status == "active" {
                println!("✅ Nginx 服务：运行中");
            } else {
                println!("❌ Nginx 服务：已停止");
            }
        }
        Err(_) => println!("❌ Nginx 服务：未安装"),
    }
    
    // 检查 PostgreSQL
    match Command::new("systemctl").args(["is-active", "postgresql"]).output() {
        Ok(output) => {
            let status = String::from_utf8_lossy(&output.stdout);
            let status = status.trim();
            if status == "active" {
                println!("✅ PostgreSQL 服务：运行中");
            } else {
                println!("❌ PostgreSQL 服务：已停止");
            }
        }
        Err(_) => println!("⚠️  PostgreSQL 服务：使用远程数据库"),
    }
    
    Ok(())
}

async fn cmd_start() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;
    
    println!("🚀 启动服务...");
    
    Command::new("systemctl").args(["start", "bingxi"]).status()?;
    println!("✅ 后端服务已启动");
    
    Command::new("systemctl").args(["reload", "nginx"]).status()?;
    println!("✅ Nginx 已重新加载");
    
    Ok(())
}

async fn cmd_stop() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;
    
    println!("🛑 停止服务...");
    
    Command::new("systemctl").args(["stop", "bingxi"]).status()?;
    println!("✅ 后端服务已停止");
    
    Ok(())
}

async fn cmd_restart() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;
    
    println!("🔄 重启服务...");
    
    Command::new("systemctl").args(["restart", "bingxi"]).status()?;
    println!("✅ 后端服务已重启");
    
    Command::new("systemctl").args(["reload", "nginx"]).status()?;
    println!("✅ Nginx 已重新加载");
    
    Ok(())
}

async fn cmd_logs(
    lines: u16,
    follow: bool,
    log_type: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::process::{Command, Stdio};
    
    let log_type = log_type.as_deref().unwrap_or("backend");
    let lines_str = lines.to_string();
    
    match log_type {
        "backend" => {
            let mut args = vec!["-u", "bingxi", "-n", &lines_str];
            if follow {
                args.push("-f");
            }
            Command::new("journalctl").args(&args).stdin(Stdio::inherit()).status()?;
        }
        "frontend" => {
            let log_path = "/opt/bingxi-erp/frontend/logs/error.log";
            if follow {
                Command::new("tail").args(["-f", log_path]).stdin(Stdio::inherit()).status()?;
            } else {
                Command::new("tail").args(["-n", &lines.to_string(), log_path])
                    .stdin(Stdio::inherit())
                    .status()?;
            }
        }
        "nginx" => {
            let log_path = "/var/log/nginx/error.log";
            if follow {
                Command::new("tail").args(["-f", log_path]).stdin(Stdio::inherit()).status()?;
            } else {
                Command::new("tail").args(["-n", &lines.to_string(), log_path])
                    .stdin(Stdio::inherit())
                    .status()?;
            }
        }
        _ => {
            println!("❌ 未知日志类型：{}", log_type);
            println!("可用类型：backend, frontend, nginx");
        }
    }
    
    Ok(())
}

async fn cmd_backup(backup_type: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;
    use std::time::SystemTime;
    
    let backup_type = backup_type.as_deref().unwrap_or("all");
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();
    let backup_dir = format!("/opt/bingxi-erp/backups/{}", timestamp);
    
    println!("💾 开始备份...");
    println!("📁 备份目录：{}", backup_dir);
    
    Command::new("mkdir").args(["-p", &backup_dir]).status()?;
    
    match backup_type {
        "database" | "all" => {
            println!("📊 备份数据库...");
            let db_backup = format!("{}/database.sql", backup_dir);
            Command::new("pg_dump")
                .args(&[
                    "-h", "111.230.99.236",
                    "-U", "bingxi",
                    "-d", "bingxi",
                    "-f",
                    &db_backup,
                ])
                .status()?;
        }
        _ => {}
    }
    
    if backup_type == "files" || backup_type == "all" {
        println!("📁 备份文件...");
        Command::new("cp")
            .args(["-r", "/opt/bingxi-erp/backend", &backup_dir])
            .status()?;
        Command::new("cp")
            .args(["-r", "/opt/bingxi-erp/frontend", &backup_dir])
            .status()?;
    }
    
    println!("✅ 备份完成：{}", backup_dir);
    
    Ok(())
}

async fn cmd_restore(file: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;
    
    println!("♻️  恢复数据...");
    println!("📁 备份文件：{}", file);
    
    if !std::path::Path::new(file).exists() {
        println!("❌ 文件不存在：{}", file);
        return Ok(());
    }
    
    if file.ends_with(".sql") {
        println!("📊 恢复数据库...");
        Command::new("psql")
            .args(&[
                "-h", "111.230.99.236",
                "-U", "bingxi",
                "-d", "bingxi",
                "-f",
                file,
            ])
            .status()?;
        println!("✅ 数据库恢复完成");
    } else {
        println!("⚠️  仅支持 SQL 文件恢复");
    }
    
    Ok(())
}

async fn cmd_health() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;
    
    println!("🏥 健康检查...\n");
    
    // 支持环境变量配置健康检查地址
    let health_url = std::env::var("BINGXI_HEALTH_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8082/api/v1/erp/health".to_string());
    
    // 检查 HTTP 服务
    match Command::new("curl")
        .args(["-s", "-o", "/dev/null", "-w", "%{http_code}", &health_url])
        .output()
    {
        Ok(output) => {
            let status = String::from_utf8_lossy(&output.stdout);
            if status == "200" {
                println!("✅ HTTP 服务：正常");
            } else {
                println!("❌ HTTP 服务：异常 (状态码：{})", status);
            }
        }
        Err(_) => println!("❌ HTTP 服务：无法连接"),
    }
    
    // 检查数据库连接
    match Command::new("psql")
        .args(&[
            "-h", "111.230.99.236",
            "-U", "bingxi",
            "-d", "bingxi",
            "-c",
            "SELECT 1;",
        ])
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                println!("✅ 数据库连接：正常");
            } else {
                println!("❌ 数据库连接：失败");
            }
        }
        Err(_) => println!("❌ 数据库连接：无法执行"),
    }
    
    // 检查磁盘空间
    match Command::new("df").args(["-h", "/opt/bingxi"]).output() {
        Ok(output) => {
            let output_str = String::from_utf8_lossy(&output.stdout);
            println!("💾 磁盘空间:\n{}", output_str);
        }
        Err(_) => println!("⚠️  无法检查磁盘空间"),
    }
    
    Ok(())
}

async fn cmd_migrate(direction: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 执行数据库迁移 (方向：{})...", direction);
    
    let migration_dir = "/opt/bingxi-erp/backend/database/migration";
    
    if !std::path::Path::new(migration_dir).exists() {
        println!("❌ 迁移目录不存在：{}", migration_dir);
        return Ok(());
    }
    
    match direction {
        "up" => {
            println!("⬆️  应用迁移...");
            // 这里应该调用实际的迁移工具
            println!("⚠️  请手动执行迁移脚本");
        }
        "down" => {
            println!("⬇️  回滚迁移...");
            println!("⚠️  请手动执行回滚脚本");
        }
        _ => {
            println!("❌ 未知方向：{}", direction);
            println!("可用方向：up, down");
        }
    }
    
    Ok(())
}

async fn cmd_rollback(version: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔙 版本回滚...");
    
    match version {
        Some(v) => {
            println!("📦 回滚到版本：{}", v);
            println!("⚠️  请使用 git checkout 或手动替换文件");
        }
        None => {
            println!("⚠️  请指定目标版本：--version <版本号>");
        }
    }
    
    Ok(())
}

async fn cmd_upgrade(version: Option<String>, no_backup: bool) -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;
    use std::time::SystemTime;
    
    println!("⬆️  系统升级...\n");
    
    // 多个加速镜像源，自动切换
    let mirror_urls = vec![
        "https://ghproxy.net",
        "https://github.moeyy.xyz",
        "https://gh.api.99988866.xyz",
        "https://mirror.ghproxy.com",
    ];
    
    // 1. 检查当前版本
    let current_version = env!("CARGO_PKG_VERSION");
    println!("📦 当前版本：v{}", current_version);
    
    // 2. 获取目标版本
    let target_version = match &version {
        Some(v) => v.clone(),
        None => {
            println!("🔍 获取最新版本号...");
            // 尝试多个镜像源获取最新版本
            let mut latest_version: Option<String> = None;
            
            for mirror in &mirror_urls {
                println!("🌐 尝试从 {} 获取版本...", mirror);
                let api_url = format!("{}/https://api.github.com/repos/57231307/1/releases/latest", mirror);
                let output = Command::new("curl")
                    .args([
                        "-s", "-m", "10", // 10 秒超时
                        "-H", "Accept: application/vnd.github.v3+json",
                        &api_url
                    ])
                    .output();
                
                if let Ok(out) = output {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    if stdout.contains("tag_name") {
                        if let Some(tag_name) = stdout.split('"')
                            .skip_while(|s| !s.starts_with("tag_name"))
                            .nth(2)
                            .map(|s| s.split('"').next().unwrap_or(""))
                        {
                            if tag_name.starts_with('v') {
                                latest_version = Some(tag_name.to_string());
                                println!("✅ 从 {} 获取成功：{}\n", mirror, tag_name);
                                break;
                            }
                        }
                    }
                }
            }
            
            match latest_version {
                Some(v) => v,
                None => {
                    println!("❌ 所有镜像源都无法获取最新版本");
                    println!("请手动指定版本：bingxi upgrade --version v2026.x.x.xxxx");
                    return Ok(());
                }
            }
        }
    };
    
    println!("🎯 目标版本：{}\n", target_version);
    
    // 3. 备份当前版本
    if !no_backup {
        println!("💾 备份当前版本...");
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();
        let backup_dir = format!("/opt/bingxi-erp/backups/pre_upgrade_{}", timestamp);
        
        Command::new("mkdir").args(["-p", &backup_dir]).status()?;
        Command::new("cp").args(["-r", "/opt/bingxi-erp/backend", &backup_dir]).status()?;
        Command::new("cp").args(["-r", "/opt/bingxi-erp/frontend", &backup_dir]).status()?;
        
        println!("✅ 备份完成：{}\n", backup_dir);
    }
    
    // 4. 下载新版本（尝试多个镜像源）
    println!("📥 下载新版本...");
    let github_url = format!(
        "https://github.com/57231307/1/releases/download/{}/release-{}.tar.gz",
        target_version, target_version
    );
    let download_path = format!("/tmp/release-{}.tar.gz", target_version);
    
    let mut download_success = false;
    
    // 先尝试直连
    println!("🌐 尝试直连 GitHub...");
    let download_result = Command::new("curl")
        .args([
            "-fsSL", "-m", "30", // 30 秒超时
            "-o", &download_path,
            &github_url
        ])
        .status();
    
    if download_result.is_ok() && download_result.unwrap().success() {
        download_success = true;
        println!("✅ 直连下载成功\n");
    } else {
        // 尝试镜像源
        for mirror in &mirror_urls {
            println!("🌐 尝试镜像源：{}...", mirror);
            let mirror_url = format!("{}/{}", mirror, github_url);
            
            let download_result = Command::new("curl")
                .args([
                    "-fsSL", "-m", "30", // 30 秒超时
                    "-o", &download_path,
                    &mirror_url
                ])
                .status();
            
            if download_result.is_ok() && download_result.unwrap().success() {
                download_success = true;
                println!("✅ 从 {} 下载成功\n", mirror);
                break;
            } else {
                println!("⚠️  {} 下载失败，尝试下一个...", mirror);
            }
        }
    }
    
    if !download_success {
        println!("❌ 所有下载方式都失败了");
        println!("\n下载地址：{}", github_url);
        println!("\n请检查网络连接或手动下载后上传到服务器：");
        println!("  curl -fsSL -o /tmp/release-{}.tar.gz {}", target_version, github_url);
        return Ok(());
    }
    
    // 5. 停止服务
    println!("🛑 停止服务...");
    Command::new("systemctl").args(["stop", "bingxi"]).status()?;
    println!("✅ 服务已停止\n");
    
    // 6. 解压并替换
    println!("📦 解压新版本...");
    Command::new("tar")
        .args(["-xzf", &download_path, "-C", "/tmp"])
        .status()?;
    
    println!("🔄 替换文件...");
    
    // 备份旧文件
    Command::new("mv")
        .args(["/opt/bingxi-erp/backend/server", "/opt/bingxi-erp/backend/server.old"])
        .status()?;
    Command::new("mv")
        .args(["/opt/bingxi-erp/backend/bingxi", "/opt/bingxi-erp/backend/bingxi.old"])
        .status()?;
    Command::new("mv")
        .args(["/opt/bingxi-erp/frontend/dist", "/opt/bingxi-erp/frontend/dist.old"])
        .status()?;
    
    // 替换新文件 (从正确的路径复制)
    Command::new("cp")
        .args(["/tmp/bingxi-erp/backend/server", "/opt/bingxi-erp/backend/server"])
        .status()?;
    Command::new("cp")
        .args(["/tmp/bingxi-erp/backend/bingxi", "/opt/bingxi-erp/backend/bingxi"])
        .status()?;
    Command::new("rm")
        .args(["-rf", "/opt/bingxi-erp/frontend/dist"])
        .status()?;
    Command::new("mv")
        .args(["/tmp/bingxi-erp/frontend/dist", "/opt/bingxi-erp/frontend/dist"])
        .status()?;
    
    // 设置权限
    Command::new("chmod")
        .args(["+x", "/opt/bingxi-erp/backend/server"])
        .status()?;
    Command::new("chmod")
        .args(["+x", "/opt/bingxi-erp/backend/bingxi"])
        .status()?;
    
    println!("✅ 文件替换完成\n");
    
    // 7. 清理临时文件
    Command::new("rm").args(["-f", &download_path]).status()?;
    Command::new("rm").args(["-rf", "/tmp/bingxi-erp"]).status()?;
    
    // 8. 启动服务
    println!("🚀 启动服务...");
    Command::new("systemctl").args(["start", "bingxi"]).status()?;
    
    // 9. 健康检查
    println!("\n🏥 健康检查...");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    cmd_health().await?;
    
    println!("\n✅ 升级完成！");
    println!("📦 新版本：{}", target_version);
    println!("💾 备份位置：/opt/bingxi-erp/backups/pre_upgrade_*");
    println!("\n如需回滚，执行:");
    println!("  bingxi rollback --version {}", current_version);
    
    Ok(())
}

async fn cmd_clean() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;
    
    println!("🧹 清理缓存...");
    
    // 清理 cargo 缓存
    if std::path::Path::new("~/.cargo/registry").exists() {
        println!("📦 清理 Cargo 缓存...");
    }
    
    // 清理日志
    Command::new("find").args([
        "/opt/bingxi-erp/backend/logs",
        "-name",
        "*.log",
        "-mtime",
        "+30",
        "-delete",
    ]).status()?;
    println!("✅ 清理 30 天前的日志");
    
    // 清理备份
    Command::new("find").args([
        "/opt/bingxi-erp/backups",
        "-name",
        "*",
        "-mtime",
        "+90",
        "-delete",
    ]).status()?;
    println!("✅ 清理 90 天前的备份");
    
    Ok(())
}

async fn cmd_config() -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    
    println!("⚙️  系统配置:\n");
    
    let config_files = vec![
        "/opt/bingxi-erp/backend/config.yaml",
        "/etc/nginx/sites-available/bingxi",
    ];
    
    for config_file in config_files {
        println!("📄 {}", config_file);
        if let Ok(content) = fs::read_to_string(config_file) {
            println!("{}\n", content);
        } else {
            println!("❌ 文件不存在或无法读取\n");
        }
    }
    
    Ok(())
}

async fn cmd_hash_password(password: &str) -> Result<(), Box<dyn std::error::Error>> {
    use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
    use rand::rngs::OsRng;
    
    println!("🔐 生成密码哈希...\n");
    
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("密码哈希失败：{}", e)))?;
    
    println!("哈希：{}\n", hash);
    println!("⚠️  请妥善保管哈希值，不要泄露原始密码");
    
    Ok(())
}
