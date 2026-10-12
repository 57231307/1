//! 列表分页页码 1-based 语义锁（商检 / 产地证 / 存货跌价准备三站点）。
//!
//! 锁定契约（权威范式 = `src/utils/pagination.rs::paginate_with_total`，其内部已完成
//! 1-based 页码 → 0-based 偏移转换，任何调用方不得再手写减 1 或误用缺省 0）：
//! - `services/export_inspection_service.rs::list`（GET /export-inspections，按 created_at 倒序）
//! - `services/certificate_of_origin_service.rs::list`（产地证列表，按 issue_date 倒序）
//! - `services/inventory_write_down_service.rs::list`（GET /inventory/write-downs，按 period 倒序）
//!
//! 三判据（每站点逐一钉死）：
//!   ① 第 1 页（显式 page=1 与缺省不传页码两种形态）必须返回排序后的首行；
//!   ② total 为过滤命中的总行数，且各页回显的 total 一致；
//!   ③ 第 2 页不得重复第 1 页首行（page_size=1 时第 2 页恰为排序次行）。
//! 非法参数 fail-visible（本仓口径：越界回 400 点名允许值，不静默夹紧）：
//! page=0、page_size=0、page_size>100 ⇒ 400 + code=VALIDATION_ERROR + 可外显原因。
//!
//! 覆盖策略选型说明（真库行为锁而非纯静态源码锁的原因）：本族缺陷是"第一页返回
//! 第二页"的**语义级**越位，静态扫描只能钉公式形状，钉不住"页码→偏移"换算结果；
//! 只有真库按行回读能区分"首行被跳过"与"首行为空"。静态扫描仅作防回潮辅助锁
//! （见本文件末尾 `source_scan_*`），两者互补而非互替。
//!
//! 夹具：`test_common::setup_test_db()` 连 TEST_DATABASE_URL 并清空业务表；
//! 集成测试在 nextest 下整组串行（.config/nextest.toml db-integration 组），
//! 因此 total 的精确值断言不受并发用例污染。

mod test_common;

use bingxi_backend::models::{certificate_of_origin, export_inspection, inventory_write_down};
use bingxi_backend::services::certificate_of_origin_service::{
    CertificateOfOriginService, ListParams as ColListParams,
};
use bingxi_backend::services::export_inspection_service::{
    ExportInspectionService, ListParams as EiListParams,
};
use bingxi_backend::services::inventory_write_down_service::{
    InventoryWriteDownService, ListParams as IwdListParams,
};
use bingxi_backend::utils::error::AppError;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection};
use std::sync::Arc;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("测试夹具日期常量必须合法")
}

/// 播种两行出口商检：created_at 相隔 1 小时（倒序下"新行"必为排序首行）。
/// 返回 (排序首行 id, 排序次行 id)。
async fn seed_two_inspections(db: &DatabaseConnection) -> (i32, i32) {
    let now = Utc::now();
    let newer = export_inspection::ActiveModel {
        inspection_no: Set("EI-W11-NEWER".to_string()),
        sales_order_id: Set(1),
        product_name: Set("w11-newer-fabric".to_string()),
        hs_code: Set("5407.10".to_string()),
        inspection_type: Set("TYPE-A".to_string()),
        inspection_agency: Set("CIQ".to_string()),
        inspection_date: Set(date(2026, 3, 1)),
        result: Set("pending".to_string()),
        created_by: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("播种新商检行必须成功");
    let older = export_inspection::ActiveModel {
        inspection_no: Set("EI-W11-OLDER".to_string()),
        sales_order_id: Set(1),
        product_name: Set("w11-older-fabric".to_string()),
        hs_code: Set("5407.10".to_string()),
        inspection_type: Set("TYPE-A".to_string()),
        inspection_agency: Set("CIQ".to_string()),
        inspection_date: Set(date(2026, 2, 1)),
        result: Set("pending".to_string()),
        created_by: Set(1),
        created_at: Set(now - chrono::Duration::hours(1)),
        updated_at: Set(now - chrono::Duration::hours(1)),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("播种旧商检行必须成功");
    (newer.id, older.id)
}

/// 播种两行产地证（按 issue_date 倒序，签发日期新的为排序首行）。
async fn seed_two_certificates(db: &DatabaseConnection) -> (i32, i32) {
    let now = Utc::now();
    let newer = certificate_of_origin::ActiveModel {
        certificate_no: Set("CO-W11-NEWER".to_string()),
        product_name: Set("w11-newer-goods".to_string()),
        hs_code: Set("6302.21".to_string()),
        origin_country: Set("China".to_string()),
        destination_country: Set("Germany".to_string()),
        quantity: Set(Decimal::from(100_u8)),
        unit: Set("M".to_string()),
        certificate_type: Set("FORM-A".to_string()),
        issue_date: Set(date(2026, 3, 1)),
        status: Set("active".to_string()),
        created_by: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("播种新产地证行必须成功");
    let older = certificate_of_origin::ActiveModel {
        certificate_no: Set("CO-W11-OLDER".to_string()),
        product_name: Set("w11-older-goods".to_string()),
        hs_code: Set("6302.21".to_string()),
        origin_country: Set("China".to_string()),
        destination_country: Set("France".to_string()),
        quantity: Set(Decimal::from(50_u8)),
        unit: Set("M".to_string()),
        certificate_type: Set("FORM-A".to_string()),
        issue_date: Set(date(2026, 1, 1)),
        status: Set("active".to_string()),
        created_by: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("播种旧产地证行必须成功");
    (newer.id, older.id)
}

/// 播种两行跌价准备（按 period 倒序，计提期间新的为排序首行）。
async fn seed_two_write_downs(db: &DatabaseConnection) -> (i32, i32) {
    let now = Utc::now();
    let newer = inventory_write_down::ActiveModel {
        product_id: Set(1),
        write_down_type: Set("slow_moving".to_string()),
        original_cost: Set(Decimal::from(100_u8)),
        net_realizable_value: Set(Decimal::from(80_u8)),
        write_down_amount: Set(Decimal::from(20_u8)),
        period: Set(date(2026, 3, 31)),
        status: Set("draft".to_string()),
        created_by: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("播种新期间跌价行必须成功");
    let older = inventory_write_down::ActiveModel {
        product_id: Set(1),
        write_down_type: Set("slow_moving".to_string()),
        original_cost: Set(Decimal::from(90_u8)),
        net_realizable_value: Set(Decimal::from(70_u8)),
        write_down_amount: Set(Decimal::from(20_u8)),
        period: Set(date(2026, 1, 31)),
        status: Set("draft".to_string()),
        created_by: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("播种旧期间跌价行必须成功");
    (newer.id, older.id)
}

/// 三判据的统一校验入参：同一次取数的第 1 页/第 2 页/缺省页三组回读结果。
struct PageProbe {
    items_p1: Vec<i32>,
    total_p1: u64,
    items_p2: Vec<i32>,
    total_p2: u64,
    items_default: Vec<i32>,
    newer_id: i32,
    older_id: i32,
}

/// 三判据的统一校验体：①page=1 返回首行 ②total 一致且为 2 ③page=2 不重复首行，
/// 外加"缺省页码 = 第 1 页"。三个站点共用同一判据，防各自写法漂移。
fn assert_one_based_three_criteria(site: &str, probe: PageProbe) {
    let PageProbe {
        items_p1,
        total_p1,
        items_p2,
        total_p2,
        items_default,
        newer_id,
        older_id,
    } = probe;

    assert_eq!(total_p1, 2, "{site}: total 应为过滤命中的总行数 2");
    assert_eq!(total_p2, total_p1, "{site}: 各页回显的 total 必须一致");

    assert_eq!(
        items_p1.len(),
        1,
        "{site}: page_size=1 时第 1 页应恰有 1 行"
    );
    assert_eq!(
        items_p1[0], newer_id,
        "{site}: 第 1 页必须返回排序首行（页码 1-based，缺省/显式 1 都取第一页）"
    );

    assert_eq!(
        items_p2.len(),
        1,
        "{site}: page_size=1 时第 2 页应恰有 1 行"
    );
    assert_eq!(
        items_p2[0], older_id,
        "{site}: 第 2 页必须是排序次行，不得重复第 1 页首行"
    );

    assert_eq!(
        items_default.first().copied(),
        Some(newer_id),
        "{site}: 不传页码时缺省必须是第 1 页（返回排序首行）"
    );
}

// =========================================================
// 真库行为锁（三站点各一条）
// =========================================================

/// 出口商检列表：1-based 页码三判据。
#[tokio::test]
async fn export_inspection_list_pagination_is_one_based() {
    let db = test_common::setup_test_db().await;
    let (newer_id, older_id) = seed_two_inspections(&db).await;
    let service = ExportInspectionService::new(Arc::new(db));

    let params = |page: Option<u64>| EiListParams {
        sales_order_id: None,
        inspection_no: None,
        result: None,
        page,
        page_size: Some(1),
    };
    let (items_p1, total_p1) = service
        .list(params(Some(1)))
        .await
        .expect("第 1 页查询必须成功");
    let (items_p2, total_p2) = service
        .list(params(Some(2)))
        .await
        .expect("第 2 页查询必须成功");
    let (items_default, _t) = service
        .list(params(None))
        .await
        .expect("缺省页码查询必须成功");

    assert_one_based_three_criteria(
        "export_inspection",
        PageProbe {
            items_p1: items_p1.into_iter().map(|m| m.id).collect(),
            total_p1,
            items_p2: items_p2.into_iter().map(|m| m.id).collect(),
            total_p2,
            items_default: items_default.into_iter().map(|m| m.id).collect(),
            newer_id,
            older_id,
        },
    );
}

/// 产地证列表：1-based 页码三判据。
#[tokio::test]
async fn certificate_of_origin_list_pagination_is_one_based() {
    let db = test_common::setup_test_db().await;
    let (newer_id, older_id) = seed_two_certificates(&db).await;
    let service = CertificateOfOriginService::new(Arc::new(db));

    let params = |page: Option<u64>| ColListParams {
        inspection_id: None,
        status: None,
        page,
        page_size: Some(1),
    };
    let (items_p1, total_p1) = service
        .list(params(Some(1)))
        .await
        .expect("第 1 页查询必须成功");
    let (items_p2, total_p2) = service
        .list(params(Some(2)))
        .await
        .expect("第 2 页查询必须成功");
    let (items_default, _t) = service
        .list(params(None))
        .await
        .expect("缺省页码查询必须成功");

    assert_one_based_three_criteria(
        "certificate_of_origin",
        PageProbe {
            items_p1: items_p1.into_iter().map(|m| m.id).collect(),
            total_p1,
            items_p2: items_p2.into_iter().map(|m| m.id).collect(),
            total_p2,
            items_default: items_default.into_iter().map(|m| m.id).collect(),
            newer_id,
            older_id,
        },
    );
}

/// 存货跌价准备列表：1-based 页码三判据。
#[tokio::test]
async fn inventory_write_down_list_pagination_is_one_based() {
    let db = test_common::setup_test_db().await;
    let (newer_id, older_id) = seed_two_write_downs(&db).await;
    let service = InventoryWriteDownService::new(Arc::new(db));

    let params = |page: Option<u64>| IwdListParams {
        product_id: None,
        write_down_type: None,
        page,
        page_size: Some(1),
    };
    let (items_p1, total_p1) = service
        .list(params(Some(1)))
        .await
        .expect("第 1 页查询必须成功");
    let (items_p2, total_p2) = service
        .list(params(Some(2)))
        .await
        .expect("第 2 页查询必须成功");
    let (items_default, _t) = service
        .list(params(None))
        .await
        .expect("缺省页码查询必须成功");

    assert_one_based_three_criteria(
        "inventory_write_down",
        PageProbe {
            items_p1: items_p1.into_iter().map(|m| m.id).collect(),
            total_p1,
            items_p2: items_p2.into_iter().map(|m| m.id).collect(),
            total_p2,
            items_default: items_default.into_iter().map(|m| m.id).collect(),
            newer_id,
            older_id,
        },
    );
}

// =========================================================
// 非法分页参数 fail-visible 锁（400 点名允许值，不静默夹紧）
// =========================================================

/// 越界分页参数必须落 400 VALIDATION_ERROR 且走可外显通道（操作员能看到是哪个参数）。
fn assert_rejected(site: &str, err: AppError) {
    assert_eq!(
        err.error_code(),
        "VALIDATION_ERROR",
        "{site}: 非法分页参数必须归 VALIDATION_ERROR 族"
    );
    assert!(
        matches!(err, AppError::ValidationErrorDisplayable(_)),
        "{site}: 拒绝原因必须可外显（用户自己提交的查询参数，安全边界允许），实际: {err:?}"
    );
}

#[tokio::test]
async fn export_inspection_list_rejects_illegal_page_params() {
    let db = test_common::setup_test_db().await;
    let service = ExportInspectionService::new(Arc::new(db));
    let params = |page: Option<u64>, page_size: Option<u64>| EiListParams {
        sales_order_id: None,
        inspection_no: None,
        result: None,
        page,
        page_size,
    };
    let e = service
        .list(params(Some(0), Some(10)))
        .await
        .expect_err("page=0 必须 400，不得静默当第 1 页");
    assert_rejected("export_inspection page=0", e);
    let e = service
        .list(params(Some(1), Some(0)))
        .await
        .expect_err("page_size=0 必须 400，不得静默夹紧或空转全表");
    assert_rejected("export_inspection page_size=0", e);
    let e = service
        .list(params(Some(1), Some(101)))
        .await
        .expect_err("page_size>100 必须 400，不得静默夹紧到 100");
    assert_rejected("export_inspection page_size=101", e);
}

#[tokio::test]
async fn certificate_of_origin_list_rejects_illegal_page_params() {
    let db = test_common::setup_test_db().await;
    let service = CertificateOfOriginService::new(Arc::new(db));
    let params = |page: Option<u64>, page_size: Option<u64>| ColListParams {
        inspection_id: None,
        status: None,
        page,
        page_size,
    };
    let e = service
        .list(params(Some(0), Some(10)))
        .await
        .expect_err("page=0 必须 400");
    assert_rejected("certificate_of_origin page=0", e);
    let e = service
        .list(params(Some(1), Some(0)))
        .await
        .expect_err("page_size=0 必须 400");
    assert_rejected("certificate_of_origin page_size=0", e);
    let e = service
        .list(params(Some(1), Some(101)))
        .await
        .expect_err("page_size>100 必须 400");
    assert_rejected("certificate_of_origin page_size=101", e);
}

#[tokio::test]
async fn inventory_write_down_list_rejects_illegal_page_params() {
    let db = test_common::setup_test_db().await;
    let service = InventoryWriteDownService::new(Arc::new(db));
    let params = |page: Option<u64>, page_size: Option<u64>| IwdListParams {
        product_id: None,
        write_down_type: None,
        page,
        page_size,
    };
    let e = service
        .list(params(Some(0), Some(10)))
        .await
        .expect_err("page=0 必须 400");
    assert_rejected("inventory_write_down page=0", e);
    let e = service
        .list(params(Some(1), Some(0)))
        .await
        .expect_err("page_size=0 必须 400");
    assert_rejected("inventory_write_down page_size=0", e);
    let e = service
        .list(params(Some(1), Some(101)))
        .await
        .expect_err("page_size>100 必须 400");
    assert_rejected("inventory_write_down page_size=101", e);
}

// =========================================================
// 防回潮辅助静态锁（真库行为锁的补充，非替代）
// =========================================================

/// 三站点必须继续走权威 helper `paginate_with_total`，且分页参数缺省不得再出现
/// `unwrap_or(0)`（0-based 缺省即"第一页跳过首行"的本族根因形态）。
#[test]
fn source_scan_pagination_sites_converge_on_authoritative_helper() {
    let sites = [
        (
            "export_inspection_service.rs",
            include_str!("../src/services/export_inspection_service.rs"),
        ),
        (
            "certificate_of_origin_service.rs",
            include_str!("../src/services/certificate_of_origin_service.rs"),
        ),
        (
            "inventory_write_down_service.rs",
            include_str!("../src/services/inventory_write_down_service.rs"),
        ),
    ];
    for (name, src) in sites {
        let norm = src.replace('\r', "");
        assert!(
            norm.contains("use crate::utils::pagination::paginate_with_total;")
                && norm.contains("paginate_with_total(paginator, page)"),
            "{name}: 分页必须收口到 utils::pagination::paginate_with_total，禁止回退到手写 fetch_page 偏移"
        );
        assert!(
            !norm.contains("params.page.unwrap_or(0)"),
            "{name}: 页码缺省不得写 unwrap_or(0)（1-based 页码被当 0-based 偏移的根因形态）"
        );
    }
}
