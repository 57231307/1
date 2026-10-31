//! 价税换算方向静态锁（契约锁族，纯静态源码普查，不连库、不依赖被测 crate）
//!
//! 锁定的契约，含税金额是录入事实，净额只能由 含税 除以 (1 加 税率) 反算，
//! 税额取 含税 减 净额 的差额并与含税逐分钩稽；反向（净额是事实）只允许内核的
//! split_from_net 一条入口。本锁在后端源码里普查五类破口形态：
//! 一 用 (1 加 税率) 因子乘出含税，
//! 二 用 (1 加 税率) 因子除出净额却绕过内核入口，
//! 三 内核之外自写 金额 乘 税率 或 除 100 的第二套税额公式，
//! 四 用 1.13 与 0.13 这类写死档位代替单据自身税率，
//! 五 用乘法从含税侧推出净额侧。
//! 判红口径，命中数经逐条带理由的豁免表抵扣后必须为 0，且每条豁免都必须被真实命中
//! 用尽（位点多、条数漂移、豁免失效一律判红，防止豁免表变成逃逸口）。
//! 检测力自证，夹具与真实源码共用同一 scan_source 入口；违规夹具五类各必被抓到，
//! 合规形态（内核入口调用、单价与数量相乘、纯字段搬运）必须零误伤，
//! 注释与字符串字面量里的反例文本不得误判，中文注释文件不得索引错位。
//! 本文件只用 std，可独立 `rustc --test` 复跑，也可由 cargo 的集成测试直接执行。

use std::path::{Path, PathBuf};

/// 一条命中：文件、行号、规则名与原文
#[derive(Debug, Clone, PartialEq, Eq)]
struct Hit {
    file: String,
    line: usize,
    rule: &'static str,
    text: String,
}

/// 一条豁免：文件、规则、钉死的命中条数与理由（缺一即判红）
struct Exemption {
    file: &'static str,
    rule: &'static str,
    expected: usize,
    reason: &'static str,
}

/// 税额内核本体，五类形态里唯一允许出现因子乘除与税率公式的位置
const KERNEL_FILE: &str = "src/utils/tax.rs";

/// 逐条豁免表，理由必须写明为什么这条形态在本文件成立
/// 新增豁免必须同时给出真实命中，否则本锁判红（不许整目录豁免）
const EXEMPTIONS: &[Exemption] = &[
    Exemption {
        file: "src/services/environmental_tax_service.rs",
        rule: "内核外的税率乘除公式",
        expected: 1,
        reason: "环保税按污染当量乘每当量税额计征，基数是当量数不是价税金额，\
                 不属价税分离域，内核文档已把该域列为例外",
    },
];

fn read_source(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("扫描锁夹具：读取 {} 失败，错误 {e}", path.display()))
        .replace('\r', "")
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("扫描锁夹具：目录 {} 读取失败，错误 {e}", dir.display()))
    {
        let path = entry
            .unwrap_or_else(|e| panic!("扫描锁夹具：目录项读取失败，错误 {e}"))
            .path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// 相对根目录的 POSIX 风格路径，用于与豁免表逐字比对
fn rel_path(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .display()
        .to_string()
        .replace('\\', "/")
}

/// 把注释与字符串字面量整段抹成空格，保留行号与列宽，避免索引错位
/// 判据只看真实代码，注释与字符串里写的反例文本不得参与匹配
fn strip_comments_and_strings(src: &str) -> String {
    let bytes: Vec<char> = src.chars().collect();
    let mut out: Vec<char> = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    let mut block_depth = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        if block_depth > 0 {
            if c == '/' && i + 1 < bytes.len() && bytes[i + 1] == '*' {
                block_depth += 1;
                out.push(' ');
                out.push(' ');
                i += 2;
                continue;
            }
            if c == '*' && i + 1 < bytes.len() && bytes[i + 1] == '/' {
                block_depth -= 1;
                out.push(' ');
                out.push(' ');
                i += 2;
                continue;
            }
            out.push(if c == '\n' { '\n' } else { ' ' });
            i += 1;
            continue;
        }
        // 行注释
        if c == '/' && i + 1 < bytes.len() && bytes[i + 1] == '/' {
            while i < bytes.len() && bytes[i] != '\n' {
                out.push(' ');
                i += 1;
            }
            continue;
        }
        // 块注释起始
        if c == '/' && i + 1 < bytes.len() && bytes[i + 1] == '*' {
            block_depth = 1;
            out.push(' ');
            out.push(' ');
            i += 2;
            continue;
        }
        // 原始字符串 r"..." 与 r#"..."#
        if c == 'r' && i + 1 < bytes.len() && (bytes[i + 1] == '"' || bytes[i + 1] == '#') {
            let mut j = i + 1;
            let mut hashes = 0usize;
            while j < bytes.len() && bytes[j] == '#' {
                hashes += 1;
                j += 1;
            }
            if j < bytes.len() && bytes[j] == '"' {
                for _ in i..j + 1 {
                    out.push(' ');
                }
                i = j + 1;
                // 扫到匹配的 "# 序列
                loop {
                    if i >= bytes.len() {
                        break;
                    }
                    if bytes[i] == '"' {
                        let mut k = i + 1;
                        let mut matched = true;
                        for _ in 0..hashes {
                            if k >= bytes.len() || bytes[k] != '#' {
                                matched = false;
                                break;
                            }
                            k += 1;
                        }
                        if matched {
                            for _ in i..k {
                                out.push(' ');
                            }
                            i = k;
                            break;
                        }
                    }
                    out.push(if bytes[i] == '\n' { '\n' } else { ' ' });
                    i += 1;
                }
                continue;
            }
        }
        // 普通字符串与字符字面量（跳过转义）
        if c == '"' {
            out.push(' ');
            i += 1;
            while i < bytes.len() {
                match bytes[i] {
                    '\\' => {
                        out.push(' ');
                        i += 1;
                        if i < bytes.len() {
                            out.push(if bytes[i] == '\n' { '\n' } else { ' ' });
                            i += 1;
                        }
                    }
                    '"' => {
                        out.push(' ');
                        i += 1;
                        break;
                    }
                    other => {
                        out.push(if other == '\n' { '\n' } else { ' ' });
                        i += 1;
                    }
                }
            }
            continue;
        }
        // 字符字面量（含转义），单引号后不是闭合单引号的一律按生命周期处理
        if c == '\'' {
            let is_char_literal = (i + 2 < bytes.len() && bytes[i + 2] == '\'' && bytes[i + 1] != '\\')
                || (i + 3 < bytes.len() && bytes[i + 1] == '\\' && bytes[i + 3] == '\'');
            if is_char_literal {
                let end = if bytes[i + 1] == '\\' { i + 4 } else { i + 3 };
                for _ in i..end.min(bytes.len()) {
                    out.push(' ');
                }
                i = end;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out.into_iter().collect()
}

/// 含税侧字面（先把 xxx_with_tax 归一成含税标记，避免 unit_price 与 unit_price_with_tax 混判）
fn normalize_gross(line: &str) -> String {
    let mut text = line.to_string();
    for pat in ["unit_price_with_tax", "amount_with_tax", "price_with_tax"] {
        text = text.replace(pat, "含税侧占位");
    }
    text
}

fn has_any(text: &str, tokens: &[&str]) -> bool {
    tokens.iter().any(|t| text.contains(t))
}

/// 税率与税因子相关字面
fn has_tax_token(text: &str) -> bool {
    has_any(
        text,
        &[
            "tax_rate",
            "tax_percent",
            "tax_pct",
            "tax_ratio",
            "vat_rate",
            "input_tax_rate",
            "output_tax_rate",
            "PERCENT",
        ],
    )
}

/// 金额与单价相关字面
fn has_money_token(text: &str) -> bool {
    has_any(
        text,
        &[
            "amount",
            "price",
            "total",
            "subtotal",
            "含税侧占位",
            "inclusive",
            "gross",
            "money",
            "cost",
            "net",
        ],
    )
}

/// 含税因子 (1 加 税率) 的字面形态
fn has_factor_sum(text: &str) -> bool {
    has_any(
        text,
        &[
            "(Decimal::ONE +",
            "(Decimal::ONE+",
            "(1 +",
            "(1+",
            "(Decimal::new(1, 0) +",
        ],
    )
}

/// 写死的含税档位字面
const TIER_LITERALS: &[&str] = &[
    "1.13", "1.130", "1.09", "1.06", "1.05", "1.03", "0.13", "0.09", "0.06", "0.03",
];

/// 行内赋值拆分，取第一个既不是比较也不是闭包的等号
/// 返回等号左侧与右侧；没有赋值形态时返回 None
fn split_assignment(line: &str) -> Option<(&str, &str)> {
    let bytes: Vec<char> = line.chars().collect();
    let mut idx = 0usize;
    while idx < bytes.len() {
        if bytes[idx] == '=' {
            let prev = if idx > 0 { bytes[idx - 1] } else { ' ' };
            let next = if idx + 1 < bytes.len() {
                bytes[idx + 1]
            } else {
                ' '
            };
            let is_compare_or_arrow = matches!(prev, '!' | '<' | '>' | '=') || next == '=' || next == '>';
            let is_binding = matches!(prev, '=' | '|' | '&' | '>' | '<' | '+' | '-' | '*' | '/');
            if !is_compare_or_arrow && !is_binding {
                return Some((line.get(..idx)?, line.get(idx + 1..)?));
            }
        }
        idx += 1;
    }
    None
}

/// 净额侧字面，等号左侧出现即认为本行在算净额
const NET_TARGETS: &[&str] = &["net", "subtotal", "unit_price", "amount", "净额", "discount_amount"];

/// 对一段源码文本按行判五类形态，返回命中清单（纯函数，夹具与真实源码共用）
fn scan_source(rel_file: &str, src: &str) -> Vec<Hit> {
    let cleaned = strip_comments_and_strings(src);
    let mut hits = Vec::new();
    if rel_file == KERNEL_FILE {
        // 内核本体不参与五类判定，但唯一入口的存在由用例单独钉住
        return hits;
    }
    for (idx, raw_line) in cleaned.lines().enumerate() {
        let line = normalize_gross(raw_line);
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let has_mul = line.contains('*');
        let has_div = line.contains('/');
        let tax = has_tax_token(&line);
        let money = has_money_token(&line);

        if has_mul && has_factor_sum(&line) && tax {
            hits.push(Hit {
                file: rel_file.to_string(),
                line: idx + 1,
                rule: "因子乘算造含税",
                text: raw_line.trim().to_string(),
            });
            continue;
        }
        if has_div && has_factor_sum(&line) && tax {
            hits.push(Hit {
                file: rel_file.to_string(),
                line: idx + 1,
                rule: "因子除算绕过内核入口",
                text: raw_line.trim().to_string(),
            });
            continue;
        }
        if tax && (has_mul || has_div) && money && !line.contains("utils::tax::") && !is_kernel_import(&line)
        {
            hits.push(Hit {
                file: rel_file.to_string(),
                line: idx + 1,
                rule: "内核外的税率乘除公式",
                text: raw_line.trim().to_string(),
            });
            continue;
        }
        if has_any(&line, TIER_LITERALS) && (has_mul || has_div) && money {
            hits.push(Hit {
                file: rel_file.to_string(),
                line: idx + 1,
                rule: "写死含税档位",
                text: raw_line.trim().to_string(),
            });
            continue;
        }
        // 第五条判据只看方向，含税事实乘以数量得到含税金额是合法形态（两枚事实相乘），
        // 违规形态是把**净额侧**当作赋值目标又从含税侧乘出来
        if has_mul && has_any(&line, &["含税侧占位", "inclusive", "gross"]) {
            if let Some((lhs, _)) = split_assignment(&line) {
                if has_any(lhs, NET_TARGETS) {
                    hits.push(Hit {
                        file: rel_file.to_string(),
                        line: idx + 1,
                        rule: "用乘法从含税侧推净额",
                        text: raw_line.trim().to_string(),
                    });
                }
            }
        }
    }
    hits
}

fn is_kernel_import(line: &str) -> bool {
    line.trim_start().starts_with("use ") && line.contains("utils::tax")
}

/// 扫描一棵源码树，返回未豁免命中与全部命中
fn scan_tree(root: &Path, dir: &Path) -> (Vec<Hit>, Vec<Hit>) {
    let mut files = Vec::new();
    collect_rs_files(dir, &mut files);
    files.sort();
    let mut all: Vec<Hit> = Vec::new();
    for file in &files {
        let rel = rel_path(root, file);
        let src = read_source(file);
        all.extend(scan_source(&rel, &src));
    }
    let outstanding = apply_exemptions(all.clone());
    (outstanding, all)
}

/// 按豁免表逐条抵扣，抵扣不干净（条数不符或找不到命中）当场 panic
/// 返回未被豁免的命中清单
fn apply_exemptions(all: Vec<Hit>) -> Vec<Hit> {
    let mut remaining = all;
    for exemption in EXEMPTIONS {
        let matched: Vec<Hit> = remaining
            .iter()
            .filter(|h| h.file == exemption.file && h.rule == exemption.rule)
            .cloned()
            .collect();
        if matched.len() != exemption.expected {
            panic!(
                "豁免表漂移，文件 {} 规则 {} 期望命中 {} 条，实际 {} 条，理由 {}",
                exemption.file,
                exemption.rule,
                exemption.expected,
                matched.len(),
                exemption.reason
            );
        }
        for hit in matched {
            if let Some(pos) = remaining.iter().position(|h| *h == hit) {
                remaining.remove(pos);
            }
        }
    }
    remaining
}

fn format_hits(hits: &[Hit]) -> String {
    hits.iter()
        .map(|h| format!("{}:{}:{} 原文 {}", h.file, h.line, h.rule, h.text))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 反向夹具，每类违规形态一枚真实写法，外加合规形态与注释字符串干扰
fn violation_fixture() -> Vec<(&'static str, &'static str, usize)> {
    vec![
        (
            "factor_mul.rs",
            "pub fn gross(net: Decimal, tax_rate: Decimal) -> Decimal {\n\
             net * (Decimal::ONE + tax_rate / PERCENT)\n\
             }\n",
            1,
        ),
        (
            "factor_div.rs",
            "fn net_of(inclusive: Decimal) -> Decimal {\n\
             let tax_rate = Decimal::new(13, 0);\n\
             inclusive / (Decimal::ONE + tax_rate / Decimal::new(100, 0))\n\
             }\n",
            1,
        ),
        (
            "inline_formula.rs",
            "fn tax_of(amount: Decimal, tax_percent: Decimal) -> Decimal {\n\
             (amount * tax_percent / Decimal::new(100, 0)).round_dp(2)\n\
             }\n",
            1,
        ),
        (
            "tier_literal.rs",
            "fn gross_of(price: Decimal) -> Decimal {\n\
             price * Decimal::new(113, 2) / Decimal::new(100, 0) + price * 0.13\n\
             }\n",
            1,
        ),
        (
            "net_from_gross.rs",
            "fn net_of(item: &Item) -> Decimal {\n\
             let net_amount = item.amount_with_tax * item.factor;\n\
             net_amount\n\
             }\n",
            1,
        ),
    ]
}

/// 合规形态，一条都不许误伤
fn compliant_fixture() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "kernel_entry.rs",
            "fn header(gross: Decimal, tax_rate: Decimal) -> TaxSplit {\n\
             crate::utils::tax::split_from_inclusive(gross, Some(tax_rate)).unwrap()\n\
             }\n",
        ),
        (
            "quantity_times_price.rs",
            "fn line_amount(quantity: Decimal, unit_price: Decimal) -> Decimal {\n\
             (quantity * unit_price).round_dp(2)\n\
             }\n",
        ),
        (
            "gross_fact_line.rs",
            "fn gross_line(quantity: Decimal, unit_price_with_tax: Decimal) -> Decimal {\n\
             (quantity * unit_price_with_tax).round_dp(tax::MONEY_SCALE)\n\
             }\n\
             fn net_line(quantity: Decimal, unit_price: Decimal) -> Decimal {\n\
             (quantity * unit_price).round_dp(tax::MONEY_SCALE)\n\
             }\n",
        ),
        (
            "field_move.rs",
            "fn build(item: &Row) -> ActiveModel {\n\
             ActiveModel {\n\
                 tax_percent: Set(item.tax_percent),\n\
                 amount: Set(item.amount),\n\
             }\n\
             }\n",
        ),
        (
            "comments_only.rs",
            "// 旧口径残留 net * (Decimal::ONE + tax_rate) 与 1.13 都已废除\n\
             /// 说明 含税 除以 (1 + tax_percent) 走内核\n\
             let s = \"net * (1 + tax_rate) 与 amount_with_tax * 0.13\";\n\
             /* 块注释里的 amount * tax_percent / 100 */\n",
        ),
    ]
}

/// 把夹具写进临时目录并扫描，返回未豁免命中与命中总数
fn scan_fixture_dir(files: &[(&str, &str)]) -> (Vec<Hit>, usize, usize) {
    static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "bingxi_tax_direction_fixture_{}_{seq}_{:016x}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64 + d.as_secs() * 1000)
            .unwrap_or_default()
    ));
    std::fs::create_dir_all(&dir)
        .unwrap_or_else(|e| panic!("扫描锁夹具：临时目录 {} 创建失败，错误 {e}", dir.display()));
    for (name, body) in files {
        let path = dir.join(name);
        std::fs::write(&path, body)
            .unwrap_or_else(|e| panic!("扫描锁夹具：写入 {} 失败，错误 {e}", path.display()));
    }
    // 真实读盘再扫一遍，证明夹具确实落到磁盘并可被读回（不是只测内存字符串）
    let mut hits = Vec::new();
    for (name, _) in files {
        let disk_text = read_source(&dir.join(name));
        hits.extend(scan_source(name, &disk_text));
    }
    let scanned_files = files.len();
    let total = hits.len();
    std::fs::remove_dir_all(&dir).unwrap_or_else(|e| {
        panic!("扫描锁夹具：临时目录 {} 清理失败，错误 {e}", dir.display());
    });
    (hits, total, scanned_files)
}

#[test]
fn violation_fixtures_are_all_caught_per_rule() {
    let (hits, total, files) = scan_fixture_dir(
        &violation_fixture()
            .iter()
            .map(|(name, body, _)| (*name, *body))
            .collect::<Vec<_>>(),
    );
    eprintln!(
        "违规夹具命中数 VIOLATION_FIXTURE_HITS={total} 夹具文件数={files} 命中清单={}",
        format_hits(&hits)
    );
    assert_eq!(
        total,
        violation_fixture().len(),
        "五类违规形态必须各被抓到一条，实际抓到 {total} 条，清单 {}",
        format_hits(&hits)
    );
    for (name, _, _) in violation_fixture() {
        assert!(
            hits.iter().any(|h| h.file == name),
            "夹具 {name} 的违规形态没被抓到，检测器判为死器"
        );
    }
    let rules: Vec<&str> = hits.iter().map(|h| h.rule).collect();
    for expected_rule in [
        "因子乘算造含税",
        "因子除算绕过内核入口",
        "内核外的税率乘除公式",
        "写死含税档位",
        "用乘法从含税侧推净额",
    ] {
        assert!(
            rules.contains(&expected_rule),
            "规则 {expected_rule} 在夹具上零命中，说明该规则没接线"
        );
    }
}

#[test]
fn compliant_forms_and_comment_text_are_not_flagged() {
    let (hits, total, _) = scan_fixture_dir(
        &compliant_fixture()
            .iter()
            .map(|(name, body)| (*name, *body))
            .collect::<Vec<_>>(),
    );
    assert_eq!(
        total, 0,
        "合规形态与注释与字符串字面量不得误伤，实际命中 {}",
        format_hits(&hits)
    );
}

#[test]
fn removing_a_violation_line_makes_the_detector_silent() {
    // 变异自证，把夹具里的违规行换成内核入口调用，检测器必须转静默
    let fixed = "fn gross(net: Decimal, tax_rate: Decimal) -> Decimal {\n\
                 crate::utils::tax::split_from_net(net, Some(tax_rate)).unwrap().inclusive\n\
                 }\n";
    let (hits, total, _) = scan_fixture_dir(&[("fixed.rs", fixed)]);
    assert_eq!(
        total, 0,
        "改为内核入口后不该再有命中，实际 {}",
        format_hits(&hits)
    );
}

#[test]
fn real_backend_sources_have_no_unexempted_direction_breaks() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src_dir = root.join("src");
    assert!(
        src_dir.is_dir(),
        "扫描锁必须能找到后端源码目录 {}，找不到就是扫描面失效",
        src_dir.display()
    );
    let (outstanding, all) = scan_tree(&root, &src_dir);
    let mut files = Vec::new();
    collect_rs_files(&src_dir, &mut files);
    eprintln!(
        "真实源码命中数 REAL_HITS={} 扫描文件数={} 豁免条目数={} 未豁免清单={}",
        all.len(),
        files.len(),
        EXEMPTIONS.len(),
        format_hits(&outstanding)
    );
    assert!(
        files.len() >= 300,
        "扫描文件数低于地板 {}，说明扫描面被截断，本锁结论不可信",
        files.len()
    );
    assert!(
        all.len() >= EXEMPTIONS.iter().map(|e| e.expected).sum(),
        "命中总数低于豁免表声明数，豁免表已失效"
    );
    assert!(
        outstanding.is_empty(),
        "价税换算方向锁判红，未豁免命中 {} 条：\n{}",
        outstanding.len(),
        format_hits(&outstanding)
    );
}

#[test]
fn kernel_file_itself_is_exempt_from_rules_but_pins_the_entries() {
    // 内核本体不参与五类判定，但必须真的存在且持有唯一除法入口与唯一正算入口，
    // 否则豁免面就成了无人守的空目录
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kernel = root.join(KERNEL_FILE);
    let src = read_source(&kernel);
    for entry in [
        "pub fn split_from_inclusive",
        "pub fn split_from_net",
        "pub fn net_from_inclusive",
        "pub fn tax_from_inclusive",
        "pub fn assert_three_way_reconcile",
        "pub const MONEY_SCALE",
    ] {
        assert!(
            src.contains(entry),
            "内核必须持有唯一入口 {entry}，缺失说明换算方向的真源被挪走"
        );
    }
    let hits = scan_source(KERNEL_FILE, &src);
    assert!(
        hits.is_empty(),
        "内核文件按设计不参与判定，实际却命中 {} 条，说明豁免面写错",
        hits.len()
    );
}

#[test]
fn detector_flags_the_regression_form_in_a_mutated_copy_of_a_real_file() {
    // 变异自证，把真实文件在内存里改回 净额 乘 税率 除 100 的旧形态，
    // 检测器必须在同一文件同一行点名判红，证明改回乘式不会静默通过。
    // 只改内存副本，磁盘上的真实源码不受影响。
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rel = "src/services/so/amounts.rs";
    let real = read_source(&root.join(rel));
    assert!(
        real.contains("crate::utils::tax::tax_amount"),
        "{rel} 当前未走内核税额入口，本变异自证的前提不成立"
    );
    let mutated = real.replace(
        "crate::utils::tax::tax_amount(amount_after_discount, tax_pct)",
        "(amount_after_discount * (tax_pct / percent_base)).round_dp(2)",
    );
    assert_ne!(
        mutated, real,
        "{rel} 的正向腿写法已变更，变异夹具的替换锚点需要同步"
    );
    let hits = scan_source(rel, &mutated);
    assert!(
        !hits.is_empty(),
        "{rel} 改回乘式后检测器零命中，说明本锁是死器"
    );
    let hit_lines: Vec<usize> = hits.iter().map(|h| h.line).collect();
    eprintln!(
        "变异自证命中 MUTATED_HITS={} 行号 {:?} 规则 {}",
        hits.len(),
        hit_lines,
        hits
            .iter()
            .map(|h| h.rule)
            .collect::<Vec<_>>()
            .join(" 与 ")
    );
    assert!(
        hits.iter()
            .any(|h| h.rule == "内核外的税率乘除公式" && h.text.contains("tax_pct")),
        "改回乘式的行必须被点名为 内核外的税率乘除公式，实际清单 {}",
        format_hits(&hits)
    );
    // 未变异的真实文本必须零命中，防止把内存判红误读成源码判红
    let clean_hits = scan_source(rel, &real);
    assert!(
        clean_hits.is_empty(),
        "{rel} 现状仍有方向破口：\n{}",
        format_hits(&clean_hits)
    );
}

#[test]
fn quotation_switch_legs_route_only_through_the_kernel() {
    // 报价单的含税与不含税两条腿都必须落在内核入口上，
    // 切换口径不得留下 乘法回算含税 的旧形态
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let calc = read_source(&root.join("src/services/quotation_ops/calc.rs"));
    for entry in [
        "tax::split_from_inclusive",
        "tax::split_from_net",
        "tax::net_from_inclusive",
        "tax::assert_three_way_reconcile",
    ] {
        assert!(
            calc.contains(entry),
            "报价单折算入口必须经内核 {entry} 做价税换算，缺失说明该腿又回到本地公式"
        );
    }
    // 建单腿与改单腿都只能调用同一个行折算入口，不得各自再写一遍换算
    for file in [
        "src/services/quotation_ops/calc.rs",
        "src/services/quotation_ops/crud.rs",
        "src/services/quotation_ops/update.rs",
    ] {
        let src = read_source(&root.join(file));
        assert!(
            src.contains("compute_quotation_line"),
            "{file} 必须复用行折算入口 compute_quotation_line，否则同一单据会有两套换算"
        );
        let hits = scan_source(file, &src);
        assert!(
            hits.is_empty(),
            "{file} 上仍有方向破口形态：\n{}",
            format_hits(&hits)
        );
    }
    // 明细行写库的两处腿都不得再出现 数量 乘 单价 直接落金额列的旧形态
    for file in [
        "src/services/quotation_ops/crud.rs",
        "src/services/quotation_ops/update.rs",
    ] {
        let src = strip_comments_and_strings(&read_source(&root.join(file)));
        for (column, price) in [
            ("amount: Set(", "unit_price"),
            ("amount_with_tax: Set(", "unit_price_with_tax"),
        ] {
            for line in src.lines() {
                if line.contains(column) {
                    assert!(
                        !line.contains(price) || line.contains("line."),
                        "{file} 的 {column} 又回到 数量 乘 {price} 的本地公式，改单腿与建单腿必须同源"
                    );
                }
            }
        }
    }
}
