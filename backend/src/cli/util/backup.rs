//! 备份与恢复子命令实现：Backup / Restore

use super::{get_backup_dir, get_install_dir, require_env, run_cmd, timestamp};

pub(super) fn cmd_backup(backup_type: &str) {
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
        let service_file = format!("/etc/systemd/system/{}.service", super::SERVICE_NAME);

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

pub(super) fn cmd_restore(file: &str) {
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
