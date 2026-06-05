//! 路由注册冒烟测试
//!
//! 验证 `src/routes/mod.rs` 中所有新增/修改的路由路径仍然存在。
//!
//! 设计思路：
//! - 由于 `create_router(state: AppState)` 需要真实的数据库连接（`AppState::default`
//!   在 release 模式下会 panic，且 `OmniAuditEngine` 等服务会启动后台任务），
//!   这里采用更轻量的静态分析方案：把 `routes/mod.rs` 的源码以字符串形式嵌入测试，
//!   断言每个期望的路径片段仍然出现在源码中。
//! - 这等价于"如果有人误删了某个路由的注册行，CI 就会失败"，从而对路由正确性
//!   提供最低程度的回归保护。
//! - 业务逻辑正确性由各 handler 的单元测试覆盖。

/// 把 `routes/mod.rs` 的源码嵌入到测试二进制中。
const ROUTES_SOURCE: &str = include_str!("../../src/routes/mod.rs");

/// 断言源码中包含所有给定路径片段（按出现顺序逐一检查）。
///
/// 选择按顺序查找而非一次性 collect，是为了在断言失败时定位到第一个缺失的路由。
fn assert_routes_registered(group_name: &str, expected_paths: &[&str]) {
    let mut missing: Vec<&str> = Vec::new();
    for path in expected_paths {
        if !ROUTES_SOURCE.contains(path) {
            missing.push(path);
        }
    }
    assert!(
        missing.is_empty(),
        "[{}] 以下路由路径未在 src/routes/mod.rs 中找到: {:?}",
        group_name,
        missing
    );
}

// -----------------------------------------------------------------------------
// 1. 生产订单
// -----------------------------------------------------------------------------
#[cfg(test)]
mod production_orders {
    use super::assert_routes_registered;

    #[test]
    fn test_production_order_routes_registered() {
        assert_routes_registered(
            "production_orders",
            &["/orders/:id/submit-approval", "/orders/:id/approve"],
        );
    }
}

// -----------------------------------------------------------------------------
// 2. MRP
// -----------------------------------------------------------------------------
#[cfg(test)]
mod mrp {
    use super::assert_routes_registered;

    #[test]
    fn test_mrp_routes_registered() {
        assert_routes_registered(
            "mrp",
            &[
                "/products",
                "/:id/cancel",
                "/:id/export",
                "/:calculation_id/materials/:material_id",
                "/convert-orders",
            ],
        );
    }
}

// -----------------------------------------------------------------------------
// 3. 币种 / 采购价格 / 排程 / 供应商评估
// -----------------------------------------------------------------------------
#[cfg(test)]
mod currency_purchase_scheduling_supplier {
    use super::assert_routes_registered;

    #[test]
    fn test_currency_routes_registered() {
        assert_routes_registered("currency", &["/exchange-rates", "/query"]);
    }

    #[test]
    fn test_purchase_price_routes_registered() {
        assert_routes_registered("purchase_price", &["/history/:product_id"]);
    }

    #[test]
    fn test_scheduling_routes_registered() {
        assert_routes_registered("scheduling", &["/tasks/:id/adjust"]);
    }

    #[test]
    fn test_supplier_evaluation_routes_registered() {
        assert_routes_registered("supplier_evaluation", &["/suppliers/:supplier_id/score"]);
    }
}

// -----------------------------------------------------------------------------
// 4. 销售分析
// -----------------------------------------------------------------------------
#[cfg(test)]
mod sales_analysis {
    use super::assert_routes_registered;

    #[test]
    fn test_sales_analysis_routes_registered() {
        assert_routes_registered(
            "sales_analysis",
            &[
                "/stats",
                "/product-ranking",
                "/customer-ranking",
                "/targets/:period",
                "/export",
            ],
        );
    }
}

// -----------------------------------------------------------------------------
// 5. 缺料预警 / BOM
// -----------------------------------------------------------------------------
#[cfg(test)]
mod material_shortage_bom {
    use super::assert_routes_registered;

    #[test]
    fn test_material_shortage_routes_registered() {
        assert_routes_registered("material_shortage", &["/:id/status"]);
    }

    #[test]
    fn test_bom_routes_registered() {
        assert_routes_registered("bom", &["/:id/submit", "/:id/approve"]);
    }
}

// -----------------------------------------------------------------------------
// 6. 染色批次 / 配方
// -----------------------------------------------------------------------------
#[cfg(test)]
mod dye_batch_recipe {
    use super::assert_routes_registered;

    #[test]
    fn test_dye_batch_recipe_routes_registered() {
        assert_routes_registered(
            "dye_batch_recipe",
            &[
                "/export",     // 同时存在于 dye-batches 与 dye-recipes
                "/:id/submit", // dye-recipes submit
            ],
        );
    }
}

// -----------------------------------------------------------------------------
// 7. 财务报表
// -----------------------------------------------------------------------------
#[cfg(test)]
mod finance_reports {
    use super::assert_routes_registered;

    #[test]
    fn test_finance_report_routes_registered() {
        assert_routes_registered(
            "finance_reports",
            &[
                "/reports/cash-flow",
                "/reports/trial-balance",
                "/reports/general-ledger/:code",
                "/reports/subsidiary-ledger",
            ],
        );
    }
}

// -----------------------------------------------------------------------------
// 8. 报表增强
// -----------------------------------------------------------------------------
#[cfg(test)]
mod reports_enhanced {
    use super::assert_routes_registered;

    #[test]
    fn test_report_enhanced_routes_registered() {
        assert_routes_registered(
            "reports_enhanced",
            &[
                "/fields/:template_type",
                "/templates/:id/export",
                "/templates/:id/preview",
                "/subscriptions/:id/send",
            ],
        );
    }
}

// -----------------------------------------------------------------------------
// 9. CRM 增强
// -----------------------------------------------------------------------------
#[cfg(test)]
mod crm_enhanced {
    use super::assert_routes_registered;

    #[test]
    fn test_crm_enhanced_routes_registered() {
        assert_routes_registered(
            "crm_enhanced",
            &[
                "/customers/:id/360",
                "/customers/enhanced/:id",
                "/pool/:customer_id/claim",
                "/pool/batch-claim",
                "/customers/:id/follow-ups",
                "/customers/:id/rfm",
                "/rfm/distribution",
            ],
        );
    }
}

// -----------------------------------------------------------------------------
// 10. AR 对账增强别名
// -----------------------------------------------------------------------------
#[cfg(test)]
mod ar_reconciliation_enhanced {
    use super::assert_routes_registered;

    #[test]
    fn test_ar_reconciliation_routes_registered() {
        assert_routes_registered(
            "ar_reconciliation_enhanced",
            &[
                "/auto-reconcile",
                "/auto-reconcile/results",
                "/aging-analysis",
                "/:id/details",
                "/:id/confirm/send",
                "/confirmations",
                "/confirmations/:id/status",
                "/disputes",
                "/disputes/:id",
                "/disputes/:id/resolve",
            ],
        );
    }
}

// -----------------------------------------------------------------------------
// 11. BPM 增强 + 凭证 PUT/DELETE + 销售价格 PUT/DELETE + 资金账户 PUT
// -----------------------------------------------------------------------------
#[cfg(test)]
mod bpm_voucher_salesprice_fund {
    use super::assert_routes_registered;

    #[test]
    fn test_bpm_routes_registered() {
        assert_routes_registered("bpm_enhanced", &["/versions/:version/activate"]);
    }

    #[test]
    fn test_voucher_routes_registered() {
        // voucher PUT/DELETE 在 gl_routes 中，需要 .put(...) 与 .delete(...)
        // 同时出现在 /vouchers/:id 块附近；为避免误判，我们用更具体的字符串
        assert_routes_registered(
            "voucher",
            &[
                "voucher_handler::update_voucher",
                "voucher_handler::delete_voucher",
            ],
        );
    }

    #[test]
    fn test_sales_price_routes_registered() {
        assert_routes_registered(
            "sales_price",
            &[
                "sales_price_handler::update_price",
                "sales_price_handler::delete_price",
            ],
        );
    }

    #[test]
    fn test_fund_account_routes_registered() {
        assert_routes_registered("fund_account", &["fund_management_handler::update_account"]);
    }
}

// -----------------------------------------------------------------------------
// 12. 库存调整 / 调拨 / 盘点
// -----------------------------------------------------------------------------
#[cfg(test)]
mod inventory {
    use super::assert_routes_registered;

    #[test]
    fn test_inventory_adjustment_routes_registered() {
        assert_routes_registered(
            "inventory_adjustment",
            &[
                "/adjustments/:id",
                "/adjustments/:id/items",
                "/adjustments/items/:item_id",
            ],
        );
    }

    #[test]
    fn test_inventory_transfer_routes_registered() {
        assert_routes_registered(
            "inventory_transfer",
            &[
                "/transfers/:id",
                "/transfers/:id/items",
                "/transfers/items/:item_id",
            ],
        );
    }

    #[test]
    fn test_inventory_count_routes_registered() {
        assert_routes_registered(
            "inventory_count",
            &["/counts/:id", "/counts/:id/items", "/counts/items/:item_id"],
        );
    }
}

// -----------------------------------------------------------------------------
// 13. 采购入库 DELETE + 扫码
// -----------------------------------------------------------------------------
#[cfg(test)]
mod purchase_receipt_scanner {
    use super::assert_routes_registered;

    #[test]
    fn test_purchase_receipt_delete_registered() {
        assert_routes_registered(
            "purchase_receipt",
            &["/receipts/:id", "purchase_receipt_handler::delete_receipt"],
        );
    }

    #[test]
    fn test_scanner_routes_registered() {
        assert_routes_registered("scanner", &["/scan-inventory", "/history", "/statistics"]);
    }
}
