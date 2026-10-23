//! 导出取数不得回退到分页入口锁（契约锁族，纯静态源码普查，不连库）
//!
//! 锁定的契约，四条导出链路的静默丢行形态：
//! - backend/src/handlers 下的四个导出函数体内，**不得**直接调用带页宽上限的
//!   对外分页入口，只允许调用语义明确的导出专用取数入口。
//!   供应商导出不得调 .list_suppliers，仓库导出不得调 .list，
//!   色卡发放导出不得调 .list_records_with_data_scope，
//!   固定资产导出不得调 .get_list。
//! - 同一函数体内不得再出现 page_size = Some(10000) 或 page_size: 10000
//!   这类"自称取全部"的大页宽赋值：对外分页入口会把页宽收敛到一页，
//!   这类写法只会把丢行重新变成静默。
//! - 为什么这一形态必须判红。分页入口的页宽上限是对外请求的防护，
//!   它把调用方传入的超大页宽收敛成一页。导出复用它并自称取全部时，
//!   响应与审计都按收敛后的行数报数，用户看到一份行数不足的文件却没有任何
//!   地方告诉他丢了行，历史导出文件也无法从审计反查。
//! - 正确形态，服务层另设导出取数入口，复用同一筛选与归属构造，
//!   把分页那一段换成分批循环取满，并把实际行数与服务端总数一并交回端点记录。
//!   对外分页入口的页宽防护必须原样保留，不得为导出放宽。
//! - 盲区显式处置，不允许静默跳过。判据精确到调用点函数名并做标识符边界判定，
//!   导出入口名与被禁名共享前缀时不误伤；注释与字符串字面量里的反例不参与匹配；
//!   调用名与左括号之间的折行与空白不得绕过判据。
//! - 检测力自证，fixture 与真实扫描共用同一入口。被禁形态注入必被抓，
//!   折行形态必被抓，合规形态不得误伤，注释内反例不得误伤。
//!   四个导出函数必须全部被解析到，解析整体错位必须在这里变红而不是
//!   让违规数假装为零。
//!
//! 调用方：静态源码普查，CI 侧随集成测试执行。
//! 入参：仓库内真实源文件与内置文本夹具。
//! 传给谁：判据结果只用于断言，不落库。
//! 存什么：不存。
//! 存哪里：不适用。

use std::path::{Path, PathBuf};

/// 读取并统一行尾：本仓工作区可能是 CRLF 而索引区是 LF，判据只看内容
fn read_source(p: &Path) -> String {
    std::fs::read_to_string(p)
        .unwrap_or_else(|e| panic!("扫描锁夹具：读取 {} 失败：{e}", p.display()))
        .replace('\r', "")
}

/// 后端源码根目录
fn backend_src() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("src");
    p
}

/// 把注释与字符串、字符字面量的内容替换为空格，保留字节数与位置，
/// 使后续的下标切片与括号配平仍然对齐
fn blank_out_noise(src: &[u8]) -> Vec<u8> {
    let mut out = src.to_vec();
    let mut i = 0usize;
    let n = src.len();
    // 状态：普通代码、行注释、块注释、字符串、字符字面量；块注释可嵌套
    let mut block_depth = 0usize;
    while i < n {
        match src[i] {
            b'/' if i + 1 < n && src[i + 1] == b'/' && block_depth == 0 => {
                while i < n && src[i] != b'\n' {
                    out[i] = b' ';
                    i += 1;
                }
            }
            b'/' if i + 1 < n && src[i + 1] == b'*' => {
                out[i] = b' ';
                out[i + 1] = b' ';
                block_depth += 1;
                i += 2;
            }
            b'*' if block_depth > 0 && i + 1 < n && src[i + 1] == b'/' => {
                out[i] = b' ';
                out[i + 1] = b' ';
                block_depth -= 1;
                i += 2;
            }
            b'"' => {
                out[i] = b' ';
                i += 1;
                while i < n {
                    if src[i] == b'\\' && i + 1 < n {
                        out[i] = b' ';
                        out[i + 1] = b' ';
                        i += 2;
                        continue;
                    }
                    if src[i] == b'"' {
                        out[i] = b' ';
                        i += 1;
                        break;
                    }
                    out[i] = b' ';
                    i += 1;
                }
            }
            b'\'' => {
                // 生命周期撇号与字符字面量的区分：字符字面量以 '\' 后紧跟非空白、
                // 且再后一个是 '\'' 或 '\\' 判定；不满足则视为生命周期，原样保留
                let looks_like_char = i + 2 < n
                    && !src[i + 1].is_ascii_whitespace()
                    && (src[i + 2] == b'\'' || src[i + 2] == b'\\');
                if !looks_like_char {
                    i += 1;
                    continue;
                }
                out[i] = b' ';
                i += 1;
                while i < n {
                    if src[i] == b'\\' && i + 1 < n {
                        out[i] = b' ';
                        out[i + 1] = b' ';
                        i += 2;
                        continue;
                    }
                    if src[i] == b'\'' {
                        out[i] = b' ';
                        i += 1;
                        break;
                    }
                    out[i] = b' ';
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    out
}

/// 在已去噪的字节序列里按函数名定位该异步函数的函数体，返回原文切片
///
/// 找到返回体内容（不含最外层花括号），找不到返回空
fn fn_body(raw: &str, name: &str) -> String {
    let bytes = raw.as_bytes();
    let clean = blank_out_noise(bytes);
    let sig = format!("async fn {name}");
    let search_in = &clean[..];
    let mut start = match find_sub(search_in, sig.as_bytes()) {
        Some(p) => p,
        None => return String::new(),
    };
    // 要求签名前的紧邻字节不是标识符字符，避免命中同前缀的另一个函数
    loop {
        if start == 0 || !is_ident_byte(clean[start - 1]) {
            break;
        }
        start = match find_sub(&search_in[start + 1..], sig.as_bytes()) {
            Some(off) => start + 1 + off,
            None => return String::new(),
        };
    }
    // 找到函数体左花括号
    let mut i = start + sig.len();
    while i < bytes.len() && clean[i] != b'{' {
        i += 1;
    }
    if i >= bytes.len() {
        return String::new();
    }
    let mut depth = 0usize;
    let body_from = i + 1;
    let mut j = i;
    while j < bytes.len() {
        match clean[j] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
        j += 1;
    }
    if depth != 0 {
        return String::new();
    }
    raw[body_from..j].to_string()
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn find_sub(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

/// 把连续空白压成单个空格，使调用名与左括号之间的折行不能作为逃逸手段
fn normalize_ws(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut in_ws = false;
    for ch in src.chars() {
        if ch.is_whitespace() {
            if !in_ws {
                out.push(' ');
                in_ws = true;
            }
        } else {
            out.push(ch);
            in_ws = false;
        }
    }
    out
}

/// 判据：函数体内是否存在被禁调用名的调用点
///
/// 被禁调用名以点号开头且不含左括号；命中要求其后第一个非空白字符是左括号，
/// 因此标识符边界自然成立：被禁名 .list_suppliers 命中 .list_suppliers 加左括号，
/// 而不命中共享前缀的 .list_suppliers_for_export；
/// 调用名与左括号之间的折行与空白不构成逃逸手段
fn has_call(body_clean: &str, forbidden: &str) -> bool {
    let norm = normalize_ws(body_clean);
    let bytes = norm.as_bytes();
    let mut from = 0usize;
    while let Some(off) = find_sub(&bytes[from..], forbidden.as_bytes()) {
        let at = from + off;
        let after = at + forbidden.len();
        let mut probe = after;
        while probe < bytes.len() && bytes[probe].is_ascii_whitespace() {
            probe += 1;
        }
        // 被禁名以点号开头，点号本身就是标识符边界，无需再看左侧
        if bytes.get(probe) == Some(&b'(') {
            return true;
        }
        from = after;
    }
    false
}

/// 判据：函数体内是否出现"自称取全部"的超大页宽赋值
fn has_huge_page_size(body_clean: &str) -> bool {
    let norm = normalize_ws(body_clean);
    let patterns = [
        "page_size = Some(10000)",
        "page_size: 10000",
        "page_size = Some(10_000)",
        "page_size: 10_000",
    ];
    patterns
        .iter()
        .any(|p| find_sub(norm.as_bytes(), p.as_bytes()).is_some())
}

/// 对单个源文件跑普查：返回该导出函数的全部违规描述，空表示合规
fn scan_export_fn(path: &Path, fn_name: &str, forbidden_calls: &[&str]) -> Vec<String> {
    let raw = read_source(path);
    let bytes = raw.as_bytes();
    let body = fn_body(&raw, fn_name);
    assert!(
        !body.is_empty(),
        "扫描锁解析失败：{} 里没取到 {fn_name} 的函数体 —— \
         解析错位必须点名变红，不允许让违规数假装为零",
        path.display()
    );
    // 函数体在原文中的起点，用于把违规定位回文件行号
    let body_start = match find_sub(bytes, body.as_bytes()) {
        Some(p) => p,
        None => 0,
    };
    let line_of = |offset: usize| raw[..offset].matches('\n').count() + 1;

    let clean_body = String::from_utf8(blank_out_noise(body.as_bytes())).expect("体内容必为 utf8");
    let mut hits = Vec::new();
    for call in forbidden_calls {
        if has_call(&clean_body, call) {
            hits.push(format!(
                "{} 行约 {}：{fn_name} 直接调用了带页宽上限的对外分页入口 {call}，导出必须改走导出专用取数入口",
                path.display(),
                line_of(body_start)
            ));
        }
    }
    if has_huge_page_size(&clean_body) {
        hits.push(format!(
            "{} 行约 {}：{fn_name} 仍在设置超大页宽，该写法在对外分页入口下只会静默丢行",
            path.display(),
            line_of(body_start)
        ));
    }
    hits
}

/// 四条导出链路各自的源文件、导出函数名与被禁调用名
fn export_targets() -> Vec<(PathBuf, &'static str, Vec<&'static str>)> {
    let root = backend_src();
    vec![
        (
            root.join("handlers/supplier_handler.rs"),
            "export_suppliers",
            vec![".list_suppliers"],
        ),
        (
            root.join("handlers/warehouse_handler.rs"),
            "export_warehouses",
            vec![".list"],
        ),
        (
            root.join("handlers/color_card/issue.rs"),
            "export_issue_records",
            vec![".list_records_with_data_scope"],
        ),
        (
            root.join("handlers/fixed_asset_handler.rs"),
            "export_assets",
            vec![".get_list"],
        ),
    ]
}

/// 真实源码普查：四条导出链路都必须合规
#[test]
fn export_fns_do_not_call_clamped_page_entries() {
    let mut report = Vec::new();
    for (path, fn_name, forbidden) in export_targets() {
        assert!(
            path.exists(),
            "扫描锁夹具：源文件 {} 不存在，判据失去锚点",
            path.display()
        );
        report.extend(scan_export_fn(&path, fn_name, &forbidden));
    }
    assert!(
        report.is_empty(),
        "导出静默丢行回潮，命中 {} 处：\n{}",
        report.len(),
        report.join("\n")
    );
}

/// 检测力自证：被禁形态必须被抓，合规形态不得误伤
#[test]
fn scanner_detects_forbidden_and_tolerates_compliant() {
    // 一、真实违规形态必被抓
    let violating = "
        let result = service.list_suppliers(params.clone(), None).await?;
        Ok(result.items)
    ";
    let clean = String::from_utf8(blank_out_noise(violating.as_bytes())).unwrap();
    assert!(
        has_call(&clean, ".list_suppliers"),
        "夹具失效：直接调用对外分页入口必须被判红"
    );

    // 二、折行逃逸必被抓（调用名与左括号之间换行）
    let wrapped = "
        let result = service
            .list_suppliers
            (params, None)
            .await?;
    ";
    let clean = String::from_utf8(blank_out_noise(wrapped.as_bytes())).unwrap();
    assert!(
        has_call(&clean, ".list_suppliers"),
        "夹具失效：调用名与左括号之间的折行不得成为逃逸手段"
    );

    // 三、合规的导出入口不得误伤（与被禁名共享前缀）
    let compliant = "
        let (rows, total) = service.list_suppliers_for_export(params, None).await?;
    ";
    let clean = String::from_utf8(blank_out_noise(compliant.as_bytes())).unwrap();
    assert!(
        !has_call(&clean, ".list_suppliers"),
        "夹具失效：导出专用入口与对外分页入口共享前缀，不得误伤"
    );

    // 四、注释与字符串字面量里的反例不得误伤
    let noise_only = "
        // 旧实现曾写 service.list_suppliers(params) 一把取完
        let note = \"service.list_suppliers(params)\";
    ";
    let clean = String::from_utf8(blank_out_noise(noise_only.as_bytes())).unwrap();
    assert!(
        !has_call(&clean, ".list_suppliers"),
        "夹具失效：注释与字符串里的反例不应参与匹配"
    );

    // 五、超大页宽赋值必被抓，含结构体字面量形态
    let huge = "
        export_query.page_size = Some(10000);
    ";
    let clean = String::from_utf8(blank_out_noise(huge.as_bytes())).unwrap();
    assert!(
        has_huge_page_size(&clean),
        "夹具失效：Option 形态的超大页宽必须被判红"
    );
    let huge_struct = "
        let params = AssetQueryParams { page: 1, page_size: 10000 };
    ";
    let clean = String::from_utf8(blank_out_noise(huge_struct.as_bytes())).unwrap();
    assert!(
        has_huge_page_size(&clean),
        "夹具失效：结构体字面量形态的超大页宽必须被判红"
    );
    // 六、正常页宽不得误伤
    let sane = "
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100);
    ";
    let clean = String::from_utf8(blank_out_noise(sane.as_bytes())).unwrap();
    assert!(
        !has_huge_page_size(&clean),
        "夹具失效：对外分页入口的既有页宽防护不应被判红"
    );
}

/// 解析自证：四个导出函数的体都必须被取到且体长合理
#[test]
fn all_four_export_bodies_are_resolved() {
    let mut resolved = 0usize;
    for (path, fn_name, _) in export_targets() {
        let raw = read_source(&path);
        let body = fn_body(&raw, fn_name);
        assert!(
            !body.is_empty(),
            "扫描锁解析失败：{} 未取到 {fn_name} 函数体",
            path.display()
        );
        assert!(
            body.len() > 200,
            "扫描锁解析可疑：{fn_name} 体长 {} 过短，可能取错范围",
            body.len()
        );
        resolved += 1;
    }
    assert_eq!(
        resolved, 4,
        "四条导出链路必须全部被解析到，缺一即判据失去锚点"
    );
}
