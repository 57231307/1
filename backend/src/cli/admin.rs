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
        /// 原始密码
        #[arg(short, long)]
        password: String,
    },
}

/// 管理员子命令入口分发
pub async fn run(cmd: AdminCommand) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        AdminCommand::HashPassword { password } => cmd_hash_password(&password),
    }
    Ok(())
}

fn cmd_hash_password(password: &str) {
    println!("=== 生成密码哈希 ===\n");

    // M3 修复（v8 复审）：通过 stdin 传递密码，避免命令行参数泄露（ps 可见）和字符串拼接注入
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
        Err(e) => {
            println!("[ERROR] 启动 python3 失败: {}", e);
            return;
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(password.as_bytes());
    }

    match child.wait_with_output() {
        Ok(output) if output.status.success() => {
            println!("{}", String::from_utf8_lossy(&output.stdout));
        }
        Ok(output) => {
            println!(
                "[ERROR] 生成失败: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Err(e) => println!("[ERROR] 等待进程失败: {}", e),
    }
}
