//! 批次 190 迁移整合：执行所有未被 Rust 模块引用的 SQL 迁移
//!
//! 问题背景：
//! 31 个 SQL 迁移目录（20260616/17/18/0706/0707 系列）从未被 Rust 迁移模块引用，
//! 导致这些表/列从未创建。m0029_drop_tenant_columns 尝试 ALTER TABLE custom_orders
//! 时报错 "relation custom_orders does not exist"。
//!
//! 整合方案（规则 0/2 真实修复 + 用户指示"迁移文件太多需要整合"）：
//! 用 include_str! 编译期嵌入所有未引用 SQL 的 up.sql，按依赖顺序执行。
//! 所有 SQL 均使用 IF NOT EXISTS / IF EXISTS，保证幂等可重入。
//! 本迁移注册在 m0028 之后、m0029 之前，确保 custom_orders 等表在 drop_tenant_columns 之前创建。

use sea_orm_migration::prelude::*;

/// 整合迁移条目：(名称, up.sql 内容)
const UNREFERENCED_MIGRATIONS: &[(&str, &str)] = &[
    // 20260616 系列（容灾表）
    ("20260616000005_create_failover_tables", include_str!(
        "../../migrations/20260616000005_create_failover_tables/up.sql"
    )),
    // 20260617 系列（定制订单/工艺/质量/售后/色卡/AI/维度表）
    ("20260617000001_create_custom_orders", include_str!(
        "../../migrations/20260617000001_create_custom_orders/up.sql"
    )),
    ("20260617000002_create_process_nodes", include_str!(
        "../../migrations/20260617000002_create_process_nodes/up.sql"
    )),
    ("20260617000003_create_process_logs", include_str!(
        "../../migrations/20260617000003_create_process_logs/up.sql"
    )),
    ("20260617000004_create_quality_issues", include_str!(
        "../../migrations/20260617000004_create_quality_issues/up.sql"
    )),
    ("20260617000005_create_after_sales", include_str!(
        "../../migrations/20260617000005_create_after_sales/up.sql"
    )),
    ("20260617000006_create_color_cards", include_str!(
        "../../migrations/20260617000006_create_color_cards/up.sql"
    )),
    ("20260617000007_create_color_card_items", include_str!(
        "../../migrations/20260617000007_create_color_card_items/up.sql"
    )),
    ("20260617000008_create_color_card_borrow_records", include_str!(
        "../../migrations/20260617000008_create_color_card_borrow_records/up.sql"
    )),
    ("20260617000009_create_ai_process_optimizations", include_str!(
        "../../migrations/20260617000009_create_ai_process_optimizations/up.sql"
    )),
    ("20260617000010_create_ai_quality_predictions", include_str!(
        "../../migrations/20260617000010_create_ai_quality_predictions/up.sql"
    )),
    ("20260617000011_create_sales_facts", include_str!(
        "../../migrations/20260617000011_create_sales_facts/up.sql"
    )),
    ("20260617000012_create_dim_products", include_str!(
        "../../migrations/20260617000012_create_dim_products/up.sql"
    )),
    ("20260617000013_create_dim_customers", include_str!(
        "../../migrations/20260617000013_create_dim_customers/up.sql"
    )),
    ("20260617000014_create_dim_dates", include_str!(
        "../../migrations/20260617000014_create_dim_dates/up.sql"
    )),
    // 20260618 系列（增强版销售报价/色价历史/梯度/客户色价/季节性价格）
    ("20260618000001_create_sales_quotations", include_str!(
        "../../migrations/20260618000001_create_sales_quotations/up.sql"
    )),
    ("20260618000001_extend_product_color_prices", include_str!(
        "../../migrations/20260618000001_extend_product_color_prices/up.sql"
    )),
    ("20260618000002_create_color_price_history", include_str!(
        "../../migrations/20260618000002_create_color_price_history/up.sql"
    )),
    ("20260618000002_create_sales_quotation_items", include_str!(
        "../../migrations/20260618000002_create_sales_quotation_items/up.sql"
    )),
    ("20260618000003_create_color_price_tiers", include_str!(
        "../../migrations/20260618000003_create_color_price_tiers/up.sql"
    )),
    ("20260618000003_create_sales_quotation_terms", include_str!(
        "../../migrations/20260618000003_create_sales_quotation_terms/up.sql"
    )),
    ("20260618000004_create_customer_color_prices", include_str!(
        "../../migrations/20260618000004_create_customer_color_prices/up.sql"
    )),
    ("20260618000005_create_seasonal_price_rules", include_str!(
        "../../migrations/20260618000005_create_seasonal_price_rules/up.sql"
    )),
    // 20260706 系列（TOTP/库存对齐/跟踪/预算）
    ("20260706000001_add_totp_recovery_codes_to_users", include_str!(
        "../../migrations/20260706000001_add_totp_recovery_codes_to_users/up.sql"
    )),
    ("20260706000002_align_inventory_count_schema", include_str!(
        "../../migrations/20260706000002_align_inventory_count_schema/up.sql"
    )),
    ("20260706000003_create_tracking_tables", include_str!(
        "../../migrations/20260706000003_create_tracking_tables/up.sql"
    )),
    ("20260706000004_add_max_stock_point_to_inventory_stocks", include_str!(
        "../../migrations/20260706000004_add_max_stock_point_to_inventory_stocks/up.sql"
    )),
    ("20260706000005_extend_budget_management", include_str!(
        "../../migrations/20260706000005_extend_budget_management/up.sql"
    )),
    // 20260707 系列（仓库容量/API密钥描述/密码历史）
    ("20260707000001_add_capacity_to_warehouses", include_str!(
        "../../migrations/20260707000001_add_capacity_to_warehouses/up.sql"
    )),
    ("20260707000002_add_description_to_api_keys", include_str!(
        "../../migrations/20260707000002_add_description_to_api_keys/up.sql"
    )),
    ("20260707000003_create_password_histories", include_str!(
        "../../migrations/20260707000003_create_password_histories/up.sql"
    )),
];

/// 修复 SQL 中 BIGINT 外键类型不匹配问题
///
/// 引用 SERIAL (INTEGER) id 列的 BIGINT 外键会导致 PostgreSQL 报错。
/// 这些表由 m0001/m0002/m0003 等 Rust 迁移创建，id 类型为 SERIAL (INTEGER)。
/// 引用 BIGSERIAL (BIGINT) id 列的 BIGINT 外键无需修改。
fn fix_fk_types(sql: &str) -> String {
    // id 类型为 SERIAL (INTEGER) 的表
    const INTEGER_ID_TABLES: &[&str] = &[
        "users",
        "products",
        "customers",
        "dye_recipe",
        "sales_orders",
        "product_categories",
        "warehouses",
        "suppliers",
        "product_colors",
        "fixed_assets",
    ];
    let mut result = sql.to_string();
    for table in INTEGER_ID_TABLES {
        // BIGINT NOT NULL REFERENCES "table"("id") → INTEGER NOT NULL REFERENCES
        let bigint_nn = format!("BIGINT NOT NULL REFERENCES \"{}\"(\"id\")", table);
        let integer_nn = format!("INTEGER NOT NULL REFERENCES \"{}\"(\"id\")", table);
        result = result.replace(&bigint_nn, &integer_nn);
        // BIGINT REFERENCES "table"("id") → INTEGER REFERENCES
        let bigint = format!("BIGINT REFERENCES \"{}\"(\"id\")", table);
        let integer = format!("INTEGER REFERENCES \"{}\"(\"id\")", table);
        result = result.replace(&bigint, &integer);
    }
    result
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for (name, sql) in UNREFERENCED_MIGRATIONS {
            if !sql.trim().is_empty() {
                // 修复 BIGINT 外键类型不匹配后再执行
                let fixed_sql = fix_fk_types(sql);
                db.execute_unprepared(&fixed_sql).await.map_err(|e| {
                    DbErr::Custom(format!("执行整合迁移 {} 失败: {}", name, e))
                })?;
            }
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // 整合迁移不支持回滚（含 CREATE TABLE 等不可逆操作）
        Ok(())
    }
}
