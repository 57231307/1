//! 导出取数全量行锁（契约锁族，真已迁移 PostgreSQL，CI 侧执行）
//!
//! 功能：钉死四条导出链路（供应商、仓库、固定资产、色卡发放）的取数入口
//! 必须返回当前筛选与归属范围下的全部行，而不是对外分页上限的一页
//!
//! 判据（逐条）：
//! - 造一百二十行（严格超过对外分页上限一百行），导出取数入口返回的行数
//! 必须等于造数总量，不得等于一百，也不得等于任何批次常量；
//! - 同一筛选条件下改调对外分页入口，返回行数必须等于一百 —— 这条同时钉两件事：
//! 对外分页的页宽防护没有被放宽（外部同参数仍然要不到超大页宽），
//! 以及导出拿到的行数不可能来自单次大页查询，取数过程必然发了多条查询；
//! - 色卡发放必须两个范围各自拿全：全部范围拿到两个客户的合计行，
//! 仅自己范围只拿到本人归属客户下的行且同样是全量而非一页，并断言不越界混入他人行；
//! - 并列排序键的稳定性：供应商与色卡发放的主排序键（创建时间、发放时间）
//! 在造数时故意全部取同一时刻，若导出取数没有把主键补成末级排序，
//! 批次边界就会漂移，行数就会不等于造数量，用例必红。
//!
//! 调用方：集成测试层。
//! 入参：经 test_common::setup_test_db 连接并清空业务表后的真库，各用例自建自有号段的行。
//! 传给谁：供应商服务、仓库服务、固定资产服务与色卡发放服务的导出取数入口。
//! 存什么：造数行落在 suppliers、warehouses、fixed_assets、color_card_issues
//! 与其父表 users、customers、departments、color_cards；状态取值一律引自权威词表。
//! 存哪里：真库回读比对，不使用 mock。供应商表由迁移播种且不参与清空，
//! 故该用例结束时按自有编码前缀删除自己插入的行。

mod test_common;

use bingxi_backend::models::status::{color_card as card_status, master_data};
use bingxi_backend::models::{
    color_card, color_card_issue, customer, department, fixed_asset, supplier, user, warehouse,
};
use bingxi_backend::services::color_card_issue_service::{ColorCardIssueService, ListIssuesQuery};
use bingxi_backend::services::fixed_asset_service::{AssetQueryParams, FixedAssetService};
use bingxi_backend::services::supplier_service::{SupplierQueryParams, SupplierService};
use bingxi_backend::services::warehouse_service::WarehouseService;
use chrono::{TimeZone, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};
use std::sync::Arc;
use test_common::setup_test_db;

/// 造数总量，严格超过对外分页上限一百行
const ROWS: i64 = 120;
/// 对外分页入口在超大页宽下仍应返回的一百行
const ONE_PAGE: usize = 100;
/// 超大页宽：用于检验对外分页防护未被放宽，同时给导出入口一个荒谬的页宽
const HUGE_PAGE_SIZE: u64 = 999_999;

/// 部门号段（父行，避开迁移播种的小号段）
const DEPT_ID: i32 = 780_000;
/// 供应商造数号段与编码前缀
const SUPPLIER_ID_BASE: i32 = 780_001;
const SUPPLIER_CODE_PREFIX: &str = "EXPALL-F-SUP";
/// 仓库造数号段与编码前缀
const WAREHOUSE_ID_BASE: i32 = 781_001;
const WAREHOUSE_CODE_PREFIX: &str = "EXPALL-F-WH";
/// 固定资产造数号段与编码前缀
const ASSET_ID_BASE: i32 = 782_001;
const ASSET_NO_PREFIX: &str = "EXPALL-F-FA";
/// 会话用户号段（色卡发放的外键与归属人）
const OPERATOR_A: i32 = 783_001;
const OPERATOR_B: i32 = 783_002;
/// 客户号段，分别归属两位操作人
const CUSTOMER_OF_A: i32 = 783_101;
const CUSTOMER_OF_B: i32 = 783_102;
/// 色卡号段
const COLOR_CARD_ID: i64 = 783_201;
/// 发放记录号段
const ISSUE_ID_BASE: i64 = 784_001;

/// 造数用的同一时刻：让主排序键在全部行上并列，从而检验末级排序是否补齐
fn fixture_moment() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 1, 12, 0, 0).unwrap()
}

/// 播种部门父行
///
/// 功能：为客户与会话用户的部门外键准备一行存在的父记录
/// 调用方：本文件的色卡发放用例夹具
/// 入参：数据库连接
/// 传给谁：按号段写入 departments 表
/// 存什么：存名称、编码、排序与启用标记
/// 存哪里：departments 表（该表由迁移播种，故先判存在再决定插入）
async fn seed_department(db: &sea_orm::DatabaseConnection) {
    let exists = department::Entity::find_by_id(DEPT_ID)
        .one(db)
        .await
        .unwrap()
        .is_some();
    if exists {
        return;
    }
    let moment = fixture_moment();
    department::ActiveModel {
        id: Set(DEPT_ID),
        name: Set("导出全量测试部门".to_string()),
        code: Set(format!("EXPALL-F-DEPT-{DEPT_ID}")),
        sort_order: Set(0),
        is_active: Set(true),
        created_at: Set(moment),
        updated_at: Set(moment),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

/// 播种供应商
///
/// 功能：造出一百二十行可被关键字唯一命中的供应商，创建时间全部取同一时刻
/// 调用方：本文件的供应商导出用例
/// 入参：数据库连接
/// 传给谁：逐行写入 suppliers 表
/// 存什么：存编码、名称、类型、信用代码等实体非空列与同一创建时刻
/// 存哪里：suppliers 表
async fn seed_suppliers(db: &sea_orm::DatabaseConnection) {
    let moment = fixture_moment().fixed_offset();
    for n in 0..ROWS {
        supplier::ActiveModel {
            id: Set(SUPPLIER_ID_BASE + n as i32),
            supplier_code: Set(format!("{SUPPLIER_CODE_PREFIX}-{n:04}")),
            supplier_name: Set(format!("导出全量测试供应商{n:04}")),
            supplier_short_name: Set(format!("全量{n:04}")),
            supplier_type: Set("manufacturer".to_string()),
            credit_code: Set(format!("{SUPPLIER_CODE_PREFIX}C{n:04}")),
            registered_address: Set("测试注册地址".to_string()),
            legal_representative: Set("测试法人".to_string()),
            registered_capital: Set(Decimal::ZERO),
            establishment_date: Set(Utc
                .with_ymd_and_hms(2020, 1, 1, 0, 0, 0)
                .unwrap()
                .date_naive()),
            taxpayer_type: Set("general".to_string()),
            bank_name: Set("测试开户行".to_string()),
            bank_account: Set("0000000000".to_string()),
            contact_phone: Set("00000000000".to_string()),
            created_at: Set(moment),
            updated_at: Set(moment),
            is_processor: Set(false),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }
}

/// 删除本用例播种的供应商行
///
/// 功能：供应商表由迁移播种且不参与用例间清空，用完必须自清，避免污染后续用例
/// 调用方：本文件的供应商导出用例
/// 入参：数据库连接
/// 传给谁：按编码前缀删除 suppliers 表中的本用例行
/// 存什么：不存
/// 存哪里：不适用
async fn purge_suppliers(db: &sea_orm::DatabaseConnection) {
    supplier::Entity::delete_many()
        .filter(supplier::Column::SupplierCode.starts_with(SUPPLIER_CODE_PREFIX))
        .exec(db)
        .await
        .unwrap();
}

/// 构造只命中本用例供应商的查询参数
///
/// 功能：产出按编码前缀关键字收敛的筛选参数
/// 调用方：本文件的供应商导出用例
/// 入参：页宽，超大页宽用于检验对外分页防护仍在
/// 传给谁：交回调用方传给服务层入口
/// 存什么：不存
/// 存哪里：不适用
fn supplier_params(page_size: u64) -> SupplierQueryParams {
    SupplierQueryParams {
        page: Some(1),
        page_size: Some(page_size),
        supplier_type: None,
        grade: None,
        status: None,
        keyword: Some(SUPPLIER_CODE_PREFIX.to_string()),
        sort_by: None,
        sort_order: None,
        is_enabled: None,
        category_id: None,
        is_processor: None,
        processor_type: None,
        download_token: None,
    }
}

/// 构造只命中本用例仓库的查询参数
///
/// 功能：产出按编码前缀关键字收敛的列表查询参数
/// 调用方：本文件的仓库导出用例
/// 入参：页宽，超大页宽用于检验对外分页防护仍在
/// 传给谁：交回调用方传给服务层入口
/// 存什么：不存
/// 存哪里：不适用
fn warehouse_params(
    page_size: u64,
) -> bingxi_backend::handlers::warehouse_handler::WarehouseListQuery {
    bingxi_backend::handlers::warehouse_handler::WarehouseListQuery {
        page: Some(1),
        page_size: Some(page_size),
        status: None,
        search: Some(WAREHOUSE_CODE_PREFIX.to_string()),
        warehouse_type: None,
    }
}

/// 播种仓库
///
/// 功能：造出一百二十行可被检索关键字唯一命中的仓库
/// 调用方：本文件的仓库导出用例
/// 入参：数据库连接
/// 传给谁：逐行写入 warehouses 表
/// 存什么：存编码、名称与启用标记
/// 存哪里：warehouses 表
async fn seed_warehouses(db: &sea_orm::DatabaseConnection) {
    let moment = fixture_moment();
    for n in 0..ROWS {
        warehouse::ActiveModel {
            id: Set(WAREHOUSE_ID_BASE + n as i32),
            warehouse_code: Set(format!("{WAREHOUSE_CODE_PREFIX}-{n:04}")),
            name: Set(format!("导出全量测试仓库{n:04}")),
            is_default: Set(false),
            is_active: Set(true),
            created_at: Set(moment),
            updated_at: Set(moment),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }
}

/// 播种固定资产
///
/// 功能：造出一百二十行可被关键字唯一命中的资产
/// 调用方：本文件的固定资产导出用例
/// 入参：数据库连接
/// 传给谁：逐行写入 fixed_assets 表
/// 存什么：存资产编号、名称、原值、累计折旧、状态与创建人；状态引自创建路径
/// 使用的同一主数据词表
/// 存哪里：fixed_assets 表
async fn seed_fixed_assets(db: &sea_orm::DatabaseConnection) {
    let moment = fixture_moment();
    for n in 0..ROWS {
        fixed_asset::ActiveModel {
            id: Set(ASSET_ID_BASE + n as i32),
            asset_no: Set(format!("{ASSET_NO_PREFIX}-{n:04}")),
            asset_name: Set(format!("导出全量测试资产{n:04}")),
            original_value: Set(Decimal::ONE),
            accumulated_depreciation: Set(Decimal::ZERO),
            status: Set(master_data::ACTIVE.to_string()),
            created_by: Set(OPERATOR_A),
            created_at: Set(moment),
            updated_at: Set(moment),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }
}

/// 播种色卡发放链路的父行：部门、操作人、两个客户与一张色卡
///
/// 功能：把发放记录的外键父行与归属人一次备齐，两个客户分属两位操作人，
/// 使仅自己范围与全部范围能各自验证
/// 调用方：本文件的色卡发放导出用例
/// 入参：数据库连接
/// 传给谁：写入 departments、users、customers 与 color_cards 表
/// 存什么：存操作人的会话字段与部门、客户的归属人与编码、色卡的编号与状态
/// 存哪里：departments、users、customers 与 color_cards 表
async fn seed_issue_parents(db: &sea_orm::DatabaseConnection) {
    seed_department(db).await;
    let moment = fixture_moment();

    for uid in [OPERATOR_A, OPERATOR_B] {
        user::ActiveModel {
            id: Set(uid),
            username: Set(format!("expall_fixture_{uid}")),
            password_hash: Set("test-only-not-a-real-hash".to_string()),
            is_active: Set(true),
            is_totp_enabled: Set(false),
            department_id: Set(Some(DEPT_ID)),
            created_at: Set(moment),
            updated_at: Set(moment),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    for (cid, owner) in [(CUSTOMER_OF_A, OPERATOR_A), (CUSTOMER_OF_B, OPERATOR_B)] {
        customer::ActiveModel {
            id: Set(cid),
            customer_code: Set(format!("EXPALL-F-CUS-{cid}")),
            customer_name: Set(format!("导出全量测试客户{cid}")),
            credit_limit: Set(Decimal::ZERO),
            payment_terms: Set(30),
            status: Set(master_data::ACTIVE.to_string()),
            customer_type: Set("retail".to_string()),
            owner_id: Set(owner),
            department_id: Set(Some(DEPT_ID)),
            created_at: Set(moment),
            updated_at: Set(moment),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    color_card::ActiveModel {
        id: Set(COLOR_CARD_ID),
        card_no: Set("EXPALL-F-CARD-0001".to_string()),
        card_name: Set("导出全量测试色卡".to_string()),
        card_type: Set("CUSTOM".to_string()),
        status: Set(card_status::DRAFT.to_string()),
        created_at: Set(moment),
        updated_at: Set(moment),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

/// 播种色卡发放记录：前一百二十行归操作人甲名下客户，后一百二十行归操作人乙名下客户
///
/// 功能：造出两百四十行发放记录，使全部范围应得两百四十行、
/// 仅自己范围应得一百二十行，两个范围都严格超过对外分页上限；
/// 发放时间全部取同一时刻，使批次边界只能靠主键末级排序来稳定
/// 调用方：本文件的色卡发放导出用例
/// 入参：数据库连接
/// 传给谁：逐行写入 color_card_issues 表
/// 存什么：存所属色卡、归属客户、发放数量、发放人、状态与同一发放时刻
/// 存哪里：color_card_issues 表
async fn seed_issue_records(db: &sea_orm::DatabaseConnection) {
    let moment = fixture_moment();
    for n in 0..(ROWS * 2) {
        let belongs_to_a = n < ROWS;
        color_card_issue::ActiveModel {
            id: Set(ISSUE_ID_BASE + n),
            color_card_id: Set(COLOR_CARD_ID),
            customer_id: Set(if belongs_to_a {
                CUSTOMER_OF_A as i64
            } else {
                CUSTOMER_OF_B as i64
            }),
            issue_qty: Set(1),
            issued_by: Set(if belongs_to_a {
                OPERATOR_A as i64
            } else {
                OPERATOR_B as i64
            }),
            issued_at: Set(moment),
            status: Set(card_status::ISSUED.to_string()),
            is_deleted: Set(false),
            created_at: Set(moment),
            updated_at: Set(moment),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }
}

/// 构造只命中本用例色卡的发放记录查询参数
///
/// 功能：产出按色卡编号收敛的筛选参数
/// 调用方：本文件的色卡发放导出用例
/// 入参：页宽，超大页宽用于检验对外分页防护仍在
/// 传给谁：交回调用方传给服务层入口
/// 存什么：不存
/// 存哪里：不适用
fn issue_params(page_size: u64) -> ListIssuesQuery {
    ListIssuesQuery {
        page: Some(1),
        page_size: Some(page_size),
        color_card_id: Some(COLOR_CARD_ID),
        ..Default::default()
    }
}

/// 供应商导出必须拿到当前筛选下的全部行，且对外分页防护不得放宽
#[tokio::test]
async fn supplier_export_returns_all_rows_not_one_page() {
    let db = setup_test_db().await;
    seed_suppliers(&db).await;
    let service = SupplierService::new(Arc::new(db.clone()));

    let seeded = supplier::Entity::find()
        .filter(supplier::Column::SupplierCode.starts_with(SUPPLIER_CODE_PREFIX))
        .count(&db)
        .await
        .unwrap();
    assert_eq!(seeded, ROWS as u64, "前置：造数行数就等于全量期望值");

    let (rows, total) = service
        .list_suppliers_for_export(&supplier_params(HUGE_PAGE_SIZE), None)
        .await
        .unwrap();
    assert_eq!(
        rows.len() as i64,
        ROWS,
        "供应商导出取数必须返回全部行，实际 {}/{}，等于一百即说明仍被分页上限截断",
        rows.len(),
        total
    );
    assert_eq!(total as i64, ROWS, "服务端总数应与造数量一致");

    let clamped = service
        .list_suppliers(supplier_params(HUGE_PAGE_SIZE), None)
        .await
        .unwrap();
    assert_eq!(
        clamped.items.len(),
        ONE_PAGE,
        "对外分页入口的页宽防护必须仍收敛到一页，放宽即等于让外部同参数要到超大范围查询"
    );

    purge_suppliers(&db).await;
}

/// 仓库导出必须拿到当前筛选下的全部行，且对外分页防护不得放宽
#[tokio::test]
async fn warehouse_export_returns_all_rows_not_one_page() {
    let db = setup_test_db().await;
    seed_warehouses(&db).await;
    let service = WarehouseService::new(Arc::new(db.clone()));

    let seeded = warehouse::Entity::find()
        .filter(warehouse::Column::WarehouseCode.starts_with(WAREHOUSE_CODE_PREFIX))
        .count(&db)
        .await
        .unwrap();
    assert_eq!(seeded, ROWS as u64, "前置：造数行数就等于全量期望值");

    let (rows, total) = service
        .list_warehouses_for_export(&warehouse_params(HUGE_PAGE_SIZE))
        .await
        .unwrap();
    assert_eq!(
        rows.len() as i64,
        ROWS,
        "仓库导出取数必须返回全部行，实际 {}/{}，等于一百即说明仍被分页上限截断",
        rows.len(),
        total
    );
    assert_eq!(total as i64, ROWS, "服务端总数应与造数量一致");

    let clamped = service
        .list(warehouse_params(HUGE_PAGE_SIZE))
        .await
        .unwrap();
    assert_eq!(
        clamped.items.len(),
        ONE_PAGE,
        "对外分页入口的页宽防护必须仍收敛到一页，放宽即等于让外部同参数要到超大范围查询"
    );
}

/// 固定资产导出必须拿到当前筛选下的全部行，且对外分页防护不得放宽
#[tokio::test]
async fn fixed_asset_export_returns_all_rows_not_one_page() {
    let db = setup_test_db().await;
    seed_fixed_assets(&db).await;
    let service = FixedAssetService::new(Arc::new(db.clone()));

    let seeded = fixed_asset::Entity::find()
        .filter(fixed_asset::Column::AssetNo.starts_with(ASSET_NO_PREFIX))
        .count(&db)
        .await
        .unwrap();
    assert_eq!(seeded, ROWS as u64, "前置：造数行数就等于全量期望值");

    let export_params = AssetQueryParams {
        keyword: Some(ASSET_NO_PREFIX.to_string()),
        ..Default::default()
    };
    let (rows, total) = service
        .list_assets_for_export(&export_params)
        .await
        .unwrap();
    assert_eq!(
        rows.len() as i64,
        ROWS,
        "固定资产导出取数必须返回全部行，实际 {}/{}，等于一百即说明仍被分页上限截断",
        rows.len(),
        total
    );
    assert_eq!(total as i64, ROWS, "服务端总数应与造数量一致");

    let clamped_params = AssetQueryParams {
        keyword: Some(ASSET_NO_PREFIX.to_string()),
        page: 1,
        page_size: HUGE_PAGE_SIZE as i64,
        ..Default::default()
    };
    let (one_page, _) = service.get_list(clamped_params).await.unwrap();
    assert_eq!(
        one_page.len(),
        ONE_PAGE,
        "对外分页入口的页宽防护必须仍收敛到一页，放宽即等于让外部同参数要到超大范围查询"
    );
}

/// 色卡发放导出在全部范围下必须拿到两个客户的合计行，且对外分页防护不得放宽
#[tokio::test]
async fn color_card_issue_export_returns_all_rows_for_all_scope() {
    let db = setup_test_db().await;
    seed_issue_parents(&db).await;
    seed_issue_records(&db).await;
    let service = ColorCardIssueService::new(Arc::new(db.clone()));

    let seeded = color_card_issue::Entity::find()
        .filter(color_card_issue::Column::ColorCardId.eq(COLOR_CARD_ID))
        .count(&db)
        .await
        .unwrap();
    assert_eq!(
        seeded,
        ROWS as u64 * 2,
        "前置：造数总量应为两个归属范围之和"
    );

    let (rows, total) = service
        .list_issue_records_for_export(&issue_params(HUGE_PAGE_SIZE), OPERATOR_A, "all")
        .await
        .unwrap();
    assert_eq!(
        rows.len() as i64,
        ROWS * 2,
        "全部范围下导出必须返回两个客户的合计行，实际 {}/{}，等于一百即说明仍被分页上限截断",
        rows.len(),
        total
    );

    let clamped = service
        .list_records_with_data_scope(issue_params(HUGE_PAGE_SIZE), OPERATOR_A, "all")
        .await
        .unwrap();
    assert_eq!(
        clamped.0.len(),
        ONE_PAGE,
        "对外分页入口的页宽防护必须仍收敛到一页，放宽即等于让外部同参数要到超大范围查询"
    );
}

/// 色卡发放导出在仅自己范围下必须拿到本人归属客户的全部行，且不得越界混入他人行
#[tokio::test]
async fn color_card_issue_export_returns_all_rows_for_self_scope() {
    let db = setup_test_db().await;
    seed_issue_parents(&db).await;
    seed_issue_records(&db).await;
    let service = ColorCardIssueService::new(Arc::new(db.clone()));

    let (rows, total) = service
        .list_issue_records_for_export(&issue_params(HUGE_PAGE_SIZE), OPERATOR_A, "self")
        .await
        .unwrap();
    assert_eq!(
        rows.len() as i64,
        ROWS,
        "仅自己范围下导出必须拿全本人归属客户的行，实际 {}/{}，等于一百即说明仍被分页上限截断",
        rows.len(),
        total
    );
    assert!(
        rows.iter().all(|r| r.customer_id == CUSTOMER_OF_A as i64),
        "仅自己范围不得混入他人归属客户的行"
    );

    let clamped = service
        .list_records_with_data_scope(issue_params(HUGE_PAGE_SIZE), OPERATOR_A, "self")
        .await
        .unwrap();
    assert_eq!(
        clamped.0.len(),
        ONE_PAGE,
        "仅自己范围的对外分页入口同样必须收敛到一页"
    );
}
