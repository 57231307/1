//! 管理员子命令：密码哈希、用户管理、强制登出等
//!
//! 原 cli.rs 中管理员相关的命令仅有 `hash-password`（生成密码哈希）。
//! 后续可在此模块扩展：用户激活、密码重置、强制登出等运维命令。

use clap::Subcommand;

/// 管理员子命令枚举
#[derive(Subcommand, Debug)]
pub enum AdminCommand {
    /// 生成密码哈希
    HashPassword {
        /// H-2 修复（v9 复审）：从 stdin 读取密码，避免命令行参数泄露（ps / /proc 可见）
        /// 用法：echo "密码" | bingxi admin hash-password --password-stdin
        /// 或：  bingxi admin hash-password --password-stdin < password.txt
        #[arg(long)]
        password_stdin: bool,
    },
}

/// 管理员子命令入口分发
pub async fn run(cmd: AdminCommand) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        AdminCommand::HashPassword { password_stdin } => cmd_hash_password(password_stdin)?,
    }
    Ok(())
}

/// H-2 修复（v9 复审）：安全获取密码
/// 优先级：BINGXI_ADMIN_PASSWORD 环境变量 > --password-stdin（stdin 读取）
/// 移除了原 --password 命令行参数（会出现在 ps / /proc/<pid>/cmdline 中）
fn read_password(from_stdin: bool) -> Result<String, String> {
    // 1. 优先从环境变量读取
    if let Ok(p) = std::env::var("BINGXI_ADMIN_PASSWORD") {
        if !p.is_empty() {
            return Ok(p);
        }
    }

    // 2. 从 stdin 读取
    if from_stdin {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| format!("读取 stdin 失败: {}", e))?;
        let password = buf.trim_end_matches(['\n', '\r']).to_string();
        if password.is_empty() {
            return Err("stdin 输入为空".to_string());
        }
        return Ok(password);
    }

    // 3. 都没提供，报错提示
    Err(
        "未提供密码。请使用 --password-stdin 从 stdin 读取，或设置 BINGXI_ADMIN_PASSWORD 环境变量。\n\
         示例：\n  \
         echo '密码' | bingxi admin hash-password --password-stdin\n  \
         或：export BINGXI_ADMIN_PASSWORD='密码' && bingxi admin hash-password"
            .to_string(),
    )
}

fn cmd_hash_password(password_stdin: bool) -> Result<(), String> {
    let password = read_password(password_stdin)?;

    // V15 P2 修复（B03-P2-9）：使用 Rust 原生 argon2 crate 替换 python3 子进程，
    // 消除外部 python3 依赖 + 子进程通信开销 + stdin 密码传递风险
    use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};

    let salt = SaltString::generate(&mut OsRng);
    let params = argon2::Params::new(65536, 3, 4, None)
        .map_err(|e| format!("Argon2 参数初始化失败: {}", e))?;
    let argon2 = argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("生成 Argon2 哈希失败: {}", e))?;

    // V15 P2 修复（B03-P2-8）：哈希输出到 stderr 而非 stdout，
    // 避免 stdout 被 CI/日志系统捕获导致哈希泄露；stdout 仅输出操作状态
    eprintln!("=== 密码哈希生成成功 ===");
    eprintln!("Argon2 哈希: {}", password_hash.to_string());
    eprintln!("\n请将上述哈希写入配置文件的 password_hash 字段。");
    println!("OK: 密码哈希已生成（输出到 stderr，请从终端或重定向 stderr 查看）");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// H-2 测试（v9 复审）：未提供密码时返回错误
    #[test]
    fn test_read_password_no_source() {
        std::env::remove_var("BINGXI_ADMIN_PASSWORD");
        let result = read_password(false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("未提供密码"));
    }

    /// H-2 测试（v9 复审）：从环境变量读取密码
    #[test]
    fn test_read_password_from_env() {
        std::env::set_var("BINGXI_ADMIN_PASSWORD", "test_secret_123");
        let result = read_password(false);
        assert_eq!(result.unwrap(), "test_secret_123");
        std::env::remove_var("BINGXI_ADMIN_PASSWORD");
    }

    /// H-2 测试（v9 复审）：环境变量优先于 stdin
    #[test]
    fn test_read_password_env_takes_precedence() {
        std::env::set_var("BINGXI_ADMIN_PASSWORD", "env_password");
        // 即使 from_stdin=true，环境变量也优先
        let result = read_password(true);
        assert_eq!(result.unwrap(), "env_password");
        std::env::remove_var("BINGXI_ADMIN_PASSWORD");
    }

    /// H-2 测试（v9 复审）：空环境变量被忽略
    #[test]
    fn test_read_password_empty_env_ignored() {
        std::env::set_var("BINGXI_ADMIN_PASSWORD", "");
        let result = read_password(false);
        assert!(result.is_err());
        std::env::remove_var("BINGXI_ADMIN_PASSWORD");
    }
}
