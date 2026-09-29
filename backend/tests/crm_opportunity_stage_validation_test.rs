//! CRM 商机建单阶段校验（`ensure_valid_opportunity_stage`）回归守卫。
//!
//! 钉住的根因（commit 6a0cfa4b）：`services/crm/opp.rs::create_opportunity` 在写入前，
//! 对传入 / 缺省的 `opportunity_stage` 调用 `ensure_valid_opportunity_stage`，
//! 非权威词表取值应被拒为 `AppError::validation`（HTTP 400，服务层错误文案含非法值与合法取值列表），
//! 合法 `QUALIFICATION` 放行。修复前：非法取值直穿到 DB，被 `chk_crm_opportunity_stage`
//! 拒绝后经 `From<DbErr>` 归为 500 DATABASE_ERROR——用户看不到可读的 400 与合法列表。
//!
//! 为什么用「确定性纯判定 + 源码结构守卫」而非活库 e2e：
//! - `ensure_valid_opportunity_stage` 是模块私有 `fn`，集成测试（外部 crate）无法直调；
//! - `create_opportunity` 是 `async` 且首步即 `customer::Entity::find_by_id(...).one(&*self.db)`，
//!   要走到阶段校验必须先存在 customer 行，且常规 CI 集成测试连的是**空 sqlite 库（无表）**
//!   （见 `po_status_column_drift_test.rs` 同源取舍），`#[ignore]` 活库测在此环境下是永不执行的死账
//!   （反面教材：`quotation_e2e_test.rs` 的空占位 ignore 用例）；
//! 故本文件用两层**每次 CI 必跑、零依赖、可真实执行**的守卫锁死根因，二者组合覆盖「非法值→拒绝、
//! 合法值→放行」的判定与「建单确实执行该校验」的接线，任一退化即红，不会假绿：
//!   (A) 成员判定：直接对权威 `models::status::crm_opportunity::ALL_STAGES`（即校验函数 `contains`
//!       所依据的同一份事实来源）复现校验函数的接受/拒绝决策，并钉死大小写精确匹配；
//!   (B) 源码结构守卫：扫 `services/crm/opp.rs` 文本，钉 `create_opportunity` 调用校验函数、默认落
//!       合法 `QUALIFICATION`，以及校验函数体以 `ALL_STAGES` 门控并返回 `AppError::validation`
//!       且文案含「合法取值为」+ 以 `/` 连接合法列表。
//!
//! 覆盖不到的残余风险（交回编排方，不谎称全绿）：
//! create_opportunity 端真实 HTTP「非法→400 / 合法→201」的端到端断言未在此文件覆盖，因其需已迁移
//! PG + customer 前置行；该链路的行为目前由本文件的接线守卫 + e2e `flow/22-crm-full.spec.ts` 的
//! stage-change 用例（阶段词表大小写已对齐权威值）共同兜底。

use std::fs;
use std::path::PathBuf;

use bingxi_backend::models::status::crm_opportunity::ALL_STAGES;

fn read_opp_rs() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/services/crm/opp.rs");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {:?} 失败: {}", path, e))
}

/// 截取从 `needle`（签名起点）到 `stop`（下一个同级签名/块边界）之间的源码片段。
/// 找不到 needle 或 stop 位置非法即 panic（守卫源码结构一旦变形应显式失败，而非静默通过）。
fn slice_between(text: &str, needle: &str, stop: &str) -> String {
    let start = text
        .find(needle)
        .unwrap_or_else(|| panic!("源码未找到片段起点 {:?}（被守卫的结构已漂移）", needle));
    let rest = &text[start..];
    let end = rest[needle.len()..]
        .find(stop)
        .map(|i| needle.len() + i)
        .unwrap_or_else(|| panic!("源码未找到 {:?} 之后的边界 {:?}", needle, stop));
    rest[..end].to_string()
}

// ===== (A) 成员判定：非法值被拒、合法值放行、大小写精确（复现 ensure_valid_opportunity_stage 的决策） =====

/// 校验函数对 stage 的接受条件 = `ALL_STAGES.contains(&stage)`；此处对该判定逐项钉死。
/// 权威列表：QUALIFICATION / NEEDS_ANALYSIS / PROPOSAL / NEGOTIATION / CLOSED_WON / CLOSED_LOST。
fn stage_is_valid(stage: &str) -> bool {
    ALL_STAGES.contains(&stage)
}

#[test]
fn 非法商机阶段建单应被校验拒绝() {
    // 任务点名的非法值 + 边界/畸形值：均不属于权威词表，校验函数会返回 AppError::validation。
    for bad in [
        "INVALID",        // 完全不在词表
        "won",            // 小写：与权威 CLOSED_WON 逐字符不同（大小写漂移即非法）
        "qualification",  // 小写：与权威 QUALIFICATION 逐字符不同
        "proposal",       // 小写：e2e 历史漂移值，非法
        "qualifying",     // 非词表 token：e2e 历史漂移值，非法
        "",               // 空串
        "QUALIFICATION ", // 尾随空格
        " Proposal",      // 前导空格
        "Proposal",       // 混合大小写
        "CLOSED",         // 半截终态名（词表只有 CLOSED_WON/CLOSED_LOST）
    ] {
        assert!(
            !stage_is_valid(bad),
            "非法商机阶段 {:?} 必须被 ensure_valid_opportunity_stage 拒绝（不在 ALL_STAGES），\
             否则修复 commit 6a0cfa4b 的校验被绕过",
            bad
        );
    }
}

#[test]
fn 合法商机阶段建单应通过校验() {
    // 任务点名的合法值 QUALIFICATION（缺省值）必须放行。
    assert!(
        stage_is_valid("QUALIFICATION"),
        "合法阶段 QUALIFICATION 必须通过校验（缺省建单即此值）"
    );
    // 权威词表全部六值均须放行，且逐字符大写（守卫词表本身不被误改小写）。
    for legal in ALL_STAGES {
        assert!(stage_is_valid(legal), "权威词表值 {:?} 必须通过校验", legal);
        assert!(
            legal.chars().all(|c| c.is_uppercase() || c == '_'),
            "商机阶段权威值应全大写（发现 {:?}）——大小写规则是校验精确匹配的前提",
            legal
        );
    }
}

#[test]
fn 阶段校验大小写精确_权威大写存在而小写变体不存在() {
    // 钉死「小写漂移」根因：同一 token 的小写/混合大小写变体一律非法。
    for canon in ["QUALIFICATION", "NEEDS_ANALYSIS", "PROPOSAL", "NEGOTIATION"] {
        assert!(stage_is_valid(canon), "权威大写 {:?} 必须合法", canon);
        let lower = canon.to_lowercase();
        let mixed = canon
            .chars()
            .enumerate()
            .map(|(i, c)| {
                if i == 0 {
                    c.to_ascii_uppercase()
                } else {
                    c.to_ascii_lowercase()
                }
            })
            .collect::<String>();
        assert!(
            !stage_is_valid(&lower),
            "{:?} 的小写变体 {:?} 必须非法（大小写精确匹配，防词表被小写化）",
            canon,
            lower
        );
        assert!(
            !stage_is_valid(&mixed),
            "{:?} 的首字母大写变体 {:?} 必须非法",
            canon,
            mixed
        );
    }
}

// ===== (B) 源码结构守卫：钉死 create_opportunity 的接线与校验函数的实现契约 =====

#[test]
fn create_opportunity_在建单前调用阶段校验() {
    let src = read_opp_rs();
    // create_opportunity 方法体：从签名到下一个同级方法（list_opportunities）。
    let body = slice_between(
        &src,
        "pub async fn create_opportunity",
        "\n    pub async fn list_opportunities",
    );

    // (1) 接线：建单确实调用了校验函数——移除该调用（回归到直穿 DB 报 500）即红。
    assert!(
        body.contains("ensure_valid_opportunity_stage("),
        "create_opportunity 必须调用 ensure_valid_opportunity_stage（否则 commit 6a0cfa4b 的建单校验被移除，\
         非法阶段将重新直穿 DB 触发 500 而非 400）"
    );
    // (2) 校验须发生在写库之前：调用点先于 insert。
    let call_pos = body.find("ensure_valid_opportunity_stage(").unwrap();
    let insert_pos = body
        .find(".insert(")
        .expect("create_opportunity 应包含写库 insert");
    assert!(
        call_pos < insert_pos,
        "阶段校验必须在 insert 之前执行（否则非法值先落库再报错，起不到 400 前置拦截作用）"
    );
    // (3) 缺省阶段（用户未传 opportunity_stage）落合法 QUALIFICATION，而非任何非法值。
    assert!(
        body.contains("opp_status::QUALIFICATION"),
        "create_opportunity 缺省阶段必须是权威合法值 QUALIFICATION（缺省值本身不得是非法词）"
    );
}

#[test]
fn 阶段校验函数以权威词表门控并返回validation错误含合法列表() {
    let src = read_opp_rs();
    // 校验函数体：从定义到 impl CrmService 之前。
    let f = slice_between(&src, "fn ensure_valid_opportunity_stage", "impl CrmService");

    // 门控来源 = 权威 ALL_STAGES（与 (A) 成员判定同一事实来源，闭环）。
    assert!(
        f.contains("ALL_STAGES.contains("),
        "ensure_valid_opportunity_stage 必须以权威 ALL_STAGES 作为合法性判据"
    );
    // 非法值返回构造器 = AppError::validation（对应 HTTP 400，非 500/其它）。
    assert!(
        f.contains("AppError::validation"),
        "ensure_valid_opportunity_stage 非法值须返回 AppError::validation（400），\
         不得退化为 internal/database/business 等其它变体"
    );
    // 服务层错误文案须携带「合法取值为」并列出合法值（用 join 拼接 ALL_STAGES）——
    // 即任务要求的「消息含合法列表」（注：该文案在服务层错误对象/日志中；HTTP 出参 message 经
    // public_message 脱敏为固定常量，属另一契约，不在此函数职责内）。
    assert!(
        f.contains("合法取值为"),
        "校验错误文案必须携带合法取值列表提示（「合法取值为」）"
    );
    assert!(
        f.contains("ALL_STAGES.join(\"/\")"),
        "校验错误文案必须以 `/` 分隔 join 全部合法取值（ALL_STAGES.join(\"/\")）"
    );
}
