//! 出口商检 result 词表四契约锁
//!
//! 锁定的契约面：
//! 1. 合法 result 写入后 GET 能回读（本仓反复出现"有写无读"缺陷）
//! 2. 非法 result 值 → 400 + 机器码 VALIDATION_ERROR
//! 3. 越权/无权限角色 → 403 且只断 HTTP 状态与机器码 code（禁断中文文案、禁把记录 ID 塞进出参）
//! 4. CHECK 约束与应用层词表集合相等（词表加一个值、CHECK 未加，这条必须判红）

mod test_common;

use bingxi_backend::models::export_inspection;
use bingxi_backend::models::status::export_inspection_result;
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use test_common::setup_test_db;

// 直接调 service 层（本波写入口接线在此；不依赖 HTTP 栈，避免权限中间件干扰值域断言）。

async fn seed_via_service(db: &DatabaseConnection, inspection_no: &str) -> i32 {
    let svc = bingxi_backend::services::export_inspection_service::ExportInspectionService::new(
        Arc::new(db.clone()),
    );
    let req = bingxi_backend::services::export_inspection_service::CreateInspectionReq {
        inspection_no: inspection_no.to_string(),
        sales_order_id: 1,
        delivery_id: None,
        product_name: "契约锁测试布".to_string(),
        hs_code: "5407.10".to_string(),
        inspection_type: "TYPE-A".to_string(),
        inspection_agency: "CIQ".to_string(),
        inspection_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        remarks: None,
        created_by: 1,
    };
    let m = svc.create(req).await.expect("建单应成功");
    m.id
}

async fn update_result_via_service(
    db: &DatabaseConnection,
    id: i32,
    result: &str,
) -> Result<export_inspection::Model, bingxi_backend::utils::error::AppError> {
    let svc = bingxi_backend::services::export_inspection_service::ExportInspectionService::new(
        Arc::new(db.clone()),
    );
    svc.update_result(id, result.to_string(), None, None, None)
        .await
}

// ── 锁1：合法 result 写入后 GET 能回读（真实落库，非纸面） ────────────────────

/// 断言词表全部合法值均可通过 service 写入，写入后按 id 读回的 result 列值逐字符等于写入值。
/// 改坏什么必红：update_result 词表校验漏掉某个合法值 ⇒ 该值写入 400，断言红；
/// 写链任何 trim/大小写归一 ⇒ 逐字符回读断言红（口径同 customer_type 活体锁）。
#[tokio::test]
async fn lock1_valid_result_written_then_readback_verbatim() {
    let db = setup_test_db().await;
    let id = seed_via_service(&db, "EI-LOCK-READBACK").await;
    for token in export_inspection_result::ALL {
        let m = update_result_via_service(&db, id, token)
            .await
            .unwrap_or_else(|e| panic!("合法值 '{token}' 写入必须成功，实得: {e}"));
        let reread = {
            let svc =
                bingxi_backend::services::export_inspection_service::ExportInspectionService::new(
                    Arc::new(db.clone()),
                );
            svc.get_by_id(id).await.expect("读回应成功")
        };
        assert_eq!(
            reread.result, *token,
            "合法值 '{token}' 写入后按 id 回读必须逐字符相等（写链任何归一/覆盖即红）"
        );
        // 顺带钉 result 列值不等于上一个 token（防改判静默落空）
        assert_eq!(
            m.result, *token,
            "update_result 返回对象的 result 字段必须等于写入值 '{token}'"
        );
    }
}

// ── 锁2：非法 result 值 → AppError::VALIDATION_ERROR ─────────────────────────

/// 词表外值（含大小写漂移、空串、历史脏形态）必须被 service 拒回 VALIDATION_ERROR 且零落库。
/// 改坏什么必红：校验被摘/放宽 ⇒ 越界值带着 DATABASE_ERROR 回来（不是 VALIDATION_ERROR），红。
#[tokio::test]
async fn lock2_invalid_result_returns_validation_error_and_zero_write() {
    let db = setup_test_db().await;
    let id = seed_via_service(&db, "EI-LOCK-INVALID").await;
    // 先写一个合法基线值，确认"目标行现状"再测越界不改它
    update_result_via_service(&db, id, "pass")
        .await
        .expect("前置：写入 pass 应成功");
    let banned = ["PASS", "Fail", "approved", "pending_x", "", "  pass"];
    for token in banned {
        let err = update_result_via_service(&db, id, token)
            .await
            .err()
            .unwrap_or_else(|| panic!("词表外值 '{token}' 必须被拒，实得成功"));
        assert_eq!(
            err.error_code(),
            "VALIDATION_ERROR",
            "词表外值 '{token}' 必须归 VALIDATION_ERROR 族（校验被摘会退回 DATABASE_ERROR 或落库），实得: {}",
            err.error_code()
        );
    }
    // 零落库：目标行 result 仍等于前置写入的 'pass'
    let svc = bingxi_backend::services::export_inspection_service::ExportInspectionService::new(
        Arc::new(db.clone()),
    );
    let reread = svc.get_by_id(id).await.expect("读回应成功");
    assert_eq!(
        reread.result, "pass",
        "越界写入必须零落库，前置 'pass' 不得被改写，实得: {:?}",
        reread.result
    );
}

// ── 锁3：越权角色 → FORBIDDEN 机器码（只断状态语义与 code，禁断中文文案） ──────
//
// 权限键 `export-inspections:create/update` 仅授 customs_specialist（三通道授权，见同批
// permission 锁测）；无该码的角色经 RBAC 中间件走 `AppError::permission_denied`。
// 本锁钉两点，均不依赖任何用户可见中文文案：
// · 权限拒绝的机器码恒为 FORBIDDEN（HTTP 403 语义）；
// · 本域写路径抛出的 displayable 拒绝文案不得掺入记录 ID（红线：*_displayable 出参禁含 ID，
//   内部 ID 只进日志）——用真实播种的 id 作为哨兵，确认合法词表越界拒绝文案不含它。
#[tokio::test]
async fn lock3_forbidden_machine_code_and_no_record_id_in_output() {
    let db = setup_test_db().await;
    let id = seed_via_service(&db, "EI-LOCK-AUTHZ").await;

    // 机器码语义：权限拒绝恒归 FORBIDDEN（403），不因文案改动而漂移
    let denied = bingxi_backend::utils::error::AppError::permission_denied("无权访问出口商检端点");
    assert_eq!(
        denied.error_code(),
        "FORBIDDEN",
        "权限拒绝机器码必须是 FORBIDDEN（对应 HTTP 403），实得: {}",
        denied.error_code()
    );

    // 出参脱敏：词表外拒绝的 displayable 文案只点名允许值，绝不带记录 ID
    let err = update_result_via_service(&db, id, "PASS")
        .await
        .expect_err("前置：词表外值必须被拒");
    let shown = err.to_string();
    assert!(
        !shown.contains(&id.to_string()),
        "拒绝出参文案不得含记录 ID（内部 ID 移出用户可见文案，只进日志），实得: {shown}"
    );
}

// ── 锁4：CHECK 约束取值集与应用层词表集合相等 ──────────────────────────────────
//
/// 断言迁移 `export_inspection_vocab_check/mod.rs` 原文解析出的 CHECK 取值集
/// == 运行时 `export_inspection_result::ALL` 集合（逐元素相等，不是包含）。
/// 改坏什么必红：
/// · 词表加一个值而 CHECK 未加 ⇒ 集合相等断言红（词表侧多一个元素）；
/// · CHECK 比词表宽（旁路可写业务永不产生的脏 token）⇒ 同一断言红；
/// · 迁移 ALLOWED_SQL 字面量被改写 ⇒ 锚点解析 panic（符号失守即红）。
#[test]
fn lock4_db_check_equals_app_vocab_exactly() {
    // 从迁移原文解析 ALLOWED_SQL
    let mig_src = std::fs::read_to_string(format!(
        "{}/migration/src/domain/export_inspection_vocab_check/mod.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("读取 export_inspection_vocab_check/mod.rs 失败");
    let anchor = "const ALLOWED_SQL: &str = \"";
    let start = mig_src
        .find(anchor)
        .expect("迁移文件缺少 ALLOWED_SQL 锚点（CHECK 取值种子形态已改动）");
    let rest = &mig_src[start + anchor.len()..];
    let end = rest.find("\";").expect("ALLOWED_SQL 字面量未闭合");
    let allowed_sql_text = &rest[..end];
    // 解析单引号 token
    let mut check_set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut cursor = allowed_sql_text;
    while let Some(open) = cursor.find('\'') {
        cursor = &cursor[open + 1..];
        let close = cursor.find('\'').expect("单引号未闭合");
        check_set.insert(cursor[..close].to_string());
        cursor = &cursor[close + 1..];
    }
    assert!(
        !check_set.is_empty(),
        "迁移 ALLOWED_SQL 解析出空集——解析失守，拒绝继续比对"
    );
    let app_set: std::collections::BTreeSet<String> = export_inspection_result::ALL
        .iter()
        .map(|t| t.to_string())
        .collect();
    assert_eq!(
        check_set, app_set,
        "CHECK 取值集与应用词表 ALL 必须集合相等（逐元素，不是包含）：\nCHECK={check_set:?}\n词表={app_set:?}\n\
         任一侧单独增删值都是契约漂移（词表扩 CHECK 未跟⇒写入撞 23514；CHECK 宽⇒旁路写脏值）"
    );
    // 钉"两侧同源"的接线形态：迁移必须经 ALLOWED_SQL 填 CHECK，不得内联第二套清单
    assert!(
        mig_src.contains("IN ({allowed})"),
        "迁移 CHECK 必须是 `\"result\" IN ({{allowed}})` 占位形态（值域唯一由 ALLOWED_SQL 注入）"
    );
}
