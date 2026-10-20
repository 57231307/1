//! span 守卫跨 await 持有禁止锁（契约锁族，纯静态源码普查，不连库）
//!
//! 锁定的契约，仪表盘等端点偶发 500 的根因形态：
//! - `backend/src/**` 的**异步区域**（`async fn` 函数体，以及同步函数里的
//!   `async`/`async move` 块）内，以 span 为接收者的同步 `enter()`/`entered()` 守卫，
//!   其后**不得**再出现 `.await`。
//! - 为什么这一形态必须判红。`Entered` 只借出 `Span` 的引用，因而**是 `Send`**，
//!   axum 与 hyper 的 Send 约束拦不住它。守卫的 push 发生在让出前的 worker A，任务恢复
//!   后 drop 触发的 exit 落在 worker B，B 的线程本地 span 栈里找不回该 id 即空转，
//!   A 的栈上永久残留该 id。该 span 关闭后，下一个被调度到 A 的请求创建子 span 时把这个
//!   失效 id 当作父级去 clone，命中 tracing-subscriber 分片 registry 的引用计数断言
//!   （tried to clone a span that already closed）而 panic，再被内层 catch_panic 层
//!   转成 500。命中的是哪个端点、哪个角色只看线程调度，与业务口径无关。
//! - 正确形态，把 span 交给 `Instrument` 绑定 future（或由 `#[instrument]` 展开），
//!   `Instrumented` 在单次 poll 内成对进出 span，让出时线程栈上不留守卫。
//! - 盲区显式处置，不允许静默跳过。异步区域内**接收者名字不含 span** 的 `enter()`
//!   若其后也有 await，一律进待核清单判红，防止改个变量名即绕过判据。
//!   异步区域内 enter 之后**没有** await 的形态不判红，因为守卫不可能被持有跨过它之前
//!   的让出点。判据是位置保守的，enter 之后再手工 drop 守卫仍判红，文本层区分不了它
//!   与真违规，处置方式是改用 in_scope 闭包或 instrument 绑定 future（两种合规逃生
//!   通道都在自测里钉着）。
//! - 检测力自证，fixture 与真实扫描共用 `scan_source` 同一入口。被禁形态注入必被抓，
//!   rustfmt 折行不得绕过，中文注释文件不得索引错位而漏判，注释与字符串字面量里的
//!   反例不得误伤。异步区域解析数量不得低于地板，解析整体错位必须在这里变红而不是
//!   让违规数假装为 0。

use std::path::{Path, PathBuf};

/// 读取并统一行尾，本仓工作区可能是 CRLF 而索引区是 LF，判据只看内容
fn read_source(p: &Path) -> String {
    std::fs::read_to_string(p)
        .unwrap_or_else(|e| panic!("扫描锁夹具：读取 {} 失败：{e}", p.display()))
        .replace('\r', "")
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("扫描锁夹具：目录 {} 读取失败：{e}", dir.display()))
    {
        let path = entry.expect("目录项读取").path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// 相对路径，POSIX 分隔符
fn rel_path(root: &Path, f: &Path) -> String {
    f.strip_prefix(root)
        .unwrap_or(f)
        .display()
        .to_string()
        .replace('\\', "/")
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// `bytes[i]` 处是 `\` 开头的转义序列时返回其后第一个字节的下标
fn skip_escape(bytes: &[u8], i: usize) -> Option<usize> {
    if bytes.get(i) != Some(&b'\\') {
        return Some(i);
    }
    let first = i + 1;
    match bytes.get(first)? {
        // `\u{...}` 需扫到右花括号，其余转义（`\n` `\\` `\'` `\xHH`）按两字节消费
        b'u' => {
            let open = bytes[first + 1..].iter().position(|b| *b == b'{')?;
            let close = bytes[first + 1 + open + 1..]
                .iter()
                .position(|b| *b == b'}')?;
            Some(first + 1 + open + 1 + close + 1)
        }
        b'x' => Some(first + 3),
        _ => Some(first + 1),
    }
}

/// 把行注释、块注释、字符串与原始字符串的**内容**剔除，换行一律保留，
/// 于是结果与原文逐行对齐，任何偏移都能由 `'\n'` 计数还原成真实行号。
///
/// 全程按**字节索引**推进，与 `str::find` 的返回同一坐标系。判据字符
/// （`/` `*` `"` `'` `{` `}` `(` `)` `.` 与标识符）全是 ASCII，UTF-8 多字节序列的
/// 后续字节恒 ≥0x80，不可能与它们撞码，因此字节扫描既正确又不会在中文注释处错位。
fn strip_comments_and_literals(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        let next = bytes.get(i + 1).copied();
        // 行注释，消到行尾（换行保留）
        if b == b'/' && next == Some(b'/') {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        // 块注释，Rust 允许嵌套，内部换行保留
        if b == b'/' && next == Some(b'*') {
            i += 2;
            let mut depth = 1usize;
            while i < bytes.len() && depth > 0 {
                match bytes[i] {
                    b'\n' => {
                        out.push(b'\n');
                        i += 1;
                    }
                    b'/' if bytes.get(i + 1) == Some(&b'*') => {
                        depth += 1;
                        i += 2;
                    }
                    b'*' if bytes.get(i + 1) == Some(&b'/') => {
                        depth -= 1;
                        i += 2;
                    }
                    _ => i += 1,
                }
            }
            continue;
        }
        // 原始字符串 r"…"、r#"…"##，内容整体剔除（换行保留以维持行号）
        if b == b'r' && !(i > 0 && is_ident_byte(bytes[i - 1])) {
            if let Some(end) = raw_string_end(bytes, i) {
                out.extend(std::iter::repeat_n(b'\n', count_newlines(&bytes[i..end])));
                i = end;
                continue;
            }
        }
        // 普通字符串，内容剔除，转义引号不闭串
        if b == b'"' {
            i += 1;
            while i < bytes.len() {
                match bytes[i] {
                    // 行末续行反斜杠，`\` + 换行必须保住换行，否则后续所有行号整体前移
                    b'\\' if bytes.get(i + 1) == Some(&b'\n') => {
                        out.push(b'\n');
                        i += 2;
                    }
                    b'\\' => i = skip_escape(bytes, i).unwrap_or(bytes.len()),
                    b'"' => {
                        i += 1;
                        break;
                    }
                    b'\n' => {
                        // 多行字面量里的裸换行，同样保住行号对齐
                        out.push(b'\n');
                        i += 1;
                    }
                    _ => i += 1,
                }
            }
            continue;
        }
        // 字符字面量整体剔除（其中的 `"` 不得误开字符串状态），生命周期原样保留
        if b == b'\'' {
            if let Some(end) = char_literal_end(bytes, i) {
                i = end;
                continue;
            }
        }
        out.push(b);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// 原始字符串起始于 `bytes[i] == b'r'` 时返回其结束后的下标，否则 None
fn raw_string_end(bytes: &[u8], i: usize) -> Option<usize> {
    let mut j = i + 1;
    let mut hashes = 0usize;
    while bytes.get(j) == Some(&b'#') {
        hashes += 1;
        j += 1;
    }
    if bytes.get(j) != Some(&b'"') {
        return None;
    }
    j += 1;
    while j < bytes.len() {
        if bytes[j] == b'"' && bytes.get(j + 1) == Some(&b'#') {
            let mut k = j + 1;
            let mut matched = 0usize;
            while matched < hashes && bytes.get(k) == Some(&b'#') {
                matched += 1;
                k += 1;
            }
            if matched == hashes {
                return Some(k);
            }
        }
        j += 1;
    }
    None
}

/// `bytes[i]` 处是 `'` 时尝试按字符字面量消费，返回闭引号之后的下标；
/// 不是字符字面量（是生命周期）则 None，交调用方原样输出
fn char_literal_end(bytes: &[u8], i: usize) -> Option<usize> {
    if bytes.get(i) != Some(&b'\'') {
        return None;
    }
    let j = i + 1;
    let after = match bytes.get(j)? {
        b'\\' => skip_escape(bytes, j)?,
        c if *c != b'\'' && *c != b'\n' => j + 1,
        _ => return None,
    };
    if bytes.get(after) == Some(&b'\'') {
        Some(after + 1)
    } else {
        None
    }
}

fn count_newlines(bytes: &[u8]) -> usize {
    let mut n = 0usize;
    for b in bytes {
        if *b == b'\n' {
            n += 1;
        }
    }
    n
}

/// 去掉除换行以外的全部空白，行号仍可由 `'\n'` 计数还原，
/// 判据因此不被 rustfmt 折行绕过
fn compact(src: &str) -> String {
    src.chars()
        .filter(|c| !c.is_ascii_whitespace() || *c == '\n')
        .collect()
}

/// 归一化文本中某字节偏移所在的原文件行号
fn line_of(normalized: &str, byte_idx: usize) -> usize {
    count_newlines(&normalized.as_bytes()[..byte_idx.min(normalized.len())]) + 1
}

/// 一个异步区域，`async fn` 的函数体或同步代码里的 `async`/`async move` 块
struct AsyncRegion {
    /// 区域标签，`async fn` 用函数名，块用 async 块
    label: String,
    /// 区域体所在的首行（`{` 那一行）行号
    body_first_line: usize,
    /// 已去空白（保留换行）的区域体
    body: String,
}

/// 从 `bytes[idx]` 处的开括号起做配平，返回配对闭括号**之后**的下标
fn balanced_end(bytes: &[u8], idx: usize) -> Option<usize> {
    let opener = *bytes.get(idx)?;
    let closer = match opener {
        b'(' => b')',
        b'{' => b'}',
        _ => return None,
    };
    let mut depth = 0usize;
    let mut i = idx;
    while i < bytes.len() {
        if bytes[i] == opener {
            depth += 1;
        } else if bytes[i] == closer {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(i + 1);
            }
        }
        i += 1;
    }
    None
}

/// 枚举归一化文本里的异步区域。发出一个区域后游标越过其整个函数体，
/// 区域内的嵌套 async 块由父区域覆盖，不重复计入（违规因此只报一次）。
fn async_regions(normalized: &str) -> Vec<AsyncRegion> {
    let bytes = normalized.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 5 <= bytes.len() {
        if !bytes[i..].starts_with(b"async") {
            i += 1;
            continue;
        }
        if i > 0 && is_ident_byte(bytes[i - 1]) {
            i += 1;
            continue;
        }
        let mut j = i + 5;
        while bytes.get(j).is_some_and(|b| b.is_ascii_whitespace()) {
            j += 1;
        }
        if bytes[j..].starts_with(b"move") {
            j += 4;
            while bytes.get(j).is_some_and(|b| b.is_ascii_whitespace()) {
                j += 1;
            }
        }
        if bytes[j..].starts_with(b"fn") {
            let mut k = j + 2;
            while bytes.get(k).is_some_and(|b| b.is_ascii_whitespace()) {
                k += 1;
            }
            let name_start = k;
            while bytes.get(k).is_some_and(|b| is_ident_byte(*b)) {
                k += 1;
            }
            let Some(paren_rel) = bytes[k..].iter().position(|b| *b == b'(') else {
                i = k;
                continue;
            };
            let paren = k + paren_rel;
            let Some(after_params) = balanced_end(bytes, paren) else {
                i = paren + 1;
                continue;
            };
            // 无体声明（trait 里的 `async fn f() -> R;`）在遇到 `{` 前先撞上 `;`，
            // 此时不得把下一个函数的体误当本函数体
            let mut probe = after_params;
            while bytes.get(probe).is_some_and(|b| *b != b'{' && *b != b';') {
                probe += 1;
            }
            if bytes.get(probe) != Some(&b'{') {
                i = probe + 1;
                continue;
            }
            let brace = probe;
            let Some(after_body) = balanced_end(bytes, brace) else {
                i = brace + 1;
                continue;
            };
            out.push(AsyncRegion {
                label: String::from_utf8_lossy(&bytes[name_start..k]).into_owned(),
                body_first_line: line_of(normalized, brace),
                body: compact(&normalized[brace + 1..after_body - 1]),
            });
            i = after_body;
            continue;
        }
        if bytes.get(j) == Some(&b'{') {
            let Some(after_body) = balanced_end(bytes, j) else {
                i = j + 1;
                continue;
            };
            out.push(AsyncRegion {
                label: "async 块".to_string(),
                body_first_line: line_of(normalized, j),
                body: compact(&normalized[j + 1..after_body - 1]),
            });
            i = after_body;
            continue;
        }
        i = j;
    }
    out
}

/// 取 `.enter(` 之前那个点号（`dot_idx` 指向点号）的接收者名字。
/// 覆盖三种写法，`span.enter(`、`root_span(..).enter(`、`span_business!(..).enter(`。
/// 全程字节下标且区间右开，`span` 这类纯标识符接收者必须完整取回，
/// 少一个字符就会让 span 判据静默失效（退化成待核清单）。
fn receiver_label(body: &str, dot_idx: usize) -> String {
    let bytes = body.as_bytes();
    if dot_idx == 0 {
        return String::new();
    }
    let mut end = dot_idx;
    // 区域体是 compact 后的文本，换行为判行号而保留，接收者起点必须先跳过
    // rustfmt 在 `)` 与 `.` 之间插入的换行，否则折行写法会被当成无接收者而漏判
    while end > 0 && bytes[end - 1] == b'\n' {
        end -= 1;
    }
    if end == 0 {
        return String::new();
    }
    if bytes[end - 1] == b')' {
        // 接收者是调用结果，回退到配对开括号，开括号本身不属于名字
        let mut depth = 0usize;
        let mut k = end - 1;
        loop {
            match bytes[k] {
                b')' => depth += 1,
                b'(' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            if k == 0 {
                break;
            }
            k -= 1;
        }
        end = k;
        // 宏调用形态，`!` 也不属于名字
        if end > 0 && bytes[end - 1] == b'!' {
            end -= 1;
        }
    }
    let mut start = end;
    while start > 0 && is_ident_byte(bytes[start - 1]) {
        start -= 1;
    }
    String::from_utf8_lossy(&bytes[start..end]).into_owned()
}

/// 判据本体，真实扫描与 fixture 自测共用同一入口。
///
/// 返回 (违规清单, 非 span 接收者的待核清单, 异步区域数)
fn scan_source(rel: &str, src: &str) -> (Vec<String>, Vec<String>, usize) {
    let normalized = strip_comments_and_literals(src);
    let mut violations = Vec::new();
    let mut unverified = Vec::new();
    let regions = async_regions(&normalized);
    let region_count = regions.len();
    for region in &regions {
        for needle in [".enter(", ".entered("] {
            let mut from = 0usize;
            while let Some(rel_pos) = region.body[from..].find(needle) {
                let pos = from + rel_pos;
                from = pos + 1;
                let dot_idx = pos;
                let receiver = receiver_label(&region.body, dot_idx);
                let line = region.body_first_line + count_newlines(&region.body.as_bytes()[..pos]);
                // 守卫之后同一区域内还有 await，说明守卫被持有跨过让出点
                if !region.body[pos..].contains(".await") {
                    continue;
                }
                let entry = format!("{rel}:{line}");
                if receiver.to_lowercase().contains("span") {
                    violations.push(format!(
                        "{entry} 异步区域 {} 内以 {receiver} 为接收者的同步守卫被持有跨过 await，必须把 span 交给 instrument 绑定 future",
                        region.label
                    ));
                } else {
                    unverified.push(format!(
                        "{entry} 异步区域 {} 内出现接收者 {receiver:?} 的 enter 且其后有 await，本锁按名字认 span，改名绕过的通道必须人工处置",
                        region.label
                    ));
                }
            }
        }
    }
    (violations, unverified, region_count)
}

#[test]
fn src_tree_must_not_hold_span_guard_across_await() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src_root = root.join("src");
    let mut files = Vec::new();
    collect_rs_files(&src_root, &mut files);
    assert!(
        files.len() > 300,
        "扫描锁前提，src 下 .rs 文件数异常（目录指错？）读到 {}",
        files.len()
    );

    let mut violations: Vec<String> = Vec::new();
    let mut unverified: Vec<String> = Vec::new();
    let mut regions = 0usize;
    for f in &files {
        let src = read_source(f);
        let (v, u, n) = scan_source(&rel_path(&root, f), &src);
        violations.extend(v);
        unverified.extend(u);
        regions += n;
    }
    // 检测力地板，异步区域数量骤降说明解析整体错位，扫描会空转成假绿。
    // src 实测 async fn 出现 4458 次（另有 async 块），取远小于实测的保守地板。
    assert!(
        regions >= 3000,
        "扫描锁检测力地板，本轮解析到异步区域 {regions} 个，低于地板 3000，判据整体失效必须显式红"
    );
    assert!(
        unverified.is_empty(),
        "扫描锁盲区清单非空（待人工确认是否为被改名的 span 守卫）：\n{}",
        unverified.join("\n")
    );
    assert!(
        violations.is_empty(),
        "扫描锁，以下异步区域把 span 的同步 enter 守卫持有跨过 await，会让线程本地 span 栈残留已关闭 span 的 id，\
         后续请求在该线程上以它为父级克隆 span 时 panic（偶发 500），必须改为 instrument 绑定 future：\n{}",
        violations.join("\n")
    );
}

// =========================================================
// 检测力双向自证，fixture 走与真实扫描完全相同的代码路径
// =========================================================

/// 被禁形态必须被抓
#[test]
fn gate_selftest_banned_shapes_must_be_caught() {
    let cases = [
        (
            "async fn 内同步 enter 守卫后接 await",
            "async fn handle() {\n    let span = root_span(&ctx, \"GET\", \"/x\");\n    let _guard = span.enter();\n    inner().await;\n}\n",
        ),
        (
            "rustfmt 折行的接收者调用不得绕过",
            "pub async fn mw() {\n    let _g = root_span(\n        &ctx,\n        \"GET\",\n        \"/x\",\n    )\n    .enter();\n    next.run(req).await;\n}\n",
        ),
        (
            "同步函数里的 async 块同样受锁",
            "fn build() {\n    let task = async move {\n        let _e = biz_span.enter();\n        work().await;\n    };\n}\n",
        ),
        (
            "宏创建的 span 句柄 entered 后接 await",
            "async fn job() {\n    let _e = span_business!(\"op\").entered();\n    run().await;\n}\n",
        ),
        (
            "中文文档注释前置不得让索引错位而漏判",
            "/// 追踪上下文中间件，把 span 绑定到下游整段 future。\n/// 中文文档注释前置，用于钉死字节与字符混用的错位盲区。\npub async fn trace_ctx() {\n    let _guard = span.enter();\n    next.run(request).await;\n}\n",
        ),
        (
            "enter 后手工 drop 再 await，文本层与真违规同形，按保守口径判红",
            "async fn handle() {\n    let _g = span.enter();\n    drop(_g);\n    fetch().await;\n}\n",
        ),
        (
            "字符串续行反斜杠不得让行号漂移，判据本身也要抓到守卫行",
            "async fn handle() {\n    let hint = \"第一段\\\n     第二段\";\n    let _g = span.enter();\n    inner().await;\n}\n",
        ),
        (
            "async fn 内的嵌套 async 块由父区域覆盖，违规只报一次",
            "async fn outer() {\n    let t = async move {\n        let _e = span.enter();\n        inner().await;\n    };\n    t.await;\n}\n",
        ),
    ];
    for (desc, src) in cases {
        let (v, _u, n) = scan_source("fixture.rs", src);
        assert_eq!(n, 1, "{desc} 应解析到 1 个异步区域，实际 {n}");
        assert_eq!(v.len(), 1, "{desc} 是被禁形态，必须被抓 1 条，实际 {v:?}");
    }
}

/// 合规形态与噪声上下文不得误伤
#[test]
fn gate_selftest_compliant_and_noise_shapes_must_not_be_caught() {
    let cases = [
        (
            "instrument 绑定 future（本仓正确形态）",
            "async fn handle() {\n    inner().instrument(span).await;\n}\n",
        ),
        (
            "同步函数里的 enter（无 await，进出同一次 poll）",
            "fn sync_helper() {\n    let _g = span.enter();\n    do_sync_work();\n}\n",
        ),
        (
            "await 在 enter 之前（守卫无法被持有跨过它之前的让出点）",
            "async fn handle() {\n    let x = fetch().await;\n    let _g = span.enter();\n    sync_only(x);\n}\n",
        ),
        (
            "in_scope 闭包形态（守卫随调用返回退出）",
            "async fn handle() {\n    span.in_scope(|| record_sync());\n    fetch().await;\n}\n",
        ),
        (
            "注释里的反例不得命中",
            "async fn handle() {\n    // 反例记录，let _guard = span.enter(); inner().await;\n    inner().instrument(span).await;\n}\n",
        ),
        (
            "字符串字面量里的反例不得命中",
            "async fn handle() {\n    let hint = \"span.enter(); 然后 await 就会炸\";\n    log(hint).await;\n}\n",
        ),
        (
            "trait 里的无体 async fn 声明不得吞掉下一个函数的体",
            "trait Svc {\n    async fn a() -> Ret;\n    async fn b() {\n        let _g = span.enter();\n    }\n}\n",
        ),
    ];
    for (desc, src) in cases {
        let (v, u, _n) = scan_source("fixture.rs", src);
        assert!(
            v.is_empty() && u.is_empty(),
            "{desc} 是合规形态，不得判违规或待核，实际 v={v:?} u={u:?}"
        );
    }
}

/// 盲区通道必须显式变红，接收者改名不得绕过
#[test]
fn gate_selftest_renamed_receiver_goes_to_explicit_unverified_list() {
    let src = "async fn handle() {\n    let _g = gate.enter();\n    fetch().await;\n}\n";
    let (v, u, _n) = scan_source("fixture.rs", src);
    assert!(
        v.is_empty(),
        "接收者名字不含 span 时不直接判违规，实际 {v:?}"
    );
    assert_eq!(
        u.len(),
        1,
        "改名后的 span 守卫必须进待核清单（防绕过），实际 {u:?}"
    );
}

/// 违规条目的行号必须真的落在守卫行上，报告不可退化成不可核对的噪声。
/// 第二例专门带一条字符串续行反斜杠，早期实现在这里会把后续行号整体前移两行。
#[test]
fn gate_selftest_reported_line_points_at_the_guard_site() {
    let cases = [
        (
            "//! 模块头一行。\n//! 模块头二行。\npub async fn handle() {\n    let other = 1;\n    let _guard = span.enter();\n    inner().await;\n}\n",
            "span.enter()",
        ),
        (
            "//! 中文模块头。\npub async fn handle() {\n    let hint = \"前半\\\n     后半\";\n    let other = 1;\n    let _guard = span.enter();\n    inner().await;\n}\n",
            "span.enter()",
        ),
    ];
    for (src, marker) in cases {
        let (v, _u, _n) = scan_source("fixture.rs", src);
        assert_eq!(v.len(), 1, "被禁形态必须被抓，实际 {v:?}");
        let reported: usize = v[0]
            .split(':')
            .nth(1)
            .and_then(|s| s.trim().split(' ').next())
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| panic!("违规条目应带 file:line，实际 {}", v[0]));
        let expected = count_newlines(&src.as_bytes()[..src.find(marker).unwrap()]) + 1;
        assert_eq!(
            reported, expected,
            "行号必须落在 enter 守卫所在行（expected {expected}），实际条目 {}",
            v[0]
        );
    }
}

// =========================================================
// 反向防假修，换形态而不是删功能
// =========================================================

/// 追踪中间件仍必须创建 root span 并以 instrument 绑定 future，
/// 该文件内不得再出现同步 enter 守卫，收尾行为不得被顺手删掉
#[test]
fn trace_middleware_must_still_bind_root_span_by_instrument() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mw = root.join("src").join("middleware").join("trace_context.rs");
    let src = read_source(&mw);
    let code = strip_comments_and_literals(&src);
    assert!(
        code.contains("root_span("),
        "假修拦截，请求 root span 仍在创建（把 span 删掉让扫描变干净不算修好）"
    );
    assert!(
        code.contains(".instrument("),
        "契约要求，请求 span 必须由 instrument 绑定到被 poll 的 future"
    );
    assert!(
        !code.contains(".enter("),
        "本文件不得再出现同步 enter 守卫，异步路径唯一正确形态是 instrument"
    );
    // 尾采样标记、X-Trace-Id 回写与两条收尾日志的出参形态不得在重构中丢失，
    // 字符串常量只出现在原文里（归一化会剔除字面量内容），故对原文断言
    for needle in [
        "x-trace-sampled",
        "x-trace-id",
        "trace.complete",
        "trace.tail_sampled",
    ] {
        assert!(
            src.contains(needle),
            "收尾步骤缺失，中间件重构后仍必须保留 {needle} 行为"
        );
    }
    assert!(
        code.contains("or_insert"),
        "X-Trace-Id 回写仍是仅缺失时写入的单一形态"
    );
}
