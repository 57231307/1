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
    println!("=== 生成密码哈希 ===\n");

    let password = read_password(password_stdin)?;

    // M3 修复（v8 复审）：通过 stdin 传递密码给 python，避免命令行参数泄露和字符串拼接注入
    let python_code = r#"
import sys, hashlib, base64, os
password = sys.stdin.read()
try:
    from argon2 import PasswordHasher
    ph = PasswordHasher()
    hash = ph.hash(password)
    print("Argon2 哈希:", hash)
except ImportError:
    salt = os.urandom(32)
    hash = hashlib.pbkdf2_hmac('sha256', password.encode(), salt, 100000)
    print("PBKDF2 哈希:", base64.b64encode(salt + hash).decode())
"#;

    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = match Command::new("python3")
        .arg("-c")
        .arg(python_code)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => return Err(format!("启动 python3 失败: {}", e)),
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(password.as_bytes());
    }

    match child.wait_with_output() {
        Ok(output) if output.status.success() => {
            println!("{}", String::from_utf8_lossy(&output.stdout));
            Ok(())
        }
        Ok(output) => Err(format!(
            "生成失败: {}",
            String::from_utf8_lossy(&output.stderr)
        )),
        Err(e) => Err(format!("等待进程失败: {}", e)),
    }
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
