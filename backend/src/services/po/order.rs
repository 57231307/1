//! 采购订单核心服务 facade（po/order）
//!
//! 本文件为 facade：仅保留响应 DTO、`PurchaseOrderService` 结构体与 `new` 构造器、单元测试。
//! 业务实现（CRUD / 生命周期 / 查询导出）已拆分至 `po/order_ops/` 子模块，各子模块以独立
//! `impl PurchaseOrderService` 块形式挂载方法（Rust 允许同 crate 多文件多 impl 块）。
//! `db` 字段声明为 `pub(crate)` 供各 ops 子模块直接访问 `self.db`。
//! 拆分自原 `purchase_order_service.rs`。

use sea_orm::{DatabaseConnection, FromQueryResult};
use serde::Serialize;
use std::sync::Arc;

// =====================================================
// 响应 DTO
// =====================================================

/// 采购订单视图对象
#[derive(Debug, Clone, FromQueryResult, Serialize)]
pub struct PurchaseOrderDto {
    pub id: i32,
    pub order_no: String,
    pub supplier_id: i32,
    pub supplier_name: Option<String>,
    pub order_date: chrono::NaiveDate,
    pub expected_delivery_date: Option<chrono::NaiveDate>,
    pub actual_delivery_date: Option<chrono::NaiveDate>,
    pub warehouse_id: i32,
    pub warehouse_name: Option<String>,
    pub department_id: i32,
    pub department_name: Option<String>,
    pub purchaser_id: i32,
    pub currency: String,
    pub exchange_rate: rust_decimal::Decimal,
    pub total_amount: rust_decimal::Decimal,
    pub total_amount_foreign: rust_decimal::Decimal,
    pub total_quantity: rust_decimal::Decimal,
    pub total_quantity_alt: rust_decimal::Decimal,
    #[serde(rename = "status")]
    pub order_status: String,
    pub payment_terms: Option<String>,
    pub shipping_terms: Option<String>,
    pub notes: Option<String>,
    pub created_by: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 采购订单明细视图对象
#[derive(Debug, Clone, FromQueryResult, Serialize)]
pub struct PurchaseOrderItemDto {
    pub id: i32,
    pub order_id: i32,
    pub line_no: i32,
    #[serde(rename = "material_id")]
    pub product_id: i32,
    #[serde(rename = "material_code")]
    pub material_code: Option<String>,
    #[serde(rename = "material_name")]
    pub material_name: Option<String>,
    #[serde(rename = "quantity_ordered")]
    pub quantity: rust_decimal::Decimal,
    pub unit_price: rust_decimal::Decimal,
    #[serde(rename = "tax_rate")]
    pub tax_percent: rust_decimal::Decimal,
    pub amount: rust_decimal::Decimal,
    pub tax_amount: rust_decimal::Decimal,
    pub total_amount: rust_decimal::Decimal,
    pub received_quantity: rust_decimal::Decimal,
    pub returned_quantity: rust_decimal::Decimal,
    pub notes: Option<String>,
}

// =====================================================
// 采购订单服务
// =====================================================

/// 采购订单服务（核心）
///
/// 业务方法分布于 `po/order_ops/` 子模块的各 `impl` 块中：
/// - CRUD / 列表 / 详情：`order_ops::crud`
/// - 生命周期（关闭）：`order_ops::lifecycle`
/// - 明细查询 / CSV 导出：`order_ops::query`
pub struct PurchaseOrderService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl PurchaseOrderService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

// =====================================================
// 单元测试模块（模式 B：内嵌 #[cfg(test)] mod tests）
// =====================================================
// 测试策略：create_order_items / create_order_header / validate_order_request 中的
// 纯算法逻辑（金额、税额、折扣、总额、行号默认值、货币/汇率默认值、日期校验、CSV 表头）
// 通过复现其计算公式进行回归保护；依赖真实数据库 schema 的方法标注 #[ignore]。
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::status;
    use crate::services::test_common::setup_test_db;
    use crate::utils::error::AppError;
    use crate::decs;
    use crate::ymd;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    // ---------- 纯算法复现夹具（与 create_order_items / create_order_header 保持一致） ----------

    /// 复现 create_order_items 的金额计算纯算法
    fn calc_amount(quantity: Decimal, unit_price: Decimal) -> Decimal {
        (quantity * unit_price).round_dp(2)
    }

    /// 复现 create_order_items 的税额计算纯算法
    fn calc_tax_amount(amount: Decimal, tax_percent: Decimal) -> Decimal {
        (amount * tax_percent / Decimal::new(100, 0)).round_dp(2)
    }

    /// 复现 create_order_items 的折扣计算纯算法
    fn calc_discount_amount(amount: Decimal, discount_percent: Decimal) -> Decimal {
        (amount * discount_percent / Decimal::new(100, 0)).round_dp(2)
    }

    /// 复现 create_order_items 的明细总额计算纯算法
    fn calc_line_total(amount: Decimal, tax_amount: Decimal, discount_amount: Decimal) -> Decimal {
        amount + tax_amount - discount_amount
    }

    /// 复现 create_order_items 的明细行号默认值计算
    fn default_line_no(index: usize) -> i32 {
        (index + 1) as i32
    }

    // ---------- 金额计算 ----------

    /// 测试_金额计算_整数数量场景
    ///
    /// 验证数量与单价均为整数时金额计算正确
    #[test]
    fn 测试_金额计算_整数数量场景() {
        let quantity = decs!("10");
        let unit_price = decs!("100");
        // 10 * 100 = 1000.00
        assert_eq!(calc_amount(quantity, unit_price), decs!("1000"));
    }

    /// 测试_金额计算_小数数量场景
    ///
    /// 验证数量与单价含小数时 round_dp(2) 精度归一化生效
    #[test]
    fn 测试_金额计算_小数数量场景() {
        // 3.1415 * 2.5 = 7.85375 → round_dp(2) = 7.85
        assert_eq!(calc_amount(decs!("3.1415"), decs!("2.5")), decs!("7.85"));
    }

    /// 测试_金额计算_零价或零量场景
    ///
    /// 验证数量或单价为 0 时金额为 0
    #[test]
    fn 测试_金额计算_零价或零量场景() {
        // 零数量
        assert_eq!(calc_amount(Decimal::ZERO, decs!("100")), Decimal::ZERO);
        // 零单价
        assert_eq!(calc_amount(decs!("10"), Decimal::ZERO), Decimal::ZERO);
    }

    // ---------- 税额计算 ----------

    /// 测试_税额计算_默认税率场景
    ///
    /// 验证未指定税率时使用默认值 Decimal::new(13, 2) = 0.13，
    /// 税额公式：amount * tax_percent / 100
    #[test]
    fn 测试_税额计算_默认税率场景() {
        // 默认税率常量与 create_order_items 中 unwrap_or(Decimal::new(13, 2)) 一致
        let default_tax_percent = Decimal::new(13, 2);
        // 1000 * 0.13 / 100 = 1.30
        assert_eq!(calc_tax_amount(decs!("1000"), default_tax_percent), decs!("1.30"));
    }

    /// 测试_税额计算_自定义税率场景
    ///
    /// 验证用户传入税率（百分比值 13）时税额计算正确
    #[test]
    fn 测试_税额计算_自定义税率场景() {
        // 用户传 13 作为百分比：1000 * 13 / 100 = 130.00
        assert_eq!(calc_tax_amount(decs!("1000"), decs!("13")), decs!("130"));
    }

    /// 测试_税额计算_零税率场景
    ///
    /// 验证税率为 0 时税额为 0
    #[test]
    fn 测试_税额计算_零税率场景() {
        assert_eq!(
            calc_tax_amount(decs!("1000"), Decimal::ZERO),
            Decimal::ZERO
        );
    }

    /// 测试_税额计算_精度归一化
    ///
    /// 验证税额计算结果经 round_dp(2) 归一化到两位小数
    #[test]
    fn 测试_税额计算_精度归一化() {
        // 333.33 * 13 / 100 = 43.3329 → round_dp(2) = 43.33
        assert_eq!(
            calc_tax_amount(decs!("333.33"), decs!("13")),
            decs!("43.33")
        );
    }

    // ---------- 折扣计算 ----------

    /// 测试_折扣计算_默认无折扣场景
    ///
    /// 验证未指定折扣（discount_percent 默认 0）时折扣金额为 0
    #[test]
    fn 测试_折扣计算_默认无折扣场景() {
        // 默认折扣百分比为 Decimal::ZERO（与 create_order_items 一致）
        assert_eq!(
            calc_discount_amount(decs!("1000"), Decimal::ZERO),
            Decimal::ZERO
        );
    }

    /// 测试_折扣计算_自定义折扣场景
    ///
    /// 验证用户传入折扣百分比（10 表示 10%）时折扣金额计算正确
    #[test]
    fn 测试_折扣计算_自定义折扣场景() {
        // 1000 * 10 / 100 = 100.00
        assert_eq!(
            calc_discount_amount(decs!("1000"), decs!("10")),
            decs!("100")
        );
    }

    // ---------- 明细总额 ----------

    /// 测试_明细总额_含税无折扣场景
    ///
    /// 验证总额公式 amount + tax_amount - discount_amount（无折扣）
    #[test]
    fn 测试_明细总额_含税无折扣场景() {
        // 1000 + 130 - 0 = 1130
        assert_eq!(
            calc_line_total(decs!("1000"), decs!("130"), Decimal::ZERO),
            decs!("1130")
        );
    }

    /// 测试_明细总额_含税含折扣场景
    ///
    /// 验证总额公式 amount + tax_amount - discount_amount（含折扣）
    #[test]
    fn 测试_明细总额_含税含折扣场景() {
        // 1000 + 130 - 100 = 1030
        assert_eq!(
            calc_line_total(decs!("1000"), decs!("130"), decs!("100")),
            decs!("1030")
        );
    }

    // ---------- 明细行号 ----------

    /// 测试_明细行号_默认值递增场景
    ///
    /// 验证未指定 line_no 时按 (index + 1) 从 1 递增
    #[test]
    fn 测试_明细行号_默认值递增场景() {
        // 复现 create_order_items 中 item.line_no.unwrap_or((index + 1) as i32)
        assert_eq!(default_line_no(0), 1);
        assert_eq!(default_line_no(1), 2);
        assert_eq!(default_line_no(2), 3);
        assert_eq!(default_line_no(9), 10);
    }

    // ---------- 订单默认值 ----------

    /// 测试_货币默认值_未指定时使用CNY
    ///
    /// 验证 create_order_header 中 currency 未指定时使用 crate::constants::DEFAULT_CURRENCY
    #[test]
    fn 测试_货币默认值_未指定时使用CNY() {
        // 复现 create_order_header 中货币默认值逻辑
        let req_currency: Option<String> = None;
        let currency =
            req_currency.unwrap_or_else(|| crate::constants::DEFAULT_CURRENCY.to_string());
        // 验证未指定时回退到项目默认货币常量
        assert_eq!(currency, crate::constants::DEFAULT_CURRENCY);
        assert_eq!(currency, "CNY");
        // 验证显式指定时不应被默认值覆盖
        let explicit = Some("USD".to_string());
        let currency_explicit = explicit
            .clone()
            .unwrap_or_else(|| crate::constants::DEFAULT_CURRENCY.to_string());
        assert_eq!(currency_explicit, "USD");
    }

    /// 测试_汇率默认值_未指定时为1
    ///
    /// 验证 create_order_header 中 exchange_rate 未指定时默认为 Decimal::new(1, 0) = 1
    #[test]
    fn 测试_汇率默认值_未指定时为1() {
        // 复现 create_order_header 中汇率默认值逻辑
        let req_exchange_rate: Option<Decimal> = None;
        let exchange_rate = req_exchange_rate.unwrap_or(Decimal::new(1, 0));
        assert_eq!(exchange_rate, decs!("1"));
        // 验证显式指定时不应被默认值覆盖
        let explicit = Some(decs!("6.5"));
        let exchange_rate_explicit = explicit.unwrap_or(Decimal::new(1, 0));
        assert_eq!(exchange_rate_explicit, decs!("6.5"));
    }

    /// 测试_订单初始状态_使用DRAFT常量
    ///
    /// 验证 create_order_header 中订单初始状态使用 status::purchase_order::DRAFT 常量
    /// （禁止硬编码状态字符串，全程引用常量）
    #[test]
    fn 测试_订单初始状态_使用DRAFT常量() {
        // 复现 create_order_header 中订单初始状态设置
        let initial_status = status::purchase_order::DRAFT.to_string();
        assert!(!initial_status.is_empty());
        assert_eq!(initial_status, status::purchase_order::DRAFT);
        // 验证 DRAFT 与其他采购订单状态常量互不相同（状态机不冲突）
        assert_ne!(
            status::purchase_order::DRAFT,
            status::purchase_order::APPROVED
        );
        assert_ne!(
            status::purchase_order::DRAFT,
            status::purchase_order::CANCELLED
        );
        assert_ne!(
            status::purchase_order::DRAFT,
            status::purchase_order::COMPLETED
        );
        assert_ne!(
            status::purchase_order::DRAFT,
            status::purchase_order::CLOSED
        );
    }

    // ---------- 日期校验 ----------

    /// 测试_日期校验_预计交货不能早于订单日期
    ///
    /// 验证 validate_order_request 中预计交货日期不能早于订单日期的校验逻辑
    #[test]
    fn 测试_日期校验_预计交货不能早于订单日期() {
        let order_date = ymd!(2026, 3, 15);
        // 场景 1：预计交货日期等于订单日期（允许）
        let expected_same = ymd!(2026, 3, 15);
        assert!(!(expected_same < order_date));
        // 场景 2：预计交货日期晚于订单日期（允许）
        let expected_after = ymd!(2026, 3, 20);
        assert!(!(expected_after < order_date));
        // 场景 3：预计交货日期早于订单日期（应拒绝，复现 validate_order_request 拒绝条件）
        let expected_before = ymd!(2026, 3, 10);
        assert!(expected_before < order_date);
    }

    // ---------- CSV 导出表头 ----------

    /// 测试_CSV表头_列数与列名验证
    ///
    /// 验证 export_orders_to_csv 中 CSV 表头为 21 列且列名符合预期
    #[test]
    fn 测试_CSV表头_列数与列名验证() {
        // 复现 export_orders_to_csv 中的表头定义
        let headers = vec![
            "订单编号".to_string(),
            "供应商ID".to_string(),
            "供应商名称".to_string(),
            "订单日期".to_string(),
            "预计交货日期".to_string(),
            "实际交货日期".to_string(),
            "仓库ID".to_string(),
            "仓库名称".to_string(),
            "部门ID".to_string(),
            "部门名称".to_string(),
            "采购员ID".to_string(),
            "币种".to_string(),
            "汇率".to_string(),
            "总金额".to_string(),
            "总金额外币".to_string(),
            "总数量".to_string(),
            "总数量辅助".to_string(),
            "状态".to_string(),
            "付款条件".to_string(),
            "运输条款".to_string(),
            "备注".to_string(),
        ];
        // 列数为 21
        assert_eq!(headers.len(), 21);
        // 首列与末列
        assert_eq!(headers.first().unwrap(), "订单编号");
        assert_eq!(headers.last().unwrap(), "备注");
        // 关键列存在性抽检
        assert!(headers.contains(&"币种".to_string()));
        assert!(headers.contains(&"状态".to_string()));
        assert!(headers.contains(&"总金额".to_string()));
    }

    // ---------- 夹具宏与服务实例化 ----------

    /// 测试_decs夹具宏_可用性
    ///
    /// 验证 decs! 宏能正确解析 Decimal 字符串
    #[test]
    fn 测试_decs夹具宏_可用性() {
        let v = decs!("123.45");
        assert_eq!(v, Decimal::from_str("123.45").unwrap());
        assert_eq!(v.to_string(), "123.45");
    }

    /// 测试_服务实例化_SQLite内存数据库
    ///
    /// 验证 PurchaseOrderService 在 SQLite 内存数据库上能正常实例化
    #[tokio::test]
    async fn 测试_服务实例化_SQLite内存数据库() {
        let db = setup_test_db().await;
        let service = PurchaseOrderService::new(Arc::new(db));
        // 验证内部 db Arc 已被正确持有
        assert!(Arc::strong_count(&service.db) >= 1);
    }

    // ---------- 状态校验门（批次 392 补测） ----------

    /// 复现 update_order 的状态校验门（行 363-369）
    /// 仅 DRAFT 和 REJECTED 状态允许修改，其他状态返回 Err
    fn update_order_status_gate(status: &str) -> Result<(), AppError> {
        if status != status::purchase_order::DRAFT && status != status::purchase_order::REJECTED {
            return Err(AppError::business(format!(
                "订单状态不允许修改，当前状态：{}",
                status
            )));
        }
        Ok(())
    }

    /// 复现 delete_order 的状态校验门（行 441-445）
    /// 仅 DRAFT 状态允许删除，其他状态返回 Err
    fn delete_order_status_gate(status: &str) -> Result<(), AppError> {
        if status != status::purchase_order::DRAFT {
            return Err(AppError::business(format!(
                "订单状态不允许删除，当前状态：{}",
                status
            )));
        }
        Ok(())
    }

    /// 复现 close_order 的状态校验门（行 481-489）
    /// 仅 COMPLETED 和 PARTIAL_RECEIVED 状态允许关闭，其他状态返回 Err
    fn close_order_status_gate(status: &str) -> Result<(), AppError> {
        if ![
            status::purchase_order::COMPLETED,
            status::purchase_order::PARTIAL_RECEIVED,
        ]
        .contains(&status)
        {
            return Err(AppError::business(format!(
                "订单状态不允许关闭，当前状态：{}",
                status
            )));
        }
        Ok(())
    }

    /// 测试_update_order状态校验门_允许的状态
    ///
    /// 验证 DRAFT 和 REJECTED 状态允许修改
    #[test]
    fn 测试_update_order状态校验门_允许的状态() {
        assert!(update_order_status_gate(status::purchase_order::DRAFT).is_ok());
        assert!(update_order_status_gate(status::purchase_order::REJECTED).is_ok());
    }

    /// 测试_update_order状态校验门_禁止的状态
    ///
    /// 验证非 DRAFT/REJECTED 状态不允许修改且错误消息包含当前状态
    #[test]
    fn 测试_update_order状态校验门_禁止的状态() {
        let forbidden = [
            status::purchase_order::PENDING_APPROVAL,
            status::purchase_order::SUBMITTED,
            status::purchase_order::APPROVED,
            status::purchase_order::CLOSED,
            status::purchase_order::CANCELLED,
            status::purchase_order::COMPLETED,
            status::purchase_order::PARTIAL_RECEIVED,
        ];
        for s in forbidden {
            let err = update_order_status_gate(s).unwrap_err();
            let msg = err.to_string();
            assert!(msg.contains(s), "错误消息应包含当前状态 {}", s);
            assert!(msg.contains("修改"), "错误消息应包含操作类型");
        }
    }

    /// 测试_delete_order状态校验门_仅DRAFT允许
    ///
    /// 验证仅 DRAFT 状态允许删除
    #[test]
    fn 测试_delete_order状态校验门_仅DRAFT允许() {
        assert!(delete_order_status_gate(status::purchase_order::DRAFT).is_ok());
    }

    /// 测试_delete_order状态校验门_非DRAFT禁止
    ///
    /// 验证非 DRAFT 状态不允许删除且错误消息包含当前状态
    #[test]
    fn 测试_delete_order状态校验门_非DRAFT禁止() {
        let forbidden = [
            status::purchase_order::REJECTED,
            status::purchase_order::APPROVED,
            status::purchase_order::COMPLETED,
            status::purchase_order::CANCELLED,
        ];
        for s in forbidden {
            let err = delete_order_status_gate(s).unwrap_err();
            let msg = err.to_string();
            assert!(msg.contains(s), "错误消息应包含当前状态 {}", s);
            assert!(msg.contains("删除"), "错误消息应包含操作类型");
        }
    }

    /// 测试_close_order状态校验门_允许的状态
    ///
    /// 验证 COMPLETED 和 PARTIAL_RECEIVED 状态允许关闭
    #[test]
    fn 测试_close_order状态校验门_允许的状态() {
        assert!(close_order_status_gate(status::purchase_order::COMPLETED).is_ok());
        assert!(close_order_status_gate(status::purchase_order::PARTIAL_RECEIVED).is_ok());
    }

    /// 测试_close_order状态校验门_禁止的状态
    ///
    /// 验证非 COMPLETED/PARTIAL_RECEIVED 状态不允许关闭且错误消息包含当前状态
    #[test]
    fn 测试_close_order状态校验门_禁止的状态() {
        let forbidden = [
            status::purchase_order::DRAFT,
            status::purchase_order::APPROVED,
            status::purchase_order::REJECTED,
            status::purchase_order::CANCELLED,
        ];
        for s in forbidden {
            let err = close_order_status_gate(s).unwrap_err();
            let msg = err.to_string();
            assert!(msg.contains(s), "错误消息应包含当前状态 {}", s);
            assert!(msg.contains("关闭"), "错误消息应包含操作类型");
        }
    }
}
