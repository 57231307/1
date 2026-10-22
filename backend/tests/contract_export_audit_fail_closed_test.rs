//! 敏感导出留痕 fail-closed 静态契约锁（不连库，纯 #[test]）。
//!
//! 锁定契约：`ExportApprovalService::record_download` 是敏感导出令牌消费的唯一留痕写口，
//! 每个导出 handler 都必须把它的错误透传给调用方（`.await?;`）。此前十三处写成
//! `let _ = ...record_download(...).await;`，注释自称 fail-closed 而实际把落库失败整条吞掉：
//! 留痕没写成、文件照样交付、同一令牌可被重复消费，审计面与授权面从此不同源。
//!
//! 判据两条：
//! - 吞错形态零命中：`src/handlers` 下不得再出现以 `let _ = ` 开头且体内含
//!   `.record_download(` 的语句；
//! - 每个调用点都必须透传：同一语句块内出现 `.record_download(` 且以 `.await?;` 收尾，
//!   命中数必须等于 `.record_download(` 的调用点总数。
//!
//! 检测力自证：把吞错形态与 `.await;` 未透传形态各做成夹具喂给同一判据，两者都必须被点名，
//! 否则本锁是假锁。

use std::path::{Path, PathBuf};

fn backend_src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("读取 {} 失败，锁前提失效: {e}", path.display()))
        .replace('\r', "")
}

/// 收集一处源码里的吞错点与未透传点，返回问题描述清单（空即合规）。
fn violations_in(src: &str, origin: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let lines: Vec<&str> = src.lines().collect();
    for (idx, line) in lines.iter().enumerate() {
        let t = line.trim();
        if !t.starts_with("let _ = ") {
            continue;
        }
        // 语句体：从本行到下一个以 `;` 收尾的行为止
        let mut end = idx;
        while end < lines.len() && !lines[end].trim().ends_with(';') {
            end += 1;
        }
        let stmt: Vec<&str> = lines[idx..=end.min(lines.len() - 1)].to_vec();
        if stmt.iter().any(|l| l.contains(".record_download(")) {
            problems.push(format!(
                "{origin}:{} 吞掉导出留痕写库错误（let _ = 形态），令牌消费未落库也照样交付文件",
                idx + 1
            ));
        }
    }
    // 每个 record_download 调用点都必须以 `.await?;` 收尾
    for (idx, line) in lines.iter().enumerate() {
        if !line.trim().contains(".record_download(") {
            continue;
        }
        let mut end = idx;
        while end < lines.len() && !lines[end].trim().ends_with(';') {
            end += 1;
        }
        let tail = lines[end.min(lines.len() - 1)].trim();
        if tail != ".await?;" && !tail.ends_with(".await?;") {
            problems.push(format!(
                "{origin}:{} 导出留痕调用点未把错误透传给调用方（收尾是 {tail:?}）",
                idx + 1
            ));
        }
    }
    problems
}

fn walk_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                walk_rs(&p, out);
            } else if p.extension().and_then(|e| e.to_str()) == Some("rs") {
                out.push(p);
            }
        }
    }
}

#[test]
fn no_export_handler_swallows_record_download_error() {
    let handlers = backend_src().join("handlers");
    let mut files = Vec::new();
    walk_rs(&handlers, &mut files);
    assert!(!files.is_empty(), "handlers 目录取不到源码，锁前提失效");
    let mut problems = Vec::new();
    let mut call_sites = 0usize;
    for f in &files {
        let src = read(f);
        call_sites += src.matches(".record_download(").count();
        let name = f.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        problems.extend(violations_in(&src, name));
    }
    // 正向非空控制：全仓确有该写口在用，否则本锁空转
    assert!(
        call_sites > 0,
        "handlers 下没有任何 record_download 调用点，本锁失去被锁对象"
    );
    assert!(
        problems.is_empty(),
        "敏感导出留痕未 fail-closed，共 {call_sites} 个调用点中发现问题: {problems:?}"
    );
}

#[test]
fn checker_detects_injected_swallow_and_non_propagating_shapes() {
    // 夹具一，吞错形态（修复前的真实形状）
    let swallowed = "\
    // 敏感导出 fail-closed：记录令牌消费
    let _ = crate::services::export_approval_service::ExportApprovalService::new(state.db.clone())
        .record_download(approval.id, file_name.clone(), count as i64, String::new())
        .await;
";
    let got = violations_in(swallowed, "fixture.rs");
    assert!(
        got.iter().any(|p| p.contains("吞掉导出留痕写库错误")),
        "吞错夹具未被检出，接线锁是假锁: {got:?}"
    );

    // 夹具二，留痕写了但错误没透传（收尾 .await;）
    let not_propagated = "\
    crate::services::export_approval_service::ExportApprovalService::new(state.db.clone())
        .record_download(approval.id, file_name.clone(), count as i64, String::new())
        .await;
";
    let got2 = violations_in(not_propagated, "fixture2.rs");
    assert!(
        got2.iter().any(|p| p.contains("未把错误透传")),
        "未透传夹具未被检出: {got2:?}"
    );

    // 反向对照：合规形状零命中，证明上述点名不是正则误报
    let compliant = "\
    crate::services::export_approval_service::ExportApprovalService::new(state.db.clone())
        .record_download(approval.id, file_name.clone(), count as i64, String::new())
        .await?;
";
    assert!(
        violations_in(compliant, "fixture3.rs").is_empty(),
        "合规形状被误报: {:?}",
        violations_in(compliant, "fixture3.rs")
    );
}
