//! 客户信用列表信封与 Decimal 出参形态的静态锁。
//!
//! 功能，钉住五层判据。
//! 一，list_credits 的出参信封必须是 PaginatedResponse，服务层算出的 total 必须随信封返回调用方，
//! 不允许以丢弃绑定或重新绑定遮蔽的方式静默漏掉。该层按真实形态判定，
//! 正向钉元组绑定与钳制绑定与信封实参，反向钉 total 的重新赋值与遮蔽，
//! 单纯的字样存在性不单独作为通过依据。
//! 二，Cargo.toml 中 rust_decimal 只启用 serde 特性，即 Decimal 的 JSON 出参形态是字符串。
//! 三，不连库直接用 serde_json 序列化 Decimal 复核第二层的实际行为，特性一旦被改成浮点形态本用例立刻变红。
//! 四，前端 customer_credit 出参接口的金额键声明必须与第二层与第三层判定的真实出参形态一致，
//! 两种特性状态下都给出明确结论，特性缺失时不允许静默通过。
//! 五，检测力自证，把总数遮蔽形态与钳制解绑形态两份做坏的夹具与一份正确形态夹具，
//! 喂给第一层同一个判据函数，坏形态必须全部被抓住，正确形态必须不误报。
//! 调用方，cargo test 静态用例与门禁，不连库，不启动服务。
//! 入参，读取 backend/Cargo.toml，backend/src/handlers/customer_credit_handler.rs，
//! frontend/src/api/customer-credit.ts 三个文件的文本，第五层的夹具文本内联在本文件。
//! 传给谁，只在测试内做文本判定，不把任何数据传给服务层，数据库或网络。
//! 存什么，无写入。
//! 存哪里，无写入。

use rust_decimal::Decimal;
use std::fs;
use std::path::Path;

/// 会让 Decimal 出参从 JSON 字符串变成 JSON 数字的 rust_decimal 特性名集合。
/// 命中任一名称即视为出参形态改变，前端金额声明必须同步改成数字。
const FLOAT_FORMING_FEATURES: [&str; 4] = [
    "serde-float",
    "serde-with-float",
    "serde-arbitrary-precision",
    "serde-with-arbitrary-precision",
];

/// 前端 customer_credit 出参接口里由后端 Decimal 承载的金额键。
const MONEY_KEYS: [&str; 3] = ["credit_limit", "used_credit", "available_credit"];

/// PaginatedResponse::new 四个实参的钉定形态，按声明次序排列，
/// 前两个必须是服务层元组绑定出的标识符本身，不接受字面量与长度表达式与兜底写法。
const ENVELOPE_ARG_FORMS: [&str; 4] = ["credits", "total", "page as u64", "page_size as u64"];

/// 正确形态夹具，函数体与真实 handler 的 list_credits 一致，供判据自证不误报。
/// 入参，喂给 lock_verdict 的源文本，只在测试内使用，不写回任何文件。
const GOOD_LIST_CREDITS_FIXTURE: &str = r#"
pub async fn list_credits(
    Query(params): Query<CreditQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<PaginatedResponse<customer_credit::Model>>>, AppError> {
    info!("用户 {} 正在查询客户信用列表", auth.user_id);

    let page = CustomerCreditService::clamp_page(params.page.unwrap_or(1));
    let page_size = params.page_size.unwrap_or(10).clamp(1, 100);

    let service = CustomerCreditService::new(state.db.clone());
    let query_params = CreditQueryParams {
        customer_id: params.customer_id,
        credit_level: params.credit_level,
        status: params.status,
        page,
        page_size,
    };

    let (credits, total) = service.get_list(query_params).await?;
    info!(
        "客户信用列表查询成功，共 {} 条记录，当前页 {} 条",
        total,
        credits.len()
    );

    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        credits,
        total,
        page as u64,
        page_size as u64,
    ))))
}
"#;

/// 总数遮蔽形态的坏夹具，在正确形态之上插入一行用 credits.len() 重新绑定 total，
/// 信封构造句文本不变，用于自证判据能让该形态变红。
/// 入参，喂给 lock_verdict 的源文本，只在测试内使用，不写回任何文件。
const SHADOWED_TOTAL_FIXTURE: &str = r#"
pub async fn list_credits(
    Query(params): Query<CreditQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<PaginatedResponse<customer_credit::Model>>>, AppError> {
    info!("用户 {} 正在查询客户信用列表", auth.user_id);

    let page = CustomerCreditService::clamp_page(params.page.unwrap_or(1));
    let page_size = params.page_size.unwrap_or(10).clamp(1, 100);

    let service = CustomerCreditService::new(state.db.clone());
    let query_params = CreditQueryParams {
        customer_id: params.customer_id,
        credit_level: params.credit_level,
        status: params.status,
        page,
        page_size,
    };

    let (credits, total) = service.get_list(query_params).await?;
    info!(
        "客户信用列表查询成功，共 {} 条记录，当前页 {} 条",
        total,
        credits.len()
    );

    let total = credits.len() as i64;

    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        credits,
        total,
        page as u64,
        page_size as u64,
    ))))
}
"#;

/// 钳制解绑形态的坏夹具，clamp_page 调用结果不再绑定给 page，page 改用裸值，
/// clamp_page 字样仍在函数体内出现，用于自证判据能让该形态变红。
/// 入参，喂给 lock_verdict 的源文本，只在测试内使用，不写回任何文件。
const UNBOUND_CLAMP_FIXTURE: &str = r#"
pub async fn list_credits(
    Query(params): Query<CreditQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<PaginatedResponse<customer_credit::Model>>>, AppError> {
    info!("用户 {} 正在查询客户信用列表", auth.user_id);

    CustomerCreditService::clamp_page(params.page.unwrap_or(1));
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(10).clamp(1, 100);

    let service = CustomerCreditService::new(state.db.clone());
    let query_params = CreditQueryParams {
        customer_id: params.customer_id,
        credit_level: params.credit_level,
        status: params.status,
        page,
        page_size,
    };

    let (credits, total) = service.get_list(query_params).await?;
    info!(
        "客户信用列表查询成功，共 {} 条记录，当前页 {} 条",
        total,
        credits.len()
    );

    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        credits,
        total,
        page as u64,
        page_size as u64,
    ))))
}
"#;

/// 读取仓库内文件文本。
/// 入参为相对 backend 目录的路径，由 CARGO_MANIFEST_DIR 定位，避免依赖测试运行时的当前工作目录。
fn read_repo_file(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("无法读取 {}: {e}", path.display()))
}

/// 取 Cargo.toml 中 rust_decimal 的依赖声明行。
/// 只认依赖段里的赋值行，排除 sea-orm 的 with-rust_decimal 特性名与注释干扰。
fn rust_decimal_dep_line(cargo_toml: &str) -> &str {
    cargo_toml
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("rust_decimal =") || l.starts_with("rust_decimal="))
        .unwrap_or_else(|| {
            panic!("Cargo.toml 应声明 rust_decimal 依赖，实际内容如下：{cargo_toml}")
        })
}

/// 依据 Cargo.toml 声明判定 Decimal 出参是否为字符串形态。
/// 返回 true 表示未启用任何浮点化特性，即出参为字符串。
fn decimal_wire_form_is_string(cargo_toml: &str) -> bool {
    let line = rust_decimal_dep_line(cargo_toml);
    !FLOAT_FORMING_FEATURES.iter().any(|f| line.contains(f))
}

/// 提取 list_credits 函数体文本，用于把出参信封与 total 的去向钉在单个函数范围内。
/// 入参为 handler 文件全文，按大括号配平扫描，避免把相邻 handler 的断言误算进来。
fn list_credits_fn_text(handler_src: &str) -> &str {
    let start = handler_src
        .find("pub async fn list_credits")
        .expect("customer_credit_handler.rs 应存在 list_credits 入口函数");
    let body_start = handler_src[start..]
        .find('{')
        .map(|i| start + i)
        .expect("list_credits 应有函数体起始大括号");
    let mut depth = 0usize;
    let bytes = handler_src.as_bytes();
    let mut idx = body_start;
    while idx < bytes.len() {
        match bytes[idx] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &handler_src[start..=idx];
                }
            }
            _ => {}
        }
        idx += 1;
    }
    panic!("list_credits 函数体大括号未配平，无法定位函数结束位置");
}

/// 提取前端 CustomerCredit 出参接口体文本，只看该接口，不看请求体 DTO。
/// 入参为 frontend/src/api/customer-credit.ts 全文。
fn customer_credit_interface_block(api_src: &str) -> &str {
    let marker = "export interface CustomerCredit {";
    let start = api_src
        .find(marker)
        .expect("frontend/src/api/customer-credit.ts 应声明 CustomerCredit 出参接口");
    let rest = &api_src[start..];
    let end = rest.find('}').expect("CustomerCredit 接口缺少结束大括号");
    &rest[..end]
}

/// 取接口体内某个键的声明类型文本，键不存在或类型无法解析时直接判红。
/// 入参为接口体文本与键名，返回形如 string 或 number 的类型词，可选标记与分号被剥掉。
fn declared_type_of(block: &str, key: &str) -> String {
    let line = block
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with(&format!("{key}?")) || l.starts_with(&format!("{key}:")))
        .unwrap_or_else(|| panic!("CustomerCredit 接口应声明金额键 {key}，实际接口体：{block}"));
    let after_colon = line
        .split_once(':')
        .map(|(_, t)| t)
        .expect("CustomerCredit 接口的金额键声明缺少类型冒号");
    after_colon.trim().trim_end_matches(';').trim().to_string()
}

/// 把注释与字符串与字符字面量的内容替换为空格，换行保留，长度逐字节不变，
/// 使输出下标与原文下标一一对应，扫描命中后可直接切回原文。
/// 入参为源文本全文，行注释与块注释与双引号串与原始字符串与字符字面量都被净化。
fn sanitize_preserving_layout(src: &str) -> String {
    let b = src.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0usize;
    while i < b.len() {
        if b[i] == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                out.push(b' ');
                i += 1;
            }
        } else if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
            out.push(b' ');
            out.push(b' ');
            i += 2;
            let mut depth = 1usize;
            while i < b.len() && depth > 0 {
                if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
                    out.push(b' ');
                    out.push(b' ');
                    i += 2;
                    depth += 1;
                } else if b[i] == b'*' && b.get(i + 1) == Some(&b'/') {
                    out.push(b' ');
                    out.push(b' ');
                    i += 2;
                    depth -= 1;
                } else if b[i] == b'\n' {
                    out.push(b'\n');
                    i += 1;
                } else {
                    out.push(b' ');
                    i += 1;
                }
            }
        } else if b[i] == b'r' && (b.get(i + 1) == Some(&b'"') || b.get(i + 1) == Some(&b'#')) {
            let mut j = i + 1;
            let mut hashes = 0usize;
            while b.get(j) == Some(&b'#') {
                hashes += 1;
                j += 1;
            }
            if b.get(j) == Some(&b'"') && (hashes > 0 || b[i + 1] == b'"') {
                for _ in i..=j {
                    out.push(b' ');
                }
                i = j + 1;
                loop {
                    if i >= b.len() {
                        break;
                    }
                    if b[i] == b'"' {
                        let mut k = i + 1;
                        let mut matched = true;
                        for _ in 0..hashes {
                            if b.get(k) == Some(&b'#') {
                                k += 1;
                            } else {
                                matched = false;
                                break;
                            }
                        }
                        if matched {
                            for _ in i..k {
                                out.push(b' ');
                            }
                            i = k;
                            break;
                        }
                    }
                    if b[i] == b'\n' {
                        out.push(b'\n');
                    } else {
                        out.push(b' ');
                    }
                    i += 1;
                }
            } else {
                out.push(b'_');
                i += 1;
            }
        } else if b[i] == b'"' {
            out.push(b' ');
            i += 1;
            while i < b.len() {
                if b[i] == b'\\' {
                    out.push(b' ');
                    i += 1;
                    if i < b.len() {
                        out.push(b' ');
                        i += 1;
                    }
                } else if b[i] == b'"' {
                    out.push(b' ');
                    i += 1;
                    break;
                } else if b[i] == b'\n' {
                    out.push(b'\n');
                    i += 1;
                } else {
                    out.push(b' ');
                    i += 1;
                }
            }
        } else if b[i] == b'\''
            && (b.get(i + 2) == Some(&b'\'')
                || (b.get(i + 1) == Some(&b'\\') && b.get(i + 3) == Some(&b'\'')))
        {
            out.push(b' ');
            i += 1;
            while i < b.len() {
                if b[i] == b'\\' {
                    out.push(b' ');
                    i += 1;
                } else if b[i] == b'\'' {
                    out.push(b' ');
                    i += 1;
                    break;
                } else {
                    out.push(b' ');
                    i += 1;
                }
            }
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).expect("净化结果应始终是合法 utf8")
}

/// 按大括号配平定位 list_credits 函数体内容的起止下标。
/// 入参为净化后的源文本，找不到函数签名或括号不配平时返回空。
fn fn_body_range(san: &str) -> Option<(usize, usize)> {
    let sig = san.find("pub async fn list_credits")?;
    let open = san[sig..].find('{')? + sig;
    let bytes = san.as_bytes();
    let mut depth = 0usize;
    let mut idx = open;
    while idx < bytes.len() {
        match bytes[idx] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((open + 1, idx));
                }
            }
            _ => {}
        }
        idx += 1;
    }
    None
}

/// 在函数体区间内按同层分号切分语句，返回每条语句不含分号的起止下标。
/// 入参为净化文本与函数体区间，字符串与注释中的分号已在净化阶段消失，嵌套括号内的分号不切。
fn split_statements(san: &str, body: (usize, usize)) -> Vec<(usize, usize)> {
    let bytes = san.as_bytes();
    let mut stmts = Vec::new();
    let mut start = body.0;
    let mut depth = 0usize;
    let mut idx = body.0;
    while idx < body.1 {
        match bytes[idx] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            b';' if depth == 0 => {
                stmts.push((start, idx));
                start = idx + 1;
            }
            _ => {}
        }
        idx += 1;
    }
    if start < body.1 {
        let tail = san[start..body.1].trim();
        if !tail.is_empty() && !tail.chars().all(|c| "{};".contains(c) || c.is_whitespace()) {
            stmts.push((start, body.1));
        }
    }
    stmts
}

/// 从给定下标起跳过空白字符，返回第一个非空白下标，越界时返回文本长度。
fn skip_ws(s: &str, from: usize) -> usize {
    let b = s.as_bytes();
    let mut i = from;
    while i < b.len() && b[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

/// 在文本内从 from 起查找整词标识符的出现下标，下划线与字母数字视为词字符，
/// 命中 total_pages 这类更长名字的组成部分时忽略。入参为待扫描文本与目标词与起始下标。
fn find_word(text: &str, word: &str, from: usize) -> Option<usize> {
    let b = text.as_bytes();
    let w = word.as_bytes();
    let mut i = from;
    while i + w.len() <= b.len() {
        if b[i..i + w.len()].starts_with(w) {
            let prev_ok = i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_');
            let next_ok = i + w.len() == b.len()
                || !(b[i + w.len()].is_ascii_alphanumeric() || b[i + w.len()] == b'_');
            if prev_ok && next_ok {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// 判断给定下标之后跳过空白是否紧跟赋值算符，即单独等号或复合赋值，比较用的双等号不算。
fn follows_assign_op(s: &str, from: usize) -> bool {
    let b = s.as_bytes();
    let i = skip_ws(s, from);
    if i >= b.len() {
        return false;
    }
    match b[i] {
        b'=' => b.get(i + 1) != Some(&b'='),
        b'+' | b'-' | b'*' | b'/' | b'%' | b'&' | b'|' | b'^' => b.get(i + 1) == Some(&b'='),
        _ => false,
    }
}

/// 判断给定下标之后是否为解构模式的收尾形态，即右括号跳过空白后同层紧跟赋值等号，
/// 用于识别把 total 放进 let 左侧元组模式的重新绑定。
fn follows_destructure_assign(s: &str, from: usize) -> bool {
    let b = s.as_bytes();
    let i = skip_ws(s, from);
    if i >= b.len() || b[i] != b')' {
        return false;
    }
    let j = skip_ws(s, i + 1);
    j < b.len() && b[j] == b'=' && b.get(j + 1) != Some(&b'=')
}

/// 找到包含给定下标的语句区间，找不到时返回空。
fn enclosing_stmt(stmts: &[(usize, usize)], abs: usize) -> Option<(usize, usize)> {
    stmts.iter().copied().find(|&(a, b)| a <= abs && abs < b)
}

/// 归一化语句文本，去掉全部空白，供与钉定的形态串做前后缀比较。
fn norm_stmt(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// 把命中下标区间切回原文并压成一行，供失败消息带出被钉句的原文。
/// 入参为原始源文本与在净化文本上取得的下标区间，两者逐字节对齐。
fn quote_src(src: &str, range: (usize, usize)) -> String {
    src[range.0..range.1]
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("，")
}

/// 从给定下标处的左括号起按配平找到右括号，返回括号内内容的起止下标。
fn balanced_paren(san: &str, open: usize) -> Option<(usize, usize)> {
    let bytes = san.as_bytes();
    let mut depth = 0usize;
    let mut idx = open;
    while idx < bytes.len() {
        match bytes[idx] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some((open + 1, idx));
                }
            }
            _ => {}
        }
        idx += 1;
    }
    None
}

/// 在给定区间内按同层逗号切分实参，返回每段的起止下标，空段被剔除。
/// 入参为净化文本与括号内区间，嵌套括号内的逗号不切。
fn split_top_level_commas(san: &str, inner: (usize, usize)) -> Vec<(usize, usize)> {
    let bytes = san.as_bytes();
    let mut parts = Vec::new();
    let mut start = inner.0;
    let mut depth = 0usize;
    let mut idx = inner.0;
    while idx < inner.1 {
        match bytes[idx] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            b',' if depth == 0 => {
                if !san[start..idx].trim().is_empty() {
                    parts.push((start, idx));
                }
                start = idx + 1;
            }
            _ => {}
        }
        idx += 1;
    }
    if start < inner.1 && !san[start..inner.1].trim().is_empty() {
        parts.push((start, inner.1));
    }
    parts
}

/// 客户信用列表 handler 的形态判据，对任意源码文本执行同一套判定，返回违规说明清单，
/// 空清单代表通过。真实 handler 全文与故意做坏的夹具都走这一个函数，保证检测力可自证。
/// 正向钉，服务层元组绑定唯一存在且右侧是 get_list 调用链，page 由 clamp_page 结果绑定，
/// page_size 同型绑定且保留钳制，信封构造实参与钉定形态逐位一致。
/// 反向钉，函数体内 total 不得被重新赋值或遮蔽，解构重绑定与裸赋值与长度顶替都判违规，
/// 命中即在说明里带出该句原文。
/// 调用方，本文件的真实文件用例与夹具自证用例。入参，一段源文本。
/// 传给谁，只返回文本判定结果给测试断言。存什么，无写入。存哪里，无写入。
fn lock_verdict(src: &str) -> Vec<String> {
    let mut violations: Vec<String> = Vec::new();
    let san = sanitize_preserving_layout(src);
    let body = match fn_body_range(&san) {
        Some(r) => r,
        None => {
            violations.push("无法定位 list_credits 函数签名与函数体，形态判据无法成立".to_string());
            return violations;
        }
    };
    let stmts = split_statements(&san, body);

    let canonical: Vec<(usize, usize)> = stmts
        .iter()
        .filter(|&&(a, b)| {
            let n = norm_stmt(&san[a..b]);
            n.starts_with("let(credits,total)=service.get_list(") && n.ends_with(".await?")
        })
        .copied()
        .collect();
    let canonical_first = canonical.first().copied();
    if canonical.len() != 1 {
        violations.push(format!(
            "函数体内应恰好存在一条把服务层 get_list 返回值绑定为 credits 与 total 的元组 let 语句，实际命中 {} 条",
            canonical.len()
        ));
    }

    let page_bound = stmts.iter().any(|&(a, b)| {
        let n = norm_stmt(&san[a..b]);
        n.starts_with("letpage=CustomerCreditService::clamp_page(") && n.ends_with(')')
    });
    if !page_bound {
        violations.push(
            "函数体内应存在 let page 绑定 CustomerCreditService::clamp_page 调用结果的语句，钳制结果被解绑或 page 改用裸值即为违规"
                .to_string(),
        );
    }

    let page_size_bound = stmts.iter().any(|&(a, b)| {
        let n = norm_stmt(&san[a..b]);
        n.starts_with("letpage_size=") && n.ends_with(".clamp(1,100)")
    });
    if !page_size_bound {
        violations.push("函数体内应存在 let page_size 同型绑定且保留上下界钳制的语句".to_string());
    }

    let body_text = &san[body.0..body.1];
    let mut scan_from = 0usize;
    while let Some(rel) = find_word(body_text, "total", scan_from) {
        scan_from = rel + "total".len();
        let tb = body_text.as_bytes();
        if rel > 0 && tb[rel - 1] == b'.' {
            continue;
        }
        let abs = body.0 + rel;
        let after = abs + "total".len();
        let assigned_directly = follows_assign_op(&san, after);
        let stmt = enclosing_stmt(&stmts, abs);
        let shadow_destructure = follows_destructure_assign(&san, after)
            && stmt.map_or(false, |r| Some(r) != canonical_first);
        if assigned_directly || shadow_destructure {
            let range = stmt.unwrap_or(body);
            let mut msg = format!(
                "total 在函数体内被重新绑定或遮蔽，服务层算出的总数可能已在该句丢失，原文，{}",
                quote_src(src, range)
            );
            if san[range.0..range.1].contains("credits.len()") {
                msg.push_str("。该句让 credits.len() 参与 total 的计算，属于用当前页长度顶替服务层总数的形态");
            }
            violations.push(msg);
        }
    }

    let marker = "PaginatedResponse::new";
    let mut call_count = 0usize;
    let mut call_args_range: Option<(usize, usize)> = None;
    let mut search = 0usize;
    while let Some(pos) = san[search..].find(marker) {
        call_count += 1;
        let open = skip_ws(&san, search + pos + marker.len());
        if san.as_bytes().get(open) == Some(&b'(') {
            if let Some(inner) = balanced_paren(&san, open) {
                if call_args_range.is_none() {
                    call_args_range = Some(inner);
                }
            }
        }
        search += pos + marker.len();
    }
    if call_count != 1 {
        violations.push(format!(
            "函数体内应有且仅有一条 PaginatedResponse::new 构造调用，实际命中 {} 条",
            call_count
        ));
    }
    if let Some(inner) = call_args_range {
        let args = split_top_level_commas(&san, inner);
        let norms: Vec<String> = args.iter().map(|&(a, b)| norm_stmt(&san[a..b])).collect();
        if norms.len() != ENVELOPE_ARG_FORMS.len() {
            violations.push(format!(
                "PaginatedResponse::new 实参个数应为 {} 个，实际 {} 个，原文，{}",
                ENVELOPE_ARG_FORMS.len(),
                norms.len(),
                quote_src(src, inner)
            ));
        } else {
            for (idx, (actual, expected)) in norms.iter().zip(ENVELOPE_ARG_FORMS.iter()).enumerate()
            {
                if actual != expected {
                    violations.push(format!(
                        "PaginatedResponse::new 第 {} 个实参应恰好是钉定形态 {} 本身，不接受字面量与长度表达式与兜底写法，实际为 {actual}，原文，{}",
                        idx + 1,
                        expected,
                        quote_src(src, inner)
                    ));
                }
            }
        }
    }

    violations
}

#[test]
fn list_credits_returns_paginated_envelope_and_carries_total() {
    let handler_src = read_repo_file("src/handlers/customer_credit_handler.rs");
    let fn_text = list_credits_fn_text(&handler_src);

    assert!(
        fn_text.contains("ApiResponse<PaginatedResponse<customer_credit::Model>>"),
        "list_credits 出参信封应为 ApiResponse 包 PaginatedResponse，供前端分页器读取总数，实际函数体：{fn_text}"
    );
    assert!(
        fn_text.contains("PaginatedResponse::new"),
        "list_credits 应通过 PaginatedResponse::new 把 items 与 total 与 page 与 page_size 一并返回，实际函数体：{fn_text}"
    );
    let nows: String = fn_text.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        nows.contains("PaginatedResponse::new(credits,total,pageasu64,page_sizeasu64,)"),
        "list_credits 构造分页信封的实参次序应为 credits 与 total 与 page 与 page_size，\
         服务层算出的总数必须进信封，实际函数体：{fn_text}"
    );
    assert!(
        fn_text.contains("let (credits, total)"),
        "list_credits 应绑定服务层 get_list 返回的 total 并进入响应信封，实际函数体：{fn_text}"
    );
    assert!(
        !handler_src.contains("_total"),
        "customer_credit_handler.rs 不应出现下划线前缀的丢弃绑定，服务层已算出的总数被静默丢掉即为缺陷"
    );
    assert!(
        handler_src.contains("PaginatedResponse"),
        "customer_credit_handler.rs 应引入 PaginatedResponse 作为分页信封类型"
    );
    assert!(
        fn_text.contains("CustomerCreditService::clamp_page"),
        "list_credits 的页码应复用服务层 clamp_page 既有口径，不新造缺省值，实际函数体：{fn_text}"
    );

    let verdict = lock_verdict(&handler_src);
    assert!(
        verdict.is_empty(),
        "list_credits 形态判据违规 {} 条，逐条如下，\n{}",
        verdict.len(),
        verdict.join("\n")
    );
}

#[test]
fn rust_decimal_dependency_pins_string_serialization_feature_set() {
    let cargo_toml = read_repo_file("Cargo.toml");
    let line = rust_decimal_dep_line(&cargo_toml);

    assert!(
        line.contains("features = [\"serde\"]"),
        "rust_decimal 依赖应只启用 serde 特性以保证金额出参为字符串，实际声明行：{line}"
    );
    for feature in FLOAT_FORMING_FEATURES {
        assert!(
            !line.contains(feature),
            "rust_decimal 不应启用 {feature}，启用后 Decimal 出参从字符串变为数字，前端金额声明必须同步改成 number，实际声明行：{line}"
        );
    }
    assert!(
        decimal_wire_form_is_string(&cargo_toml),
        "按当前 Cargo.toml 判定，Decimal 出参形态应为字符串；判定不成立时说明依赖声明出现未登记的浮点化写法，实际声明行：{line}"
    );
}

/// 行为复核，不连库直接序列化 Decimal，证明特性面声明与真实出参形态一致。
/// 特性一旦被改成浮点形态，本用例断言的实际字符串会变为数字文本而立刻变红，并给出同步前端的线索。
#[test]
fn decimal_value_is_serialized_as_json_string_at_runtime() {
    let value = Decimal::new(123456, 2);
    let json = serde_json::to_string(&value).expect("Decimal 序列化应成功");

    assert_eq!(
        json, "\"1234.56\"",
        "Decimal 出参形态应为带引号的 JSON 字符串，实际为 {json}；若 Cargo.toml 启用了浮点化特性，\
         则后端出参已变成数字，frontend/src/api/customer-credit.ts 的 CustomerCredit 金额键必须同步改成 number"
    );
}

/// 前端金额声明与后端出参形态的一致性双向锁。
/// 期望形态由本进程真实序列化一次 Decimal 得出，不靠文本推测，特性未启用时要求前端声明 string，
/// 启用浮点化特性后要求前端声明 number，两种状态都必须给出明确结论，不做跳过。
#[test]
fn frontend_credit_amount_declarations_match_backend_decimal_wire_form() {
    let api_src = read_repo_file("../frontend/src/api/customer-credit.ts");
    let block = customer_credit_interface_block(&api_src);
    let sample = serde_json::to_value(Decimal::new(100, 2)).expect("Decimal 序列化应成功");
    let expected_type = if sample.is_string() {
        "string"
    } else {
        "number"
    };

    for key in MONEY_KEYS {
        let declared = declared_type_of(block, key);
        assert_eq!(
            declared, expected_type,
            "后端 Decimal 的真实金额出参形态是 {expected_type}，\
             但 frontend/src/api/customer-credit.ts 的 CustomerCredit 接口把 {key} 声明成 {declared}，\
             声明与真实形态不符会让按声明取数的消费方拿到错误类型且运行期不报错；\
             请把该键改成 {expected_type}，或同步调整 Cargo.toml 的 rust_decimal 特性"
        );
    }
}

/// 检测力自证，把遮蔽与解绑两份坏夹具与一份正确夹具喂给同一个判据函数 lock_verdict。
/// 坏形态必须被抓住且失败说明里带出被钉句原文，正确形态必须不误报，
/// 判据失去检测力或被改坏时本用例直接变红。
#[test]
fn lock_verdict_detects_shadowed_total_and_unbound_clamp_fixtures() {
    let good = lock_verdict(GOOD_LIST_CREDITS_FIXTURE);
    assert!(
        good.is_empty(),
        "正确形态夹具不应产生违规，实际违规如下，\n{}",
        good.join("\n")
    );

    let shadowed = lock_verdict(SHADOWED_TOTAL_FIXTURE);
    assert!(
        !shadowed.is_empty(),
        "遮蔽夹具用 let total = credits.len() 顶替服务层总数且信封构造句文本不变，判据必须变红但没有抓住任何违规"
    );
    assert_eq!(
        shadowed.len(),
        1,
        "遮蔽夹具只应命中遮蔽这一条违规，实际违规如下，\n{}",
        shadowed.join("\n")
    );
    assert!(
        shadowed[0].contains("total 在函数体内被重新绑定或遮蔽")
            && shadowed[0].contains("let total = credits.len() as i64")
            && shadowed[0].contains("credits.len() 参与 total 的计算"),
        "遮蔽夹具的失败说明应带出被钉句原文与长度顶替线索，实际为 {}",
        shadowed[0]
    );

    let unbound = lock_verdict(UNBOUND_CLAMP_FIXTURE);
    assert!(
        !unbound.is_empty(),
        "解绑夹具让 clamp_page 字样仍在函数体内但结果不再绑定给 page，判据必须变红但没有抓住任何违规"
    );
    assert_eq!(
        unbound.len(),
        1,
        "解绑夹具只应命中钳制解绑这一条违规，实际违规如下，\n{}",
        unbound.join("\n")
    );
    assert!(
        unbound[0].contains("clamp_page"),
        "解绑夹具的失败说明应带出 clamp_page 绑定缺失的线索，实际为 {}",
        unbound[0]
    );
}
