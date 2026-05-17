//! 秉羲 ERP 命令行工具
//! 用途：系统运维和管理

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "bingxi")]
#[command(author = "Bingxi Team")]
#[command(version = "2.0.0")]
#[command(about = "秉羲 ERP 系统命令行工具", long_about = None)]
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
            let status = String::from_utf8_lossy(&output.stdout).trim();
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
            let status = String::from_utf8_lossy(&output.stdout).trim();
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
            let status = String::from_utf8_lossy(&output.stdout).trim();
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
    
    match log_type {
        "backend" => {
            let mut args = vec!["-u", "bingxi", "-n", &lines.to_string()];
            if follow {
                args.push("-f");
            }
            Command::new("journalctl").args(&args).stdin(Stdio::inherit()).status()?;
        }
        "frontend" => {
            let log_path = "/opt/bingxi/frontend/logs/error.log";
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
    let backup_dir = format!("/opt/bingxi/backups/{}", timestamp);
    
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
            .args(["-r", "/opt/bingxi/backend", &backup_dir])
            .status()?;
        Command::new("cp")
            .args(["-r", "/opt/bingxi/frontend", &backup_dir])
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
    
    // 检查 HTTP 服务
    match Command::new("curl")
        .args(["-s", "-o", "/dev/null", "-w", "%{http_code}", "http://127.0.0.1:8082/health"])
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
    use std::process::Command;
    
    println!("🔄 执行数据库迁移 (方向：{})...", direction);
    
    let migration_dir = "/opt/bingxi/database/migration";
    
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

async fn cmd_clean() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;
    
    println!("🧹 清理缓存...");
    
    // 清理 cargo 缓存
    if std::path::Path::new("~/.cargo/registry").exists() {
        println!("📦 清理 Cargo 缓存...");
    }
    
    // 清理日志
    Command::new("find").args([
        "/opt/bingxi/logs",
        "-name",
        "*.log",
        "-mtime",
        "+30",
        "-delete",
    ]).status()?;
    println!("✅ 清理 30 天前的日志");
    
    // 清理备份
    Command::new("find").args([
        "/opt/bingxi/backups",
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
        "/opt/bingxi/backend/config.yaml",
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
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    
    println!("密码：{}", password);
    println!("哈希：{}\n", hash);
    println!("⚠️  请妥善保管哈希值，不要泄露原始密码");
    
    Ok(())
}
