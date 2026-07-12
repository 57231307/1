//! 工具子命令：服务管理、备份恢复、升级部署、清理、配置信息等
//!
//! 本模块作为工具子命令的总入口：
//! - `UtilCommand` 枚举：所有 util 子命令的 clap 定义
//! - `run` 异步函数：按枚举变体分发到本目录下各子模块
//! - 共享辅助函数（`run_cmd`、`is_service_active`、环境/目录助手、镜像下载等）
//!   通过 `pub(crate)` 暴露给 [`super::admin`] 等同层模块使用
//!
//! 业务命令按职责拆分为同目录子文件：
//! - [`service`]：服务管理（Status / Start / Stop / Restart / Logs / Health）
//! - [`backup`]：备份与恢复（Backup / Restore）
//! - [`upgrade`]：升级、部署与回滚（Upgrade / Deploy / Rollback）
//! - [`misc`]：清理、配置、系统信息（Clean / Config / Info）

use std::process::Command;

pub mod backup;
pub mod misc;
pub mod service;
pub mod upgrade;

use clap::Subcommand;

// ==================== 配置常量 ====================

/// 服务名称 (systemd service name)
pub(crate) const SERVICE_NAME: &str = "bingxi";

/// GitHub 仓库
pub(crate) const GITHUB_REPO: &str = "57231307/1";

/// 国内 GitHub 加速镜像源 (按优先级排序)
pub(crate) const GITHUB_MIRRORS: &[&str] = &[
    "https://ghfast.top",         // FastGit 加速
    "https://ghproxy.net",        // GitHub Proxy
    "https://github.moeyy.xyz",   // Moeyy 加速
    "https://mirror.ghproxy.com", // 镜像加速
    "https://gh-proxy.com",       // 代理加速
    "https://ghps.cc",            // 加速代理
];

/// GitHub API 加速镜像
pub(crate) const GITHUB_API_MIRRORS: &[&str] = &[
    "https://ghfast.top",
    "https://ghproxy.net",
    "https://github.moeyy.xyz",
];

// ==================== 子命令枚举 ====================

/// 工具子命令枚举
#[derive(Subcommand, Debug)]
pub enum UtilCommand {
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

    /// 查看系统信息
    Info,

    /// 回滚到上一个版本
    Rollback,
}

/// 工具子命令入口分发
pub async fn run(cmd: UtilCommand) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        UtilCommand::Status => service::cmd_status(),
        UtilCommand::Start => service::cmd_start(),
        UtilCommand::Stop => service::cmd_stop(),
        UtilCommand::Restart => service::cmd_restart(),
        UtilCommand::Logs {
            lines,
            follow,
            log_type,
        } => service::cmd_logs(lines, follow, &log_type),
        UtilCommand::Backup { backup_type } => { let _ = backup::cmd_backup(&backup_type); },
        UtilCommand::Restore { file } => { let _ = backup::cmd_restore(&file); },
        UtilCommand::Health => service::cmd_health(),
        UtilCommand::Upgrade { version, no_backup } => upgrade::cmd_upgrade(version, no_backup),
        UtilCommand::Deploy { package } => upgrade::cmd_deploy(&package),
        UtilCommand::Clean { clean_type } => misc::cmd_clean(&clean_type),
        UtilCommand::Config => misc::cmd_config(),
        UtilCommand::Info => misc::cmd_info(),
        UtilCommand::Rollback => upgrade::cmd_rollback(),
    }
    Ok(())
}

// ==================== 共享辅助函数 ====================

/// 获取安装目录 (支持环境变量覆盖)
pub(crate) fn get_install_dir() -> String {
    std::env::var("BINGXI_INSTALL_DIR").unwrap_or_else(|_| "/opt/bingxi-erp".to_string())
}

/// 获取日志目录 (支持环境变量覆盖)
pub(crate) fn get_log_dir() -> String {
    std::env::var("BINGXI_LOG_DIR").unwrap_or_else(|_| format!("{}/logs", get_install_dir()))
}

/// 获取备份目录 (支持环境变量覆盖)
pub(crate) fn get_backup_dir() -> String {
    std::env::var("BINGXI_BACKUP_DIR").unwrap_or_else(|_| format!("{}/backups", get_install_dir()))
}

/// 必需的环境变量读取助手：缺失或为空时打印明确错误并退出
pub(crate) fn require_env(key: &str, hint: &str) -> String {
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

/// 执行系统命令
pub(crate) fn run_cmd(cmd: &str, args: &[&str]) -> Result<String, String> {
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
pub(crate) fn is_service_active(service: &str) -> bool {
    run_cmd("systemctl", &["is-active", service])
        .map(|s| s == "active")
        .unwrap_or(false)
}

/// 状态图标
pub(crate) fn status_icon(ok: bool) -> &'static str {
    if ok {
        "[OK]"
    } else {
        "[STOPPED]"
    }
}

/// 构建 GitHub Release 下载链接
pub(crate) fn build_release_url(version: &str) -> String {
    format!(
        "https://github.com/{}/releases/download/{}/release-{}.tar.gz",
        GITHUB_REPO, version, version
    )
}


/// 带镜像源的下载 (自动尝试多个镜像)
pub(crate) fn download_with_mirrors(url: &str, output: &str, timeout: u32) -> bool {
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
pub(crate) fn fetch_with_mirrors(api_path: &str, timeout: u32) -> Option<String> {
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
pub(crate) fn parse_json_field(json: &str, field: &str) -> Option<String> {
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
pub(crate) fn timestamp() -> u64 {
    // P3 1-17 修复：原 .unwrap() 在系统时间异常时 panic，改为 .unwrap_or_default() 安全降级
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
