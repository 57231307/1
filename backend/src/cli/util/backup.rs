//! 备份与恢复子命令实现：Backup / Restore

use super::{get_backup_dir, get_install_dir, require_env, run_cmd, timestamp};
use std::fs;
use std::path::Path;

/// 校验解压后的所有文件路径都在指定目录范围内，防止 Tar Slip 路径穿越攻击
fn validate_extracted_paths(base_dir: &str) -> Result<(), String> {
    let base_canonical = fs::canonicalize(base_dir)
        .map_err(|e| format!("无法解析基准目录 {}: {}", base_dir, e))?;
    validate_dir_recursive(&base_canonical, &base_canonical)
}

/// 递归校验目录下所有文件路径都在基准目录范围内
fn validate_dir_recursive(dir: &Path, base: &Path) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| format!("读取目录失败: {}", e))? {
        let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
        let path = entry.path();
        // canonicalize 解析符号链接，防止通过符号链接逃逸
        let canonical = fs::canonicalize(&path)
            .map_err(|e| format!("解析路径失败 {:?}: {}", path, e))?;
        if !canonical.starts_with(base) {
            return Err(format!(
                "检测到路径穿越攻击：文件 {:?} 不在安全目录范围内",
                canonical
            ));
        }
        // 如果是目录，递归校验
        if canonical.is_dir() {
            validate_dir_recursive(&canonical, base)?;
        }
    }
    Ok(())
}

pub(super) fn cmd_backup(backup_type: &str) {
    let ts = timestamp();
    let backup_dir = format!("{}/{}", get_backup_dir(), ts);

    println!("=== 开始备份 ===\n");
    println!("备份目录: {}", backup_dir);

    // 批次 92 P3-8：原 `let _ = run_cmd(...)` 静默吞错，关键路径失败应中止
    if let Err(e) = run_cmd("mkdir", &["-p", &backup_dir]) {
        println!("[ERROR] 创建备份目录失败，终止备份: {}", e);
        return;
    }

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
        let service_file = format!("/etc/systemd/system/{}.service", super::SERVICE_NAME);

        // 批次 92 P3-8：cp 失败应记录错误（不中止，尽量备份剩余文件）
        if let Err(e) = run_cmd("cp", &["-r", &config_dir, &backup_dir]) {
            println!("[ERROR] 备份 config.yaml 失败: {}", e);
        }
        if let Err(e) = run_cmd("cp", &["-r", env_file, &backup_dir]) {
            println!("[ERROR] 备份 .env 失败: {}", e);
        }
        if let Err(e) = run_cmd("cp", &["-r", &service_file, &backup_dir]) {
            println!("[ERROR] 备份 service 文件失败: {}", e);
        }

        println!("[OK] 配置文件备份完成");
    }

    // 压缩
    println!("\n压缩备份...");
    let tar_file = format!("{}/backup_{}.tar.gz", get_backup_dir(), ts);
    if let Err(e) = run_cmd(
        "tar",
        &["-czf", &tar_file, "-C", &get_backup_dir(), &ts.to_string()],
    ) {
        println!("[ERROR] 压缩失败: {}", e);
    } else {
        // 规则 12 合规：设置备份文件权限为 0o600（仅所有者可读），
        // 防止备份中的 .env（含数据库密码等敏感信息）被其他用户读取
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Err(e) = fs::set_permissions(&tar_file, fs::Permissions::from_mode(0o600)) {
                println!("[WARN] 设置备份文件权限失败（可忽略）: {}", e);
            }
        }
    }
    // 清理临时目录（非关键路径，失败仅告警）
    if let Err(e) = run_cmd("rm", &["-rf", &backup_dir]) {
        println!("[WARN] 清理临时备份目录失败（可忽略）: {}", e);
    }

    println!("\n[OK] 备份完成: {}", tar_file);
}

pub(super) fn cmd_restore(file: &str) {
    println!("=== 恢复数据 ===\n");
    println!("备份文件: {}", file);

    if !std::path::Path::new(file).exists() {
        println!("[ERROR] 文件不存在: {}", file);
        return;
    }

    let temp_dir = "/tmp/bingxi_restore";
    // 清理旧临时目录（非关键路径）
    if let Err(e) = run_cmd("rm", &["-rf", temp_dir]) {
        println!("[WARN] 清理旧临时目录失败（可忽略）: {}", e);
    }
    // 创建临时目录（关键路径）
    if let Err(e) = run_cmd("mkdir", &["-p", temp_dir]) {
        println!("[ERROR] 创建临时目录失败，终止恢复: {}", e);
        return;
    }

    println!("解压备份...");
    if let Err(e) = run_cmd("tar", &["-xzf", file, "-C", temp_dir]) {
        println!("[ERROR] 解压失败: {}", e);
        return;
    }

    // 规则 12 合规：解压后校验所有文件路径在安全目录范围内，防止 Tar Slip 路径穿越攻击
    if let Err(e) = validate_extracted_paths(temp_dir) {
        println!("[ERROR] 安全校验失败，终止恢复: {}", e);
        // 清理可能包含恶意文件的临时目录
        let _ = run_cmd("rm", &["-rf", temp_dir]);
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
            // 批次 92 P3-8：cp 失败应记录错误
            if let Err(e) = run_cmd("cp", &[&src, &dst]) {
                println!("[ERROR] 恢复 {} 失败: {}", name, e);
            } else {
                println!("[OK] 恢复: {}", name);
            }
        }
    }

    // 清理临时目录（非关键路径）
    if let Err(e) = run_cmd("rm", &["-rf", temp_dir]) {
        println!("[WARN] 清理临时目录失败（可忽略）: {}", e);
    }

    println!("\n[OK] 恢复完成，请重启服务: bingxi restart");
}
