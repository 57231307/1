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

/// 抽出消歧分支的箭头目标字面量（`=> "xxx".to_string()`），默认分支无字面量故不入选。
fn resolved_literals(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = body;
    while let Some(idx) = rest.find("=> \"") {
        let after = &rest[idx + 4..];
        match after.find('"') {
            Some(end) => {
                out.push(after[..end].to_string());
                rest = &after[end..];
            }
            None => break,
        }
    }
    out
}

/// 消歧表的每一个目标资源名都必须已登记在注册表内，否则消歧结果仍是死码授权。
#[test]
fn every_disambiguation_target_is_registered() {
    let path_utils = read_src("utils/path_utils.rs");
    let body = fn_body(&path_utils, "pub fn resolve_module_prefixed_resource");
    let targets = resolved_literals(&body);
    assert!(
        !targets.is_empty(),
        "消歧分支解析结果为空——函数形态改变会让本锁空跑，须同步改解析口径"
    );
    for target in &targets {
        assert!(
            PERMISSION_RESOURCES.contains(&target.as_str()),
            "消歧目标 `{target}` 未登记在 PERMISSION_RESOURCES，消歧后的授权行仍永不生效"
        );
    }
}

/// `/crm/leads` 的资源键消歧到规范码 crm-leads：注册表权威名是 crm-leads，
/// 而 `leads` 不在注册表内（禁止用"把 leads 补进注册表"来绕过消歧）。
#[test]
fn crm_leads_derives_to_registered_canonical_code() {
    let path_utils = read_src("utils/path_utils.rs");
    let body = fn_body(&path_utils, "pub fn resolve_module_prefixed_resource");
    assert!(
        body.contains(r#"("crm", "leads") => "crm-leads".to_string(),"#),
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
    // 前缀表真源读法（CI #4677 判责修正）：`pub fn is_module_prefix` 只是**分发器**
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
