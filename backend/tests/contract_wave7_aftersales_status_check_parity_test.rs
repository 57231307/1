//! 售后工单状态词表三端同源契约锁（波7，CI 用例 65-01 收口批次）
//!
//! 纯源码/迁移文本比对，不连库（取值集合是否违反真表约束由迁移 up 的
//! fail-visible 存量检查与 `contract_wave3_after_sales_customer_name_test.rs`
//! 的真库更新用例共同裁决，本锁防的是"三端词表漂移"本身）。
//!
//! 锁定三端逐 token 双向相等：
//! ① 写入方权威词表 `models/status/sales.rs::custom_order_ext::AFTERSALES_ALL`
//!    （常量值集合 == ALL 列举集合）；
//! ② 服务层唯一写入方状态机
//!    `services/custom_order_aftersales_service.rs::AFTERSALES_TRANSITIONS`
//!    （源节点集合 == ALL 常量名集合；目标节点 ⊆ 词表，杜绝词表外流转）；
//! ③ 生效 CHECK `chk_aftersales_status` 取值集
//!    （`migration/src/domain/production/m0067_*.rs` 的 ALLOWED_STATUS_VALUES
//!    == AFTERSALES_ALL 值集合，双向；约束名与 m0044 一致不得改；
//!    down 段回滚集合钉死为 m0044 原 5 值——回滚语义不许悄悄扩集）。
//!
//! 任何一端加/删/改 token 而其余两端未同步，本用例即红，
//! 防止 accepted/evaluated 这类"写入方已可达、约束未覆盖"的裸 500 再次回归。
//!
//! ④ 类型族同型锁（本批次收口 m0074）：写入方 create 白名单
//!    （`services/custom_order_aftersales_service.rs:151`，5 值含 return_goods）
//!    == 生效 CHECK `chk_aftersales_type` 取值集（m0074 重建）
//!    == 前端候选四处（AfterSalesPanel radio / getIssueTypeLabel known /
//!    api/custom-order.ts AFTER_SALES_TYPE / zh-CN 与 en-US issueType 键）。
//!    历史缺陷同型：m0044 CHECK 4 值而写入方/前端 5 值，创建"退货"工单撞
//!    CHECK 冒 DATABASE_ERROR(500)。m0074 down 回滚集合亦钉死 m0044 原 4 值。

use std::collections::BTreeSet;

/// 从 sales.rs 提取 AFTERSALES_* 单值常量表（名 → 值）与 AFTERSALES_ALL 列举名。
fn parse_sales_authority() -> (BTreeSet<String>, BTreeSet<String>, BTreeSet<String>) {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/models/status/sales.rs");
    let src = std::fs::read_to_string(path).expect("读取 models/status/sales.rs 失败");

    let mut consts: BTreeSet<String> = BTreeSet::new(); // 常量名
    let mut values: BTreeSet<String> = BTreeSet::new(); // 常量值（进 CHECK 的 token）
    for line in src.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("pub const AFTERSALES_") {
            if let Some((name, tail)) = rest.split_once(": &str = ") {
                let val = tail
                    .strip_prefix('"')
                    .and_then(|v| v.split('"').next())
                    .expect("AFTERSALES_* 常量值必须为双引号字面量");
                let full_name = format!("AFTERSALES_{name}");
                // ALL 里的值集合排除 ALL 自身（AFTERSALES_ALL 不是状态 token）
                if full_name != "AFTERSALES_ALL" {
                    consts.insert(full_name.clone());
                    values.insert(val.to_string());
                }
            }
        }
    }

    // AFTERSALES_ALL 块内列举的常量名
    let all_start = src
        .find("pub const AFTERSALES_ALL: &[&str] = &[")
        .expect("sales.rs 缺少 AFTERSALES_ALL 声明");
    let all_block = &src[all_start..];
    let all_end = all_block.find("];").expect("AFTERSALES_ALL 块未闭合");
    let mut all_names: BTreeSet<String> = BTreeSet::new();
    for line in all_block[..all_end + 2].lines() {
        let t = line.trim().trim_end_matches(',');
        if t.starts_with("AFTERSALES_") && t != "AFTERSALES_ALL" {
            all_names.insert(t.to_string());
        }
    }
    (consts, values, all_names)
}

/// 从服务层提取 AFTERSALES_TRANSITIONS 的源节点名与目标节点名集合。
/// 判定规则：`ext::NAME` 后随 `,` 再随 `&[` 的是源节点（元组左侧）；其余为目标节点。
fn parse_service_transitions() -> (BTreeSet<String>, BTreeSet<String>) {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/services/custom_order_aftersales_service.rs"
    );
    let src = std::fs::read_to_string(path).expect("读取 custom_order_aftersales_service.rs 失败");
    let start = src
        .find("const AFTERSALES_TRANSITIONS:")
        .expect("服务层缺少 AFTERSALES_TRANSITIONS 声明");
    let block = &src[start..];
    let end = block.find("\n];").expect("AFTERSALES_TRANSITIONS 块未闭合");
    let block = &block[..end + 3];

    let needle = "ext::AFTERSALES_";
    let mut sources: BTreeSet<String> = BTreeSet::new();
    let mut targets: BTreeSet<String> = BTreeSet::new();
    let mut pos = 0usize;
    while let Some(rel) = block[pos..].find(needle) {
        let name_start = pos + rel + needle.len();
        let name_len = block[name_start..]
            .chars()
            .take_while(|c| c.is_ascii_uppercase() || *c == '_')
            .count();
        let name = format!("AFTERSALES_{}", &block[name_start..name_start + name_len]);
        // 角色判定：源节点（元组左侧）名字后为 ',' 再为 '&'（&[ 目标列表开头）；
        // 目标节点名字后为 ',' 再为 ']'/'e'（ext::）（列表分隔或收尾），
        // 或列表末元素名字直接以 ']' 收尾。
        let after_name = block[name_start + name_len..].trim_start();
        match after_name.chars().next() {
            Some(',') => {
                let after_comma = after_name[1..].trim_start();
                match after_comma.chars().next() {
                    Some('&') => {
                        sources.insert(name);
                    }
                    Some(']' | 'e') => {
                        targets.insert(name);
                    }
                    other => panic!("{name} 后 ',' 跟了预期外字符: {other:?}"),
                }
            }
            Some(']') => {
                targets.insert(name);
            }
            other => panic!("{name} 后应紧跟 ',' 或 ']'，实际: {other:?}"),
        }
        pos = name_start + name_len;
    }
    (sources, targets)
}

/// 从迁移源文件 [start_anchor, end_anchor) 区间提取单引号 token 列表。
fn extract_quoted_tokens(path: &str, start_anchor: &str, end_anchor: &str) -> Vec<String> {
    let src =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("读取迁移源文件 {path} 失败: {e}"));
    let start = src
        .find(start_anchor)
        .unwrap_or_else(|| panic!("{path} 缺少锚点: {start_anchor}"));
    let rest = &src[start..];
    let end = rest
        .find(end_anchor)
        .unwrap_or_else(|| panic!("{path} 锚点未闭合: {end_anchor}"));
    let slice = &rest[..end];
    let mut tokens = Vec::new();
    let mut cursor = slice;
    while let Some(open) = cursor.find('\'') {
        cursor = &cursor[open + 1..];
        let close = cursor.find('\'').expect("单引号未闭合");
        tokens.push(cursor[..close].to_string());
        cursor = &cursor[close + 1..];
    }
    tokens
}

// =========================================================
// ① 写入方内部自洽：常量声明集 == AFTERSALES_ALL 列举集
// =========================================================

#[test]
fn sales_authority_constants_match_all_listing() {
    let (consts, values, all_names) = parse_sales_authority();
    assert_eq!(consts.len(), 7, "AFTERSALES_* 单值常量应为 7 个");
    assert_eq!(
        consts, all_names,
        "AFTERSALES_ALL 列举与常量声明不一致（一边改一边忘）"
    );
    assert_eq!(values.len(), 7, "7 个常量值应两两不同");
    for v in &values {
        assert_eq!(
            v,
            &v.to_ascii_lowercase(),
            "售后状态 token 必须全小写（与 DB CHECK 逐字符一致）"
        );
    }
}

// =========================================================
// ② 状态机节点集 == 写入方词表（源节点全集、目标不越词表）
// =========================================================

#[test]
fn service_transitions_nodes_equals_authority_vocabulary() {
    let (consts, _values, all_names) = parse_sales_authority();
    let (sources, targets) = parse_service_transitions();
    assert_eq!(
        sources, all_names,
        "AFTERSALES_TRANSITIONS 源节点集必须与权威词表常量名集逐 token 相等"
    );
    assert_eq!(consts, sources, "状态机源节点与常量声明集不一致");
    for t in &targets {
        assert!(
            all_names.contains(t),
            "状态机目标节点「{t}」不在权威词表内：将写入 CHECK 未覆盖的 token"
        );
    }
}

// =========================================================
// ③ 迁移 CHECK == 写入方词表（双向）；约束名与 down 回滚集合钉死
// =========================================================

#[test]
fn m0067_check_tokens_equals_aftersales_all_bidirectional() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/migration/src/domain/production/m0067_aftersales_status_add_accepted_evaluated.rs"
    );
    let migration_tokens: BTreeSet<String> = extract_quoted_tokens(
        path,
        "const ALLOWED_STATUS_VALUES",
        "#[derive(DeriveMigrationName)]",
    )
    .into_iter()
    .collect();
    let (_consts, values, _all_names) = parse_sales_authority();
    assert_eq!(
        migration_tokens.len(),
        values.len(),
        "m0067 CHECK token 数量与 AFTERSALES_ALL 不一致（先于集合比较报数，便于定位）"
    );
    // ALL ⊆ CHECK：词表内任何 token 不在约束里 ⇒ 写入必撞 CHECK 冒 500（65-01 根因）
    for v in &values {
        assert!(
            migration_tokens.contains(v),
            "权威词表 token「{v}」不在 m0067 CHECK 集合内：写入方将违反约束"
        );
    }
    // CHECK ⊆ ALL：约束比词表宽同样是漂移
    for t in &migration_tokens {
        assert!(
            values.contains(t),
            "m0067 CHECK token「{t}」不在权威词表内：约束比词表宽"
        );
    }
}

#[test]
fn constraint_name_stable_and_down_restores_original_five() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/migration/src/domain/production/m0067_aftersales_status_add_accepted_evaluated.rs"
    );
    let src = std::fs::read_to_string(path).expect("读取 m0067 迁移源文件失败");
    // 约束名保持不变：带双引号的字面出现至少 2 次（up/down 各 DROP+ADD 引用同名）
    let quoted_name_hits = src.matches("\"chk_aftersales_status\"").count();
    assert!(
        quoted_name_hits >= 2,
        "m0067 必须以带引号约束名 \"chk_aftersales_status\" 重建（后端引用/错误归因按该名），实际出现 {quoted_name_hits} 次"
    );
    // m0044 原建表约束同名核对：两文件约束名逐字符一致
    let m0044 = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/migration/src/domain/production/m0044_integrate_unreferenced_migrations.rs"
    ))
    .expect("读取 m0044 失败");
    assert!(
        m0044.contains("CONSTRAINT \"chk_aftersales_status\""),
        "m0044 建表约束名已变化——m0067 的 DROP/ADD 名称需同步核对"
    );
    // down 段回滚集合钉死为 m0044 原 5 值（不含 accepted/evaluated）
    let down_pos = src.find("async fn down").expect("m0067 缺少 down");
    let down_slice = &src[down_pos..];
    let anchor = "ADD CONSTRAINT \"chk_aftersales_status\" CHECK (\"status\" IN (";
    let five: BTreeSet<String> = extract_quoted_tokens_from_region(down_slice, anchor, "));")
        .into_iter()
        .collect();
    let expected: BTreeSet<String> = ["opened", "processing", "resolved", "closed", "rejected"]
        .into_iter()
        .map(String::from)
        .collect();
    assert_eq!(
        five, expected,
        "m0067 down 必须逐 token 恢复 m0044 原 5 值集（回滚语义不得扩集/缩水）"
    );
}

/// 区域版 token 提取：从已定位的 down 片段内锚点截取到结束锚点。
fn extract_quoted_tokens_from_region(
    region: &str,
    start_anchor: &str,
    end_anchor: &str,
) -> Vec<String> {
    let start = region
        .find(start_anchor)
        .expect("down 段缺少 ADD CONSTRAINT 锚点");
    let rest = &region[start..];
    let end = rest.find(end_anchor).expect("down 段 CHECK 未闭合");
    let slice = &rest[..end];
    let mut tokens = Vec::new();
    let mut cursor = slice;
    while let Some(open) = cursor.find('\'') {
        cursor = &cursor[open + 1..];
        let close = cursor.find('\'').expect("单引号未闭合");
        tokens.push(cursor[..close].to_string());
        cursor = &cursor[close + 1..];
    }
    tokens
}

// =========================================================
// ④ 类型族三端同源锁（波7 收口批次之二）：
//    写入方白名单（service create）== DB CHECK（m0074 重建的 chk_aftersales_type）
//    == 前端候选（radio / known / AFTER_SALES_TYPE / zh & en i18n 键）
//    缺陷同型：m0044 建表 CHECK 只 4 值，写入方/前端已 5 值（+return_goods），
//    创建"退货"工单撞 CHECK 冒裸 500。修复 = m0074（照 m0067 范式）。
//    本锁防任何一端加/删/改 token 而其余两端未同步再次回归。
// =========================================================

/// 读取仓库内相对路径文件（backend CARGO_MANIFEST_DIR 起点，照
/// color_card_status_vocabulary_test.rs::read_rel 先例，用于覆盖前端文件）。
fn read_repo_file(rel: &str) -> String {
    let path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {path} 失败: {e}"))
}

/// 从服务层提取 create 类型白名单：`if ![" ... "] .contains(&dto.issue_type...)`。
fn parse_service_type_whitelist() -> BTreeSet<String> {
    let src = read_repo_file("/src/services/custom_order_aftersales_service.rs");
    let start = src
        .find("if ![\"")
        .expect("服务层缺少 create 类型白名单数组（if ![\"...\"] 字面量）");
    let block = &src[start..];
    let arr_end = block.find(']').expect("类型白名单数组未闭合");
    let slice = &block[..arr_end];
    let mut tokens = BTreeSet::new();
    let mut cursor = slice;
    while let Some(open) = cursor.find('"') {
        cursor = &cursor[open + 1..];
        let close = cursor.find('"').expect("双引号未闭合");
        let t = &cursor[..close];
        if !t.is_empty() && t.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
            tokens.insert(t.to_string());
        }
        cursor = &cursor[close + 1..];
    }
    tokens
}

/// m0074 CHECK 取值集 == 服务层写入方白名单（双向逐 token）。
#[test]
fn m0074_check_tokens_equals_service_whitelist_bidirectional() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/migration/src/domain/production/m0074_aftersales_type_add_missing_values.rs"
    );
    let check_tokens: BTreeSet<String> = extract_quoted_tokens(
        path,
        "const ALLOWED_TYPE_VALUES",
        "#[derive(DeriveMigrationName)]",
    )
    .into_iter()
    .collect();
    let service_tokens = parse_service_type_whitelist();
    assert_eq!(
        check_tokens.len(),
        service_tokens.len(),
        "m0074 CHECK token 数量与写入方白名单不一致（先于集合比较报数，便于定位）"
    );
    // 白名单 ⊆ CHECK：写入方允许的取值不在约束里 ⇒ 必撞 CHECK 冒 500（本缺陷根因）
    for v in &service_tokens {
        assert!(
            check_tokens.contains(v),
            "写入方允许的类型「{v}」不在 m0074 CHECK 集合内：创建工单将违反约束冒裸 500"
        );
    }
    // CHECK ⊆ 白名单：约束比写入方宽同样是漂移
    for t in &check_tokens {
        assert!(
            service_tokens.contains(t),
            "m0074 CHECK token「{t}」不在写入方白名单内：约束比写入方宽"
        );
    }
    // up 的 NOTICE 逐值计数 ARRAY 与 const 白名单必须同集合（文件内两处不许漂移）
    let notice_tokens: BTreeSet<String> = extract_quoted_tokens(path, "FROM unnest(ARRAY[", "]) v")
        .into_iter()
        .collect();
    assert_eq!(
        notice_tokens, check_tokens,
        "m0074 up NOTICE 的取值 ARRAY 与 ALLOWED_TYPE_VALUES 不一致（文件内自漂移）"
    );
}

/// m0074 约束名与 m0044 逐字符一致且不变；down 回滚集合钉死 m0044 原 4 值。
#[test]
fn m0074_constraint_name_stable_and_down_restores_original_four() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/migration/src/domain/production/m0074_aftersales_type_add_missing_values.rs"
    );
    let src = std::fs::read_to_string(path).expect("读取 m0074 迁移源文件失败");
    // 约束名保持不变：带双引号的字面出现至少 2 次（up/down 各 DROP+ADD 引用同名）
    let quoted_name_hits = src.matches("\"chk_aftersales_type\"").count();
    assert!(
        quoted_name_hits >= 4,
        "m0074 必须以带引号约束名 \"chk_aftersales_type\" 重建（up/down 各 DROP+ADD 共 4 处引用），实际出现 {quoted_name_hits} 次"
    );
    // m0044 原建表约束同名核对：两文件约束名逐字符一致
    let m0044 = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/migration/src/domain/production/m0044_integrate_unreferenced_migrations.rs"
    ))
    .expect("读取 m0044 失败");
    assert!(
        m0044.contains("CONSTRAINT \"chk_aftersales_type\""),
        "m0044 建表约束名已变化——m0074 的 DROP/ADD 名称需同步核对"
    );
    // down 段回滚集合钉死为 m0044 原 4 值（不含 return_goods）——回滚语义不许扩集
    let down_pos = src.find("async fn down").expect("m0074 缺少 down");
    let down_slice = &src[down_pos..];
    let anchor = "ADD CONSTRAINT \"chk_aftersales_type\" CHECK (\"issue_type\" IN (";
    let four: BTreeSet<String> = extract_quoted_tokens_from_region(down_slice, anchor, "));")
        .into_iter()
        .collect();
    let expected: BTreeSet<String> = ["complaint", "repair", "exchange", "refund"]
        .into_iter()
        .map(String::from)
        .collect();
    assert_eq!(
        four, expected,
        "m0074 down 必须逐 token 恢复 m0044 原 4 值集（回滚语义不得扩集/缩水）"
    );
    // down 必须 fail-visible 探测 return_goods 在途行（拒绝回滚吞单据/静默洗数据）
    assert!(
        down_slice.contains("WHERE \"issue_type\" = 'return_goods'")
            && down_slice.contains("RAISE EXCEPTION"),
        "m0074 down 缺少 return_goods 存量行 fail-visible 探测"
    );
}

/// 前端候选四处（创建表单 radio / getIssueTypeLabel known / AFTER_SALES_TYPE /
/// zh-CN 与 en-US issueType 键）均与写入方白名单逐 token 相等（双向）。
#[test]
fn frontend_type_candidates_equal_service_whitelist() {
    let service_tokens = parse_service_type_whitelist();

    // 1) AfterSalesPanel.vue：创建表单 el-radio-group 的 label 候选
    let panel = read_repo_file("../frontend/src/components/AfterSalesPanel.vue");
    let rg_start = panel
        .find("<el-radio-group v-model=\"form.issue_type\">")
        .expect("AfterSalesPanel.vue 缺少类型 radio-group");
    let rg_block = &panel[rg_start..];
    let rg_end = rg_block
        .find("</el-radio-group>")
        .expect("radio-group 未闭合");
    let rg_block = &rg_block[..rg_end];
    let mut radio_tokens: BTreeSet<String> = BTreeSet::new();
    let mut cursor = rg_block;
    let needle = "label=\"";
    while let Some(rel) = cursor.find(needle) {
        let rest = &cursor[rel + needle.len()..];
        let close = rest.find('"').expect("radio label 引号未闭合");
        radio_tokens.insert(rest[..close].to_string());
        cursor = &rest[close..];
    }

    // 2) AfterSalesPanel.vue：getIssueTypeLabel known 数组（单引号 token）
    let label_pos = panel
        .find("售后类型标签映射")
        .expect("AfterSalesPanel.vue 缺少 getIssueTypeLabel 注释锚点");
    let label_block = &panel[label_pos..];
    let known_start = label_block
        .find("const known = [")
        .expect("getIssueTypeLabel 缺少 known 数组");
    let known_block = &label_block[known_start..];
    let known_end = known_block.find(']').expect("known 数组未闭合");
    let mut known_tokens: BTreeSet<String> = BTreeSet::new();
    let mut cursor = &known_block[..known_end];
    while let Some(open) = cursor.find('\'') {
        cursor = &cursor[open + 1..];
        let close = cursor.find('\'').expect("known 单引号未闭合");
        known_tokens.insert(cursor[..close].to_string());
        cursor = &cursor[close + 1..];
    }

    // 3) api/custom-order.ts：AFTER_SALES_TYPE 键集（对象键在行首，8 空格缩进）
    let api = read_repo_file("../frontend/src/api/custom-order.ts");
    let at_start = api
        .find("export const AFTER_SALES_TYPE")
        .expect("custom-order.ts 缺少 AFTER_SALES_TYPE");
    let at_block = &api[at_start..];
    let at_end = at_block.find("};").expect("AFTER_SALES_TYPE 未闭合");
    let at_keys = parse_ts_object_keys(&at_block[..at_end]);

    // 4) locales：zh-CN / en-US common.afterSales.issueType 块键集
    let zh = read_repo_file("../frontend/src/locales/zh-CN.ts");
    let en = read_repo_file("../frontend/src/locales/en-US.ts");
    let zh_keys = parse_issue_type_i18n_keys(&zh);
    let en_keys = parse_issue_type_i18n_keys(&en);

    for (name, got) in [
        ("创建表单 radio 候选", radio_tokens),
        ("getIssueTypeLabel known", known_tokens),
        ("AFTER_SALES_TYPE 键集", at_keys),
        ("zh-CN issueType 键集", zh_keys),
        ("en-US issueType 键集", en_keys),
    ] {
        assert_eq!(
            got, service_tokens,
            "前端「{name}」与写入方白名单漂移（一边改一边忘；写入方集合={service_tokens:?}）"
        );
    }
}

/// 提取 `key: 'value',` 形态的 TS 对象块键名（键名行首缩进、不含引号）。
fn parse_ts_object_keys(block: &str) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for line in block.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_suffix(',') {
            if let Some((key, val)) = rest.split_once(':') {
                let key = key.trim();
                let val = val.trim();
                if !key.is_empty()
                    && key.chars().all(|c| c.is_ascii_lowercase() || c == '_')
                    && val.starts_with('\'')
                {
                    keys.insert(key.to_string());
                }
            }
        }
    }
    keys
}

/// 定位 locales 文件 `afterSales: { ... issueType: { ... } }` 售后块内的
/// issueType 键集。锚定顺序 afterSales → issueType 保证唯一
/// （quality_issues 块也有 issueType 键，直接全局 find 会锚错）。
fn parse_issue_type_i18n_keys(src: &str) -> BTreeSet<String> {
    let as_start = src
        .find("afterSales: {")
        .expect("locales 缺少 afterSales 块");
    let block = &src[as_start..];
    let start = block
        .find("issueType: {")
        .expect("afterSales 块缺少 issueType 子块");
    let block = &block[start..];
    let end = block.find("},").expect("issueType 块未闭合");
    parse_ts_object_keys(&block[..end])
}
