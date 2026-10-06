//! 角色码解析器「正形锁」：把权限矩阵源码解析成集合的解析器，其输出必须与
//! 生产定义表（权威注册表）逐角色码、逐权限元组**双向相等**，且对无效/漂移输入
//! 显式报错而非静默返回空集。
//!
//! 背景（为什么锁"解析器形状"而不是只锁"矩阵内容"）：角色权限矩阵的派生与漂移
//! 度量依赖把 `src/services/init_service_ops/permission.rs` 源码里的角色码/权限码
//! 解析成集合；解析器形状不对（rustfmt 排版变化、分组签名改变、聚合函数改名）时，
//! 度量会得到空集或半集——空集令"无漂移"断言恒真（假绿），半集令"漂移"断言虚报
//! （假漂移）。历史上的矩阵大漂移事件定性即为测量伪影，因此禁止提交自动基线；
//! 本锁把"探测器本身有检测力"固化为契约测试。
//!
//! 判据三层（与文件头一致，逐条对应）：
//! 1. 双向集合相等：解析器对既有全部角色码返回非空集合，且解析结果与权威注册表
//!    （`InitService::all_role_permission_definition_groups()`——生产定义表本身，
//!    不在测试侧另抄矩阵副本）做双向点名，"只断包含"不允许。
//! 2. 无效/漂移输入必须显式报错：分组缺 `&[`、权限清单为空、角色码重复、括号不
//!    闭合、对不存在的角色码取集合——任一形态都要 Err/点名，不得静默给空集。
//! 3. 检测力地板：解析到的角色码数与权限元组总数必须 ≥ 地板常量，空集即判红；
//!    地板取值口径见常量文档注释（防空操作，也防"解析半集"蒙混）。
//!
//! 夹具与断言范式出处（照抄仓内同类锁的实际写法）：
//! - include_str! 读被测源码 + code_only 剔整行注释 + 集合排序去重后双向相等：
//!   `contract_wave7_piece_permission_grant_test.rs`；
//! - 大括号配对取函数体、解析空集即判红："按签名定位函数体"范式见
//!   `contract_wave11_rbac_key_derivation_test.rs`；
//! - 探针自证检测力（注入漂移样例必须被抓到，否则本锁自身就是假绿）：
//!   本文件 `injected_drift_is_detected_by_report` 用例。
//!
//! 本测试为纯静态锁：不连库、不起服务，任何环境（含未 provision 数据库的分片）
//! 都可执行，因此不存在"没连库所以空跑算过"的旁路。

use bingxi_backend::services::init_service::InitService;
use std::collections::BTreeMap;

/// 被测源码：矩阵定义表本体（唯一权威数据源，与生产运行时加载的是同一份）
const MATRIX_SRC: &str = include_str!("../src/services/init_service_ops/permission.rs");

/// 域函数名尾缀：全部域函数以此结尾（定义与调用两侧对齐用它拼接完整函数名）
const DOMAIN_FN_NAME_TAIL: &str = "_role_resources";

/// 域函数签名后缀：定义表按域分组，每个域函数返回一段角色权限定义切片。
/// 字面量须与 `DOMAIN_FN_NAME_TAIL` 的尾缀逐字一致（`concat!` 只接受字面量，
/// 不能由 const 拼出，故此处写全串；两处不一致时本文件的解析必然取空集而判红）。
const DOMAIN_FN_SUFFIX: &str = "_role_resources() -> RoleResourceSlice";

/// 聚合入口签名：全部域函数经它汇总为矩阵定义表（闸门与落库链的同一入口）
const GROUPS_FN_SIG: &str = "pub fn all_role_permission_definition_groups";

/// 检测力地板·角色码数：取当前实测（39）的约七成。地板只用于检出"空集/半集"
/// 形态的解析退化，不参与内容判定；矩阵若在真实演进中增删角色，两侧同源相等
/// 判据不受影响，只有解析器退化到漏掉大片分组时才会击穿地板。
const MIN_MATRIX_ROLE_CODES: usize = 25;

/// 检测力地板·权限元组总数：取当前实测（483）的约六成，口径同上。
const MIN_MATRIX_PERM_PAIRS: usize = 300;

type PermPairStr = (String, String);
type ParsedMatrix = BTreeMap<String, Vec<PermPairStr>>;

/// 只保留"代码 + 字符串字面量"：整行注释（`//`/`///`/`//!`）逐行剔除（同类锁同款
/// 口径：说明性注释不是矩阵条目，计入即假判；宁少剥不可错吃代码）。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_ws_byte(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\r' | b'\n')
}

fn skip_ws(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && is_ws_byte(b[i]) {
        i += 1;
    }
    i
}

fn skip_ws_and_commas(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && (is_ws_byte(b[i]) || b[i] == b',') {
        i += 1;
    }
    i
}

/// 从 `open_idx` 起对 `open`/`close` 做配对扫描，返回闭合符下标（含）。
/// 矩阵文本里括号只承载结构（字符串字面量内无括号，资源/动作码字符集受限，
/// read_quoted 会先把越界字符显式报错），故不另做字符串态跳过。
fn match_bracket(b: &[u8], open_idx: usize, open: u8, close: u8) -> Result<usize, String> {
    if b.get(open_idx) != Some(&open) {
        return Err(format!(
            "第 {open_idx} 字节期望 `{}`，实际不是（结构漂移必须点名，不得静默）",
            open as char
        ));
    }
    let mut depth = 0usize;
    for (i, c) in b.iter().enumerate().skip(open_idx) {
        if *c == open {
            depth += 1;
        } else if *c == close {
            depth -= 1;
            if depth == 0 {
                return Ok(i);
            }
        }
    }
    Err(format!(
        "自第 {open_idx} 字节起的 `{}` 未闭合——括号不闭合即解析失败，禁止返回半集",
        open as char
    ))
}

/// 读取一个双引号字面量 token，返回 (token, 闭引号后的下标)。
/// 拒绝转义与非 ASCII 字节：矩阵码一律是受限 ASCII 标识符，越界形态直接点名。
fn read_quoted(b: &[u8], i: usize) -> Result<(String, usize), String> {
    if b.get(i) != Some(&b'"') {
        return Err(format!(
            "第 {i} 字节期望 `\"` 起始的字符串字面量，实际不是（解析形状与源码形状已分叉）"
        ));
    }
    let mut tok = String::new();
    let mut j = i + 1;
    loop {
        match b.get(j) {
            Some(b'"') => return Ok((tok, j + 1)),
            Some(&b'\\') => {
                return Err(format!(
                    "第 {j} 字节出现转义——矩阵码字面量不应含转义，解析口径失效须点名"
                ));
            }
            Some(&c) if c >= 0x80 => {
                return Err(format!(
                    "第 {j} 字节出现非 ASCII 码位（{}）——矩阵码 token 必须是受限 ASCII 标识符",
                    c
                ));
            }
            Some(&c) => {
                tok.push(c as char);
                j += 1;
            }
            None => return Err("字符串字面量未闭合（截断的源码不得静默解析成半集）".to_string()),
        }
    }
}

fn valid_role_code(code: &str) -> Result<(), String> {
    let ok = !code.is_empty() && code.bytes().all(|c| c.is_ascii_lowercase() || c == b'_');
    if ok {
        Ok(())
    } else {
        Err(format!("非法角色码 `{code}`（应全部为小写字母/下划线）"))
    }
}

fn valid_resource_code(res: &str) -> Result<(), String> {
    let ok = !res.is_empty()
        && res
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' || c == b'_');
    if ok {
        Ok(())
    } else {
        Err(format!("非法资源码 `{res}`（应全部为小写字母/数字/-/_）"))
    }
}

fn valid_action_code(act: &str) -> Result<(), String> {
    let ok = !act.is_empty() && act.bytes().all(|c| c.is_ascii_lowercase() || c == b'*');
    if ok {
        Ok(())
    } else {
        Err(format!("非法操作码 `{act}`（应全部为小写字母或 `*`）"))
    }
}

/// 解析一段角色分组体（外层数组 `(` 之后的内容），产出 (角色码, 权限元组清单)。
/// 空权限清单直接报错——"静默返回空集合"正是本锁要禁掉的失败形态。
fn parse_pairs(seg: &[u8]) -> Result<Vec<PermPairStr>, String> {
    let mut out = Vec::new();
    let mut i = 0;
    loop {
        i = skip_ws_and_commas(seg, i);
        if i >= seg.len() {
            break;
        }
        if seg[i] != b'(' {
            return Err(format!(
                "权限元组应在第 {i} 字节以 `(` 起始，实际是 `{}`——分组内部形状漂移必须点名",
                seg[i] as char
            ));
        }
        let j = skip_ws(seg, i + 1);
        let (res, k) = read_quoted(seg, j)?;
        valid_resource_code(&res)?;
        let j = skip_ws(seg, k);
        if seg.get(j) != Some(&b',') {
            return Err(format!("资源码 `{res}` 之后缺逗号分隔（第 {j} 字节）"));
        }
        let j = skip_ws(seg, j + 1);
        let (act, k) = read_quoted(seg, j)?;
        valid_action_code(&act)?;
        let j = skip_ws(seg, k);
        if seg.get(j) != Some(&b')') {
            return Err(format!(
                "权限元组 `(\"{res}\", \"{act}\")` 未以 `)` 闭合（第 {j} 字节）"
            ));
        }
        out.push((res, act));
        i = j + 1;
    }
    if out.is_empty() {
        return Err("解析到空权限清单——空集即判红，禁止静默放行".to_string());
    }
    Ok(out)
}

/// 解析一个域函数的外层数组内容：逐个 `( "角色码", &[元组...] )` 分组。
/// 任一环节的期望字符不匹配一律 Err（形状漂移不许退化成"跳过该组继续"）。
fn parse_groups(arr: &[u8]) -> Result<Vec<(String, Vec<PermPairStr>)>, String> {
    let mut groups = Vec::new();
    let mut i = 0;
    loop {
        i = skip_ws_and_commas(arr, i);
        if i >= arr.len() {
            break;
        }
        if arr[i] != b'(' {
            return Err(format!(
                "角色分组应在第 {i} 字节以 `(` 起始，实际是 `{}`——外层数组里出现非分组形态项",
                arr[i] as char
            ));
        }
        let j = skip_ws(arr, i + 1);
        let (role, k) = read_quoted(arr, j)?;
        valid_role_code(&role)?;
        let j = skip_ws(arr, k);
        if arr.get(j) != Some(&b',') {
            return Err(format!("角色码 `{role}` 之后缺逗号（第 {j} 字节）"));
        }
        let j = skip_ws(arr, j + 1);
        if !(arr.get(j) == Some(&b'&') && arr.get(j + 1) == Some(&b'[')) {
            return Err(format!(
                "角色码 `{role}` 的定义不是 `&[` 起始的权限清单（第 {j} 字节）——分组形状被改写，解析器必须红而不是跳过"
            ));
        }
        let g_close = match_bracket(arr, j + 1, b'[', b']')?;
        let pairs = parse_pairs(&arr[j + 2..g_close])?;
        // 分组元组收尾：`( "角色码", &[...]` 之后（允许列表尾逗号与空白）必须闭合
        // 回 `)`，形状不对同样点名
        let g_end = skip_ws_and_commas(arr, g_close + 1);
        if arr.get(g_end) != Some(&b')') {
            return Err(format!(
                "角色码 `{role}` 的分组元组未以 `)` 收尾（第 {g_end} 字节）——外层数组里出现非分组形态项"
            ));
        }
        groups.push((role, pairs));
        i = g_end + 1;
    }
    if groups.is_empty() {
        return Err("域数组里解析到 0 个角色分组——空集即判红".to_string());
    }
    Ok(groups)
}

/// 从整份矩阵源码解析出 角色码→权限元组清单 的映射（本锁的被测解析器）。
fn parse_matrix(src: &str) -> Result<ParsedMatrix, String> {
    let code = code_only(src);
    let bytes = code.as_bytes();
    let mut map: ParsedMatrix = BTreeMap::new();
    let mut domain_count = 0usize;
    let mut pos = 0usize;
    while let Some(rel) = code[pos..].find(DOMAIN_FN_SUFFIX) {
        let sig_at = pos + rel;
        domain_count += 1;
        let body_open = match code[sig_at..].find('{') {
            Some(off) => sig_at + off,
            None => {
                return Err(
                    "域函数签名后找不到函数体起始 `{`（只剩声明？定义表形态已改变）".to_string(),
                );
            }
        };
        let body_close = match_bracket(bytes, body_open, b'{', b'}')?;
        let body = &code.as_bytes()[body_open..=body_close];
        let arr_open = body
            .iter()
            .position(|&c| c == b'[')
            .ok_or_else(|| "域函数体内没有 `&[` 数组字面量——返回表达式形态已改变".to_string())?;
        let arr_close = match_bracket(body, arr_open, b'[', b']')?;
        for (role, pairs) in parse_groups(&body[arr_open + 1..arr_close])? {
            if map.insert(role.clone(), pairs).is_some() {
                return Err(format!(
                    "角色码 `{role}` 在矩阵定义表里出现多次——聚合结果不确定，必须点名而不是后者覆盖前者"
                ));
            }
        }
        pos = body_close + 1;
    }
    if domain_count == 0 {
        return Err(format!(
            "未找到任何 `{}` 形态的域函数——解析器与定义表形状已分叉，空跑不得算通过",
            DOMAIN_FN_SUFFIX
        ));
    }
    if map.is_empty() {
        return Err("解析结果为空映射（空集即判红）".to_string());
    }
    let real_pairs: usize = map.values().map(|v| v.len()).sum();
    if real_pairs == 0 {
        return Err("解析到的权限元组总数为 0（空集即判红）".to_string());
    }
    Ok(map)
}

/// 权威注册表：生产定义表本体（聚合函数直接产出，非测试侧抄本）。
/// 与解析器同源同形才叫锁；这里若出现重复角色码，说明聚合函数契约已坏，直接 panic 点名。
fn production_matrix() -> ParsedMatrix {
    let mut map: ParsedMatrix = BTreeMap::new();
    for slice in InitService::all_role_permission_definition_groups() {
        for (role_code, pairs) in slice {
            let entry: Vec<PermPairStr> = pairs
                .iter()
                .map(|(res, act)| (res.to_string(), act.to_string()))
                .collect();
            if map.insert(role_code.to_string(), entry).is_some() {
                panic!("权威定义表自身存在重复角色码 `{role_code}`——聚合契约已破坏，本锁判红");
            }
        }
    }
    assert!(
        !map.is_empty(),
        "权威定义表为空——生产聚合函数返回空集时本锁失去意义，判红并要求先修复聚合函数"
    );
    map
}

fn sorted_pairs(v: &[PermPairStr]) -> Vec<PermPairStr> {
    let mut out: Vec<PermPairStr> = v.to_vec();
    out.sort();
    out.dedup();
    out
}

/// 双向漂移点名器：角色码层面（缺/多）与权限元组层面（缺/多）都输出。
/// 本锁的"只断相等"不写成裸 assert_eq，是为了把对称差直接打进失败消息，
/// 让判责时一眼看出漂的是哪个角色码的哪个元组。
fn drift_report(parsed: &ParsedMatrix, prod: &ParsedMatrix) -> Vec<String> {
    let mut report = Vec::new();
    for code in prod.keys().filter(|c| !parsed.contains_key(*c)) {
        report.push(format!("解析缺角色码 `{code}`（权威侧在册）"));
    }
    for code in parsed.keys().filter(|c| !prod.contains_key(*c)) {
        report.push(format!("解析多角色码 `{code}`（权威侧无此码）"));
    }
    for (code, prod_pairs) in prod {
        if let Some(parsed_pairs) = parsed.get(code) {
            let p = sorted_pairs(parsed_pairs);
            let q = sorted_pairs(prod_pairs);
            for miss in q.iter().filter(|t| !p.contains(*t)) {
                report.push(format!(
                    "角色 `{code}` 解析缺少权限元组 (\"{}\", \"{}\")",
                    miss.0, miss.1
                ));
            }
            for extra in p.iter().filter(|t| !q.contains(*t)) {
                report.push(format!(
                    "角色 `{code}` 解析多出权限元组 (\"{}\", \"{}\")",
                    extra.0, extra.1
                ));
            }
        }
    }
    report
}

/// 取某个角色码的解析集合：不在册者显式报错（无效输入不得退化成空集）。
fn role_pairs<'a>(parsed: &'a ParsedMatrix, code: &str) -> Result<&'a Vec<PermPairStr>, String> {
    parsed
        .get(code)
        .ok_or_else(|| format!("角色码 `{code}` 不在解析结果中——无效输入必须点名报错，不返回空集"))
}

/// 构造一个最小但形状合法的矩阵源码（供正/负向形态用例复用）。
fn minimal_matrix_src(inner: &str) -> String {
    format!("    fn demo_role_resources() -> RoleResourceSlice {{\n{inner}\n    }}\n")
}

const VALID_INNER: &str = "        &[\n            (\n                \"demo_role\",\n                &[(\"demo-res\", \"read\")],\n            ),\n        ]";

/// 判据①+③：解析器对既有全部角色码返回非空集合，且与权威注册表双向相等；
/// 两侧计数都必须 ≥ 检测力地板（空集/半集即判红）。
#[test]
fn role_code_parser_output_equals_production_registry_bidirectionally() {
    let prod = production_matrix();
    let parsed = parse_matrix(MATRIX_SRC).unwrap_or_else(|e| {
        panic!("解析器对现行矩阵源码必须成功产出全量集合，否则本锁失去被测对象: {e}")
    });

    assert!(
        prod.len() >= MIN_MATRIX_ROLE_CODES,
        "权威定义表角色码数 {} 低于地板 {}——定义表退化到异常规模，本锁判红",
        prod.len(),
        MIN_MATRIX_ROLE_CODES
    );
    assert!(
        parsed.len() >= MIN_MATRIX_ROLE_CODES,
        "解析器角色码数 {} 低于地板 {}——只解析出半集（或空集）正是历史上的测量伪影形态，判红",
        parsed.len(),
        MIN_MATRIX_ROLE_CODES
    );
    let prod_total: usize = prod.values().map(|v| v.len()).sum();
    let parsed_total: usize = parsed.values().map(|v| v.len()).sum();
    assert!(
        prod_total >= MIN_MATRIX_PERM_PAIRS,
        "权威定义表权限元组总数 {prod_total} 低于地板 {}，判红",
        MIN_MATRIX_PERM_PAIRS
    );
    assert!(
        parsed_total >= MIN_MATRIX_PERM_PAIRS,
        "解析器权限元组总数 {parsed_total} 低于地板 {}——半集解析判红",
        MIN_MATRIX_PERM_PAIRS
    );

    let report = drift_report(&parsed, &prod);
    assert!(
        report.is_empty(),
        "解析器输出与权威定义表必须双向相等（缺/多角色码、缺/多权限元组都算漂移）：{report:?}"
    );

    // 逐角色非空：对既有全部角色码都返回非空集合（"与注册表相等"只断合集会漏掉
    // 两边同为空集的退化形态，这里再按角色码逐个显式打一遍）
    for (code, pairs) in &parsed {
        assert!(
            !pairs.is_empty(),
            "角色码 `{code}` 解析到空权限集合——静默空集是本锁禁止的失败形态"
        );
    }
    for (code, pairs) in &prod {
        assert!(
            !pairs.is_empty(),
            "角色码 `{code}` 在权威定义表里就是空集合——上游契约已坏，判红"
        );
    }
}

/// 判据②（其一）：无效/漂移输入必须显式报错，而不是静默返回空集合。
#[test]
fn invalid_or_drifted_input_errors_explicitly() {
    // 正向对照：合法最小形状必须 Ok——否则下面的负向 Err 全是解析器"见谁都拒"，
    // 不具备区分能力，本用例就退化成另一种假锁。
    let ok_src = minimal_matrix_src(VALID_INNER);
    let ok = parse_matrix(&ok_src);
    assert!(
        ok.is_ok(),
        "合法最小矩阵样例必须解析成功（解析器与合法形状失配时负向用例没有意义）: {ok:?}"
    );

    // 空源码 / 无定义表形状的源码
    assert!(parse_matrix("").is_err(), "空源码不得解析成功");
    assert!(
        parse_matrix("fn nothing_here() {}").is_err(),
        "不含任何域函数签名的源码必须报错，不得静默给空映射"
    );

    // 漂移：角色码后面不是 `&[`（分组形状被改写）
    let broken_array = minimal_matrix_src(
        "        &[\n            (\n                \"demo_role\",\n                [(\"demo-res\", \"read\")],\n            ),\n        ]",
    );
    assert!(
        parse_matrix(&broken_array).is_err(),
        "`\"角色码\",` 后缺 `&` 的分组形态必须显式报错（不得跳过该组继续凑合解析）"
    );

    // 漂移：空权限清单（静默空集的直接形态）
    let empty_list = minimal_matrix_src(
        "        &[\n            (\n                \"demo_role\",\n                &[],\n            ),\n        ]",
    );
    assert!(
        parse_matrix(&empty_list).is_err(),
        "`&[]` 空权限清单必须报错——空集合静默在场正是矩阵假绿的入口"
    );

    // 漂移：角色码重复（聚合语义不确定）
    let duplicated = minimal_matrix_src(
        "        &[\n            (\n                \"demo_role\",\n                &[(\"demo-res\", \"read\")],\n            ),\n            (\n                \"demo_role\",\n                &[(\"demo-res\", \"update\")],\n            ),\n        ]",
    );
    assert!(
        parse_matrix(&duplicated).is_err(),
        "同一角色码出现两次必须点名，不得后者覆盖前者静默合并"
    );

    // 漂移：函数体括号不闭合（截断源码不得解析出半集）
    let unclosed = "    fn demo_role_resources() -> RoleResourceSlice {\n        &[\n            (\n                \"demo_role\",\n                &[(\"demo-res\", \"read\")],\n            ),\n        ]\n";
    assert!(
        parse_matrix(unclosed).is_err(),
        "域函数体未闭合必须报错——把半截源码当成一个更小的合法矩阵就是假绿"
    );

    // 真实矩阵上的定向破坏：抹掉 auditor 的角色码行 → 分组起始不再是字符串字面量，
    // 解析器必须红（证明它对现行源码同样具备形状检错力，不是只对合成样例敏感）
    let mutated = MATRIX_SRC.replace("\"auditor\",", "");
    assert!(
        parse_matrix(&mutated).is_err(),
        "从真实矩阵里抹掉一个角色码后必须显式报错，而不是少解析一组还宣称成功"
    );

    // 对不存在的角色码取集合：显式报错而非返回空集
    let parsed = parse_matrix(MATRIX_SRC).expect("真实矩阵必须可解析（同判据①用例）");
    assert!(
        role_pairs(&parsed, "role_code_that_definitely_does_not_exist").is_err(),
        "无效角色码查询必须点名报错"
    );
}

/// 判据②（其二，探针性质）：注入"形状合法但内容漂移"的样例，必须被
/// drift_report 抓到——证明这条锁本身有检测力，而不是恒真断言。
#[test]
fn injected_drift_is_detected_by_report() {
    let prod = production_matrix();
    let clean = parse_matrix(MATRIX_SRC).expect("真实矩阵必须可解析（同判据①用例）");
    assert!(
        drift_report(&clean, &prod).is_empty(),
        "对照基线：未注入漂移时双向点名器必须无输出"
    );

    // 注入形态 1：同形内容漂移——把一个操作码改成形状合法的别的词
    let mutated = MATRIX_SRC.replace("(\"audit-logs\", \"read\")", "(\"audit-logs\", \"readz\")");
    assert_ne!(
        mutated.len(),
        MATRIX_SRC.len(),
        "注入未生效（needle 与源码失配）——探针自身失效必须红，否则本用例空跑"
    );
    let drifted = parse_matrix(&mutated).expect("注入样例形状合法，应可解析（检测发生在对比层）");
    let report = drift_report(&drifted, &prod);
    assert!(
        !report.is_empty(),
        "注入的内容漂移没有被点名——探测器无检测力，本锁属假绿，判红"
    );
    assert!(
        report.iter().any(|r| r.contains("readz")),
        "点名结果必须指明漂移的具体元组: {report:?}"
    );

    // 注入形态 2：凭空多出一个权威侧不存在的角色码（矩阵侧"一边多"的漂移）
    let mut extra = prod.clone();
    extra.insert(
        "ghost_role_for_probe".to_string(),
        vec![("dashboard".to_string(), "read".to_string())],
    );
    let report2 = drift_report(&clean, &extra);
    assert!(
        report2
            .iter()
            .any(|r| r.contains("ghost_role_for_probe") && r.contains("缺少角色码")),
        "权威侧凭空多出的角色码必须被点名: {report2:?}"
    );
}

/// 聚合入口的调用顺序与域函数定义顺序必须同源：两处顺序一旦不同步，
/// 定义表聚合出的矩阵顺序与源码阅读顺序漂移，闸门/日志点名顺序都失去可比对性。
#[test]
fn aggregation_call_order_matches_definition_order() {
    let code = code_only(MATRIX_SRC);
    let bytes = code.as_bytes();

    // 定义顺序：按签名在源码中出现的先后
    let mut def_order: Vec<String> = Vec::new();
    let mut pos = 0usize;
    while let Some(rel) = code[pos..].find(DOMAIN_FN_SUFFIX) {
        let sig_at = pos + rel;
        let mut s = sig_at;
        while s > 0 && (bytes[s - 1].is_ascii_lowercase() || bytes[s - 1] == b'_') {
            s -= 1;
        }
        def_order.push(format!("{}{}", &code[s..sig_at], DOMAIN_FN_NAME_TAIL));
        let body_open = sig_at + code[sig_at..].find('{').expect("域函数签名后必须有函数体");
        let body_close = match_bracket(bytes, body_open, b'{', b'}').expect("域函数体必须闭合");
        pos = body_close + 1;
    }

    // 调用顺序：聚合函数体内 Self::<name>() 的出现先后
    let sig_at = code
        .find(GROUPS_FN_SIG)
        .expect("聚合入口签名必须存在（改名/移动须同步本锁）");
    let body_open = sig_at + code[sig_at..].find('{').expect("聚合入口必须有函数体");
    let body_close = match_bracket(bytes, body_open, b'{', b'}').expect("聚合入口函数体必须闭合");
    let body = &code[body_open..=body_close];
    let mut call_order: Vec<String> = Vec::new();
    let mut cpos = 0usize;
    while let Some(rel) = body[cpos..].find("Self::") {
        let s = cpos + rel + "Self::".len();
        let mut e = s;
        while e < body.len()
            && (body.as_bytes()[e].is_ascii_lowercase() || body.as_bytes()[e] == b'_')
        {
            e += 1;
        }
        let name = &body[s..e];
        if name.ends_with("_role_resources") {
            call_order.push(name.to_string());
        }
        cpos = e.max(cpos + 1);
    }

    assert!(
        !def_order.is_empty() && !call_order.is_empty(),
        "定义/调用两侧顺序清单不得为空（空集即判红）"
    );
    assert_eq!(
        call_order.len(),
        def_order.len(),
        "聚合入口调用的域函数数量 {} 与源码定义数量 {} 不一致——存在未聚合或已失焦的域函数",
        call_order.len(),
        def_order.len()
    );
    assert_eq!(
        call_order, def_order,
        "聚合调用顺序与定义顺序不同步——矩阵聚合入口与定义表发生了顺序漂移"
    );
}
