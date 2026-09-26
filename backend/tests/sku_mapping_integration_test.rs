//! 供应商对照表服务真实 DB 集成测试（需已迁移 PostgreSQL）
//!
//! 沿用仓库既有 `#[ignore]` 真库用例范式（见 services_bom_service_test.rs）：
//! 无 schema harness（sqlite::memory 无业务表），故这些用例标注 `#[ignore]`，
//! 仅在 `TEST_DATABASE_URL` 指向已 `bingxi migrate run` 的 PostgreSQL 时手动/CI 运行。
//!
//! 这里只覆盖「无需外键种子链即可判定」的分支，避免脆弱的多表 ActiveModel 拼装
//! （product/supplier/supplier_product 等 NOT NULL 列众多，无法本地类型校验），
//! 其余需真实关联数据的行为分支由以下两层保证：
//! - 源码结构契约：tests/sku_mapping_contract_test.rs（编译期锁死分支语义）
//! - e2e 打真实 server+DB：frontend/e2e/purchase/sku-mapping.spec.ts
//!
//! 覆盖：
//! - A1 resolve 无映射 → Ok(None)（非 Err）
//! - A3 validate_refs：产品不存在 → ValidationError；update 记录不存在 → NotFound

use bingxi_backend::services::sku_mapping_service::{SkuMappingService, UpsertSkuMappingInput};
use bingxi_backend::services::test_common::setup_test_db;
use bingxi_backend::utils::error::AppError;
use std::sync::Arc;

/// 构造一个 product_id 指向不存在记录的输入（validate_refs 第一步即命中产品查找失败）。
fn input_with_nonexistent_refs() -> UpsertSkuMappingInput {
    UpsertSkuMappingInput {
        // 999999 在空/迁移库中不存在（若极端巧合存在则用不可能负值区间的 id）
        product_id: 999_999,
        product_color_id: None,
        supplier_id: 999_998,
        supplier_product_id: 999_997,
        supplier_product_color_id: None,
        supplier_price: None,
        min_order_quantity: None,
        lead_time: None,
        is_primary: None,
        priority: None,
        is_enabled: None,
        remarks: None,
    }
}

/// A1：resolve 无映射返回 `Ok(None)`（契约核心：不再抛错）。
#[tokio::test]
#[ignore = "需已迁移 PostgreSQL（TEST_DATABASE_URL）；无映射记录时表存在但查空即返回 Ok(None)"]
async fn resolve_no_mapping_returns_ok_none() {
    let db = setup_test_db().await;
    let svc = SkuMappingService::new(Arc::new(db));

    // 任意 (product_id, product_color_id, supplier_id) 组合在迁移库中默认无对照记录
    let result = svc
        .resolve_supplier_sku(999_999, Some(999_998), 999_997)
        .await;

    assert!(
        result.is_ok(),
        "resolve 无映射应返回 Ok 而非 Err，实际={result:?}"
    );
    assert!(
        result.expect("上一步已断言 Ok").is_none(),
        "resolve 无映射应返回 Ok(None)"
    );
}

/// A1 补充：product_color_id 为 None 时查 IS NULL 行，同样应 Ok(None)。
#[tokio::test]
#[ignore = "需已迁移 PostgreSQL（TEST_DATABASE_URL）"]
async fn resolve_none_color_id_returns_ok_none() {
    let db = setup_test_db().await;
    let svc = SkuMappingService::new(Arc::new(db));

    let result = svc.resolve_supplier_sku(999_999, None, 999_997).await;
    assert!(result.is_ok(), "resolve(无 color) 应 Ok，实际={result:?}");
    assert!(result.expect("已断言 Ok").is_none());
}

/// A3：create 时产品不存在 → validate_refs 命中「产品 ID 不存在」→ ValidationError。
#[tokio::test]
#[ignore = "需已迁移 PostgreSQL（TEST_DATABASE_URL）；validate_refs 走 product 表查找"]
async fn create_with_nonexistent_product_is_validation_error() {
    let db = setup_test_db().await;
    let svc = SkuMappingService::new(Arc::new(db));

    let result = svc.create(input_with_nonexistent_refs(), 1).await;
    match result {
        Err(e @ AppError::ValidationError(_)) => {
            // Display 携带真实定位文案（产品 ID 不存在）
            assert!(
                e.to_string().contains("不存在"),
                "应返回产品不存在类 validation 文案，实际={e}"
            );
        }
        other => panic!("validate_refs 产品缺失应返回 ValidationError，实际={other:?}"),
    }
}

/// update 到不存在的对照记录 id → NotFound。
#[tokio::test]
#[ignore = "需已迁移 PostgreSQL（TEST_DATABASE_URL）"]
async fn update_nonexistent_mapping_is_not_found() {
    let db = setup_test_db().await;
    let svc = SkuMappingService::new(Arc::new(db));

    let result = svc.update(987_654, input_with_nonexistent_refs(), 1).await;
    assert!(
        matches!(result, Err(AppError::NotFound(_))),
        "更新不存在记录应 NotFound，实际={result:?}"
    );
}

/// import_batch 空批次 → 各计数为 0（error_count 键语义，无需种子）。
#[tokio::test]
#[ignore = "需已迁移 PostgreSQL（TEST_DATABASE_URL）"]
async fn import_batch_empty_returns_zero_counts() {
    let db = setup_test_db().await;
    let svc = SkuMappingService::new(Arc::new(db));

    let result = svc.import_batch(vec![], 1).await.expect("空批次导入应 Ok");
    assert_eq!(result.total_count, 0);
    assert_eq!(result.success_count, 0);
    assert_eq!(result.error_count, 0);
}
