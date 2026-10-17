//! 权限资源键同源锁：URL 段消歧表的目标名必须全部登记在资源注册表内。
//!
//! 判据链（RBAC 中间件按 URL 段推导权限键，非风格问题）：
//! - 运行时键 = `middleware/permission.rs::extract_resource_info` 取 seg4（或双层前缀时取
//!   seg5）后交 `utils/path_utils.rs::resolve_module_prefixed_resource` 消歧的结果；
//! - 授权行只有资源名与该键逐字符相同才命中（`matches_permission` 精确串匹配），
//!   注册表 `init_service::PERMISSION_RESOURCES` 是资源名的唯一权威清单。
//! 二者漂移时后果是双向的：注册表外的派生键使已登记的授权行变成永不生效的死码，
//! 而同名资源若被随手登记进注册表，就等于把权限键名定在权威清单之外、由 URL 决定词表。
//! 故本锁同时钉两侧：消歧目标 ⊆ 注册表，且不靠"补登记别名"绕过。

use bingxi_backend::services::init_service::PERMISSION_RESOURCES;

fn read_src(rel: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {} 失败: {e}", path.display()))
}

/// 取 `pub fn <name>` 的函数体（按大括号配对截取，注释里的括号不参与配对）。
fn fn_body(src: &str, sig: &str) -> String {
    let start = src
        .find(sig)
        .unwrap_or_else(|| panic!("源文件里找不到签名 {sig}，派生规则被改名或移动须同步本锁"));
    let open = src[start..]
        .find('{')
        .unwrap_or_else(|| panic!("签名 {sig} 后面没有函数体起始大括号"));
    let body_start = start + open;
    let bytes = src.as_bytes();
    let mut depth = 0usize;
    let mut end = body_start;
    for (i, ch) in bytes.iter().enumerate().skip(body_start) {
        match ch {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }
    src[body_start..=end].to_string()
}

/// 抽出 RESOURCE_NAME_MAP 的每一行（模块前缀，URL 段，注册表权威资源名）。
/// 定位先越过声明到赋值号，再取其右侧首个方括号，否则会把类型标注的括号当成表体起始。
/// 表体先剥净空白再按引号对取字面量，故 rustfmt 把长行折成多行元组也不改变判据。
/// 调用方是本文件两条同源锁。入参为 utils/path_utils.rs 全文，返回三元组向量。
fn map_rows(src: &str) -> Vec<(String, String, String)> {
    let decl = src
        .find("const RESOURCE_NAME_MAP")
        .expect("映射表声明被改名或移除，须同步本锁的读取真源");
    let tail = &src[decl..];
    let rhs = &tail[tail.find('=').expect("映射表声明没有赋值右侧")..];
    let open = rhs.find('[').expect("映射表右侧不是数组字面量") + 1;
    let mut depth = 1usize;
    let mut close = 0usize;
    for (i, ch) in rhs[open..].char_indices() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    close = open + i;
                    break;
                }
            }
            _ => {}
        }
    }
    assert!(close > open, "映射表数组字面量没有闭合方括号");
    let flat: String = rhs[open..close]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let mut literals = Vec::new();
    let mut rest = flat.as_str();
    while let Some(start) = rest.find('"') {
        let after = &rest[start + 1..];
        match after.find('"') {
            Some(end) => {
                literals.push(after[..end].to_string());
                rest = &after[end + 1..];
            }
            None => break,
        }
    }
    assert!(
        literals.len() % 3 == 0,
        "映射表字面量数不是 3 的倍数，表形态与本锁读取口径已漂移"
    );
    literals
        .chunks(3)
        .map(|row| (row[0].clone(), row[1].clone(), row[2].clone()))
        .collect()
}

/// 消歧表的每一个目标资源名都必须已登记在注册表内，否则消歧结果仍是死码授权。
/// 读本表本体（资源名唯一真相）而非查表函数的写法，消歧规则换形态不必回来改口径。
#[test]
fn every_disambiguation_target_is_registered() {
    let path_utils = read_src("utils/path_utils.rs");
    let rows = map_rows(&path_utils);
    assert!(
        !rows.is_empty(),
        "映射表解析结果为空，本锁读取口径与 RESOURCE_NAME_MAP 形态已漂移，须同步改解析口径，禁止让它空跑"
    );
    // 同一（模块前缀，URL 段）只许一行，两行不同目标则派生出的资源名不确定
    let mut seen: Vec<(&str, &str)> = Vec::new();
    for (prefix, segment, _) in &rows {
        let key = (prefix.as_str(), segment.as_str());
        assert!(
            !seen.contains(&key),
            "映射表里 ({}, {}) 出现多行，同一 URL 段派生出的资源名不确定",
            key.0,
            key.1
        );
        seen.push(key);
    }
    for (_, _, target) in &rows {
        assert!(
            PERMISSION_RESOURCES.contains(&target.as_str()),
            "消歧目标 `{target}` 未登记在 PERMISSION_RESOURCES，消歧后的授权行仍永不生效"
        );
    }
    // 锚住检测力，函数若不引用本表则表成死码，派生键静默退回 URL 段原名
    let body = fn_body(&path_utils, "pub fn resolve_module_prefixed_resource");
    assert!(
        body.contains("RESOURCE_NAME_MAP"),
        "resolve_module_prefixed_resource 不再引用 RESOURCE_NAME_MAP，消歧永不生效"
    );
}

/// `/crm/leads` 的资源键消歧到规范码 crm-leads：注册表权威名是 crm-leads，
/// 而 `leads` 不在注册表内（禁止用"把 leads 补进注册表"来绕过消歧）。
#[test]
fn crm_leads_derives_to_registered_canonical_code() {
    let path_utils = read_src("utils/path_utils.rs");
    let rows = map_rows(&path_utils);
    let leads = rows
        .iter()
        .find(|row| row.0.as_str() == "crm" && row.1.as_str() == "leads")
        .unwrap_or_else(|| {
            panic!("映射表缺少 crm 前缀下 leads 这一行，线索的资源键退回注册表外的裸段名")
        });
    assert_eq!(
        leads.2, "crm-leads",
        "/crm/leads 必须消歧到 crm-leads，否则矩阵里已登记的线索授权行是死码"
    );
    assert!(
        PERMISSION_RESOURCES.contains(&"crm-leads"),
        "crm-leads 必须是注册表内的权威资源名"
    );
    assert!(
        !PERMISSION_RESOURCES.contains(&"leads"),
        "禁止把 URL 段原名 leads 登记进注册表来绕过消歧（会形成第二套资源名）"
    );

    // 消歧生效的两个前提：crm 被识别为模块前缀；("crm","leads") 不是双层前缀组合
    //（否则资源名取自 seg5，本分支永不参与）。
    // 前缀表真源读法：`pub fn is_module_prefix` 只是**分发器**
    //（path_utils.rs:12-14，体内只有两个分发调用、零前缀字面量），前缀字面量在其
    // 分发目标 `fn is_system_module_prefix` / `fn is_business_module_prefix` 两张表内
    //（"crm" 现落 business 表 path_utils.rs:75）。只读分发器函数体必然永缺字面量，
    // 故本锁沿分发链读：先钉分发关系未漂移，再在两张分发表的并集内查 "crm"。
    // 检测力保持：任一分发表删掉 "crm" ⇒ 并集不含 ⇒ 判红；分发关系改名/换目标 ⇒
    // 分发前提判红——两条断言各自指向可定位的漂移，不出现空跑。
    let dispatch = fn_body(&path_utils, "pub fn is_module_prefix");
    assert!(
        dispatch.contains("is_system_module_prefix")
            && dispatch.contains("is_business_module_prefix"),
        "is_module_prefix 的分发目标改变——前缀表真源已移位，本锁读法必须同步改，禁止让它空跑"
    );
    let prefix_table_union = fn_body(&path_utils, "fn is_system_module_prefix")
        + &fn_body(&path_utils, "fn is_business_module_prefix");
    assert!(
        prefix_table_union.contains("\"crm\""),
        "crm 必须在模块前缀表内（is_module_prefix 两张分发表之一），否则 seg4 原样作键、消歧分支永不命中"
    );
    let nested = fn_body(&path_utils, "pub fn is_nested_module_prefix");
    assert!(
        !nested.contains(r#"("crm", "leads")"#),
        "crm/leads 不得被登记为双层前缀组合（会把资源键漂到 seg5）"
    );
}
