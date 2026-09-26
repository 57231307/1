//! 供应商商品/色号目录 真实 DB 集成测试（需已迁移 + 已种子的 PostgreSQL）
//!
//! 沿用仓库既有 `#[ignore]` 真库用例范式（见 sku_mapping_integration_test.rs）：
//! 无 schema harness，故标注 `#[ignore]`，仅在 `TEST_DATABASE_URL` 指向已
//! `bingxi migrate run`（含 business/m0015 目录种子）的 PostgreSQL 时运行。
//!
//! 覆盖真实链路：
//! - supplier_product：create → 按父级 list 命中 → keyword 命中 / keyword 不命中零结果；
//! - supplier_product_color：create（挂到刚建的父商品）→ list 命中 → keyword 命中；
//! - 父级不存在 create → AppError::validation（VALIDATION_ERROR）。
//! - 重复 product_code create → business_displayable（真实文案，非脱敏）。

use bingxi_backend::models::supplier;
use bingxi_backend::services::supplier_product_color_service::{
    CreateSupplierProductColorRequest, SupplierProductColorQueryParams, SupplierProductColorService,
};
use bingxi_backend::services::supplier_product_service::{
    CreateSupplierProductRequest, SupplierProductQueryParams, SupplierProductService,
};
use bingxi_backend::services::test_common::setup_test_db;
use bingxi_backend::utils::error::AppError;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
use std::sync::Arc;

fn unique_suffix() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos().to_string())
        .unwrap_or_default()
}

/// 取一个真实存在的供应商 ID（迁移+种子库应至少有 m0015 演示供应商，或 e2e 建过的供应商）。
async fn first_supplier_id(db: &sea_orm::DatabaseConnection) -> i32 {
    supplier::Entity::find()
        .select_only()
        .column(supplier::Column::Id)
        .into_tuple::<i32>()
        .one(db)
        .await
        .expect("查询供应商失败")
        .expect("迁移/种子库中无任何供应商，无法运行目录集成用例（需先 bingxi migrate run）")
}

#[tokio::test]
#[ignore = "需已迁移+种子 PostgreSQL（TEST_DATABASE_URL）"]
async fn supplier_product_create_list_keyword_roundtrip() {
    let db = Arc::new(setup_test_db().await);
    let svc = SupplierProductService::new(db.clone());
    let supplier_id = first_supplier_id(&db).await;
    let code = format!("IT-SP-{}", unique_suffix());

    let created = svc
        .create(
            CreateSupplierProductRequest {
                supplier_id,
                product_code: code.clone(),
                product_name: "集成测试面料".to_string(),
                product_description: Some("desc".to_string()),
                unit: "米".to_string(),
                is_enabled: None,
                remarks: None,
            },
            1,
        )
        .await
        .expect("create 供应商商品应成功");
    assert_eq!(created.product_code, code);

    // 按父级 list 命中
    let (items, _total, _p, _ps) = svc
        .list(SupplierProductQueryParams {
            supplier_id: Some(supplier_id),
            keyword: None,
            is_enabled: None,
            page: Some(1),
            page_size: Some(100),
        })
        .await
        .expect("list 应成功");
    assert!(
        items.iter().any(|m| m.id == created.id),
        "按 supplier_id 过滤应命中刚创建的记录"
    );

    // keyword 命中（按 product_code LIKE）
    let (hit, _, _, _) = svc
        .list(SupplierProductQueryParams {
            supplier_id: Some(supplier_id),
            keyword: Some(code.clone()),
            is_enabled: None,
            page: Some(1),
            page_size: Some(100),
        })
        .await
        .expect("keyword list 应成功");
    assert!(
        hit.iter().any(|m| m.id == created.id),
        "keyword 命中的 product_code 应包含刚创建记录"
    );

    // keyword 不命中 → 零结果
    let (miss, miss_total, _, _) = svc
        .list(SupplierProductQueryParams {
            supplier_id: Some(supplier_id),
            keyword: Some(format!("ZZZ-NOPE-{}", unique_suffix())),
            is_enabled: None,
            page: Some(1),
            page_size: Some(100),
        })
        .await
        .expect("keyword 不命中 list 应成功");
    assert!(
        miss.is_empty() && miss_total == 0,
        "不匹配 keyword 应零结果"
    );
}

#[tokio::test]
#[ignore = "需已迁移+种子 PostgreSQL（TEST_DATABASE_URL）"]
async fn supplier_product_color_create_list_keyword_roundtrip() {
    let db = Arc::new(setup_test_db().await);
    let psvc = SupplierProductService::new(db.clone());
    let csvc = SupplierProductColorService::new(db.clone());
    let supplier_id = first_supplier_id(&db).await;
    let code = format!("IT-SPC-{}", unique_suffix());

    let product = psvc
        .create(
            CreateSupplierProductRequest {
                supplier_id,
                product_code: code.clone(),
                product_name: "色号集成父商品".to_string(),
                product_description: None,
                unit: "米".to_string(),
                is_enabled: None,
                remarks: None,
            },
            1,
        )
        .await
        .expect("创建父商品应成功");

    let color_no = format!("IT-C-{}", unique_suffix());
    let created = csvc
        .create(CreateSupplierProductColorRequest {
            supplier_product_id: product.id,
            color_no: color_no.clone(),
            color_name: "集成色".to_string(),
            pantone_code: Some("PANTONE 19-0000".to_string()),
            extra_cost: Some("1.25".to_string()),
            is_enabled: None,
        })
        .await
        .expect("create 供应商色号应成功");
    assert_eq!(created.color_no, color_no);
    assert_eq!(created.extra_cost.to_string(), "1.25");

    let (items, _, _, _) = csvc
        .list(SupplierProductColorQueryParams {
            supplier_product_id: Some(product.id),
            keyword: None,
            is_enabled: None,
            page: Some(1),
            page_size: Some(100),
        })
        .await
        .expect("色号 list 应成功");
    assert!(
        items.iter().any(|m| m.id == created.id),
        "应按父商品命中色号"
    );

    let (hit, _, _, _) = csvc
        .list(SupplierProductColorQueryParams {
            supplier_product_id: Some(product.id),
            keyword: Some(color_no),
            is_enabled: None,
            page: Some(1),
            page_size: Some(100),
        })
        .await
        .expect("色号 keyword list 应成功");
    assert!(
        hit.iter().any(|m| m.id == created.id),
        "keyword 应命中色号编码"
    );
}

#[tokio::test]
#[ignore = "需已迁移+种子 PostgreSQL（TEST_DATABASE_URL）"]
async fn supplier_product_create_with_missing_supplier_is_validation_error() {
    let db = Arc::new(setup_test_db().await);
    let svc = SupplierProductService::new(db.clone());

    let result = svc
        .create(
            CreateSupplierProductRequest {
                supplier_id: 999_999,
                product_code: format!("IT-BAD-{}", unique_suffix()),
                product_name: "x".to_string(),
                product_description: None,
                unit: "米".to_string(),
                is_enabled: None,
                remarks: None,
            },
            1,
        )
        .await;
    match result {
        Err(AppError::ValidationError(_)) => {}
        other => panic!("父级供应商不存在应返回 ValidationError，实际={other:?}"),
    }
}

#[tokio::test]
#[ignore = "需已迁移+种子 PostgreSQL（TEST_DATABASE_URL）"]
async fn supplier_product_duplicate_code_is_displayable_business_error() {
    let db = Arc::new(setup_test_db().await);
    let svc = SupplierProductService::new(db.clone());
    let supplier_id = first_supplier_id(&db).await;
    let code = format!("IT-DUP-{}", unique_suffix());

    svc.create(
        CreateSupplierProductRequest {
            supplier_id,
            product_code: code.clone(),
            product_name: "重复测试".to_string(),
            product_description: None,
            unit: "米".to_string(),
            is_enabled: None,
            remarks: None,
        },
        1,
    )
    .await
    .expect("首次创建应成功");

    let dup = svc
        .create(
            CreateSupplierProductRequest {
                supplier_id,
                product_code: code,
                product_name: "重复测试".to_string(),
                product_description: None,
                unit: "米".to_string(),
                is_enabled: None,
                remarks: None,
            },
            1,
        )
        .await;
    match dup {
        Err(e @ AppError::BusinessErrorDisplayable(_)) => {
            assert_eq!(e.error_code(), "BUSINESS_ERROR");
            assert!(
                e.to_response().message.contains("已存在"),
                "重复应外显真实业务文案，实际={}",
                e.to_response().message
            );
        }
        other => panic!("重复编码应返回 BusinessErrorDisplayable，实际={other:?}"),
    }
}
