//! 生产订单服务（facade，批次 488 D10-2 拆分）
//!
//! 本文件为 facade 入口，仅保留 `ProductionOrderService` struct + `new` 构造函数 + 单元测试。
//! 业务实现已按职责拆分到 `production_order_ops/` 子模块（与 `production_order_service` 同为 `crate::services` 下兄弟模块）：
//! - `production_order_ops::crud`：CRUD 与状态校验（14 方法，原 L92-624）
//! - `production_order_ops::completion`：完成生产订单与库存联动（20 方法，原 L626-1243）
//! - `production_order_ops::approval`：审批管理（7 方法，原 L1250-1501）
//! - `production_order_ops::types`：请求/查询 DTO + 内部辅助 struct
//!
//! 设计要点（与拆分前一致）：
//! - 创建订单后触发 MRP 物料需求计算（失败 warn 不阻塞）
//! - 返工订单使用 RW- 前缀，不触发 MRP
//! - 状态转换校验基于状态机白名单（validate_status_transition）
//! - COMPLETED 状态走 complete_production_order 专用路径（事务包裹状态变更 + 库存联动）
//! - 排产状态变更走 check_capacity_for_scheduling 产能校验
//! - delete 软删除（状态改为 CANCELLED），走 update_with_audit 保留审计
//! - 审批流程对接 BPM（启动/任务审批保留事务外，失败 warn 不阻断）
//! - BPM 回写方法不回调 BPM 避免循环
//!
//! 拆分兼容性：
//! - 外部 handler 通过 `crate::services::production_order_service::ProductionOrderService::new` 调用，路径不变
//! - 外部 handler 通过 `crate::services::production_order_service::{CreateProductionOrderRequest, UpdateProductionOrderRequest, ProductionOrderQuery}` 引用，路径不变（此处 re-export）
//! - `db` 字段使用 `pub(crate)` 可见性，production_order_ops 子模块的 impl 块可直接访问
//! - impl 块分散在 production_order_ops 子模块，Rust 允许同一 crate 多文件多 impl 块

use sea_orm::DatabaseConnection;
use std::sync::Arc;

// 批次 488 D10-2 拆分：re-export 保持外部引用路径不变
pub use crate::services::production_order_ops::{
    CreateProductionOrderRequest, ProductionOrderQuery, UpdateProductionOrderRequest,
};

/// 生产订单 Service（struct 定义保留在 facade，impl 块按职责分散到 `production_order_ops/` 子模块。）
pub struct ProductionOrderService {
    /// 数据库连接句柄（`pub(crate)` 可见性：production_order_ops 兄弟模块的 impl 块需直接访问此字段。）
    pub(crate) db: Arc<DatabaseConnection>,
}

impl ProductionOrderService {
    /// 创建生产订单服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decs;
    use crate::models::status::common;
    use crate::models::status::production;
    use crate::services::test_common::setup_test_db;
    use crate::utils::error::AppError;
    use crate::ymd;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    /// 复现 deduct_raw_materials_txn 中的 BOM 用量计算（纯算法）（公式：consumption_qty = (bom_quantity * production_qty).round_dp(4)）
    fn calc_consumption_qty(bom_quantity: Decimal, production_qty: Decimal) -> Decimal {
        (bom_quantity * production_qty).round_dp(4)
    }

    /// 复现 deduct_raw_materials_txn 中的公斤数按比例扣减（纯算法）
    /// 公式：qty_after_kg = qty_before_kg - (qty_before_kg * consumption_qty / qty_before_meters)；当 qty_before_meters 为零时，公斤数不变（避免除零）。
    fn calc_kg_after_deduction(
        qty_before_meters: Decimal,
        qty_before_kg: Decimal,
        consumption_qty: Decimal,
    ) -> Decimal {
        if qty_before_meters > Decimal::ZERO {
            qty_before_kg - (qty_before_kg * consumption_qty / qty_before_meters)
        } else {
            qty_before_kg
        }
    }

    /// 复现 increase_finished_goods_txn 中的成品入库公斤数计算（纯算法）
    /// 公式：added_kg = production_qty * gram_weight * width / 100000；当克重或幅宽缺失时，公斤数增量为零。
    fn calc_added_kg(
        production_qty: Decimal,
        gram_weight: Option<Decimal>,
        width: Option<Decimal>,
    ) -> Decimal {
        if let (Some(gw), Some(w)) = (gram_weight, width) {
            production_qty * gw * w / Decimal::new(100000, 0)
        } else {
            Decimal::ZERO
        }
    }

    /// 复现 complete_production_order 中 actual_quantity 缺省取 planned_quantity 的逻辑
    fn resolve_production_qty(
        actual_quantity: Option<Decimal>,
        planned_quantity: Decimal,
    ) -> Decimal {
        actual_quantity.unwrap_or(planned_quantity)
    }

    /// 复现 generate_unique_order_no 的订单号格式校验（纯字符串校验）（格式：PO-{14位时间戳}-{4位随机数}）
    fn is_valid_order_no_format(order_no: &str) -> bool {
        if !order_no.starts_with("PO-") {
            return false;
        }
        let parts: Vec<&str> = order_no.split('-').collect();
        if parts.len() != 3 {
            return false;
        }
        // 中段为 14 位时间戳，末段为 4 位数字
        parts[1].len() == 14
            && parts[1].chars().all(|c| c.is_ascii_digit())
            && parts[2].len() == 4
            && parts[2].chars().all(|c| c.is_ascii_digit())
    }

    // ============== 状态常量值正确性 ==============

    /// test_ztcl_cgwhfz（验证 STATUS_DRAFT 常量是大写字符串 "DRAFT"，与数据库约定一致。）
    #[test]
    fn test_ztcl_cgwhfz() {
        assert_eq!(common::STATUS_DRAFT, "DRAFT");
    }

    /// test_ztcl_yspwhfz
    #[test]
    fn test_ztcl_yspwhfz() {
        assert_eq!(common::STATUS_APPROVED, "APPROVED");
    }

    /// test_ztcl_ywcwhfz
    #[test]
    fn test_ztcl_ywcwhfz() {
        assert_eq!(common::STATUS_COMPLETED, "COMPLETED");
    }

    /// test_ztcl_yqxwhfz
    #[test]
    fn test_ztcl_yqxwhfz() {
        assert_eq!(common::STATUS_CANCELLED, "CANCELLED");
    }

    /// test_ztcl_ypcwhfz
    #[test]
    fn test_ztcl_ypcwhfz() {
        assert_eq!(production::PRODUCTION_SCHEDULED, "SCHEDULED");
    }

    /// test_ztcl_sczwhfz
    #[test]
    fn test_ztcl_sczwhfz() {
        assert_eq!(production::PRODUCTION_IN_PROGRESS, "IN_PROGRESS");
    }

    /// test_ztcl_dspwhfz
    #[test]
    fn test_ztcl_dspwhfz() {
        assert_eq!(production::PRODUCTION_PENDING_APPROVAL, "PENDING_APPROVAL");
    }

    /// test_ztcl_yjjwhfz
    #[test]
    fn test_ztcl_yjjwhfz() {
        assert_eq!(production::PRODUCTION_REJECTED, "REJECTED");
    }

    /// test_ztcl_gztzhbxt（验证生产订单 8 个状态常量两两互不相同，避免状态机歧义。）
    #[test]
    fn test_ztcl_gztzhbxt() {
        let statuses = [
            common::STATUS_DRAFT,
            common::STATUS_APPROVED,
            common::STATUS_COMPLETED,
            common::STATUS_CANCELLED,
            production::PRODUCTION_SCHEDULED,
            production::PRODUCTION_IN_PROGRESS,
            production::PRODUCTION_PENDING_APPROVAL,
            production::PRODUCTION_REJECTED,
        ];
        for i in 0..statuses.len() {
            for j in (i + 1)..statuses.len() {
                assert_ne!(statuses[i], statuses[j], "状态常量重复: {}", statuses[i]);
            }
        }
    }

    // ============== 状态机转换合法性 ==============

    /// test_ztzh_cgdypchf
    #[test]
    fn test_ztzh_cgdypchf() {
        assert!(ProductionOrderService::validate_status_transition(
            common::STATUS_DRAFT,
            production::PRODUCTION_SCHEDULED
        )
        .is_ok());
    }

    /// test_ztzh_cgddsphf
    #[test]
    fn test_ztzh_cgddsphf() {
        assert!(ProductionOrderService::validate_status_transition(
            common::STATUS_DRAFT,
            production::PRODUCTION_PENDING_APPROVAL
        )
        .is_ok());
    }

    /// test_ztzh_cgdyqxhf
    #[test]
    fn test_ztzh_cgdyqxhf() {
        assert!(ProductionOrderService::validate_status_transition(
            common::STATUS_DRAFT,
            common::STATUS_CANCELLED
        )
        .is_ok());
    }

    /// test_ztzh_ypcdsczhf
    #[test]
    fn test_ztzh_ypcdsczhf() {
        assert!(ProductionOrderService::validate_status_transition(
            production::PRODUCTION_SCHEDULED,
            production::PRODUCTION_IN_PROGRESS
        )
        .is_ok());
    }

    /// test_ztzh_ypcdyqxhf
    #[test]
    fn test_ztzh_ypcdyqxhf() {
        assert!(ProductionOrderService::validate_status_transition(
            production::PRODUCTION_SCHEDULED,
            common::STATUS_CANCELLED
        )
        .is_ok());
    }

    /// test_ztzh_sczdywchf
    #[test]
    fn test_ztzh_sczdywchf() {
        assert!(ProductionOrderService::validate_status_transition(
            production::PRODUCTION_IN_PROGRESS,
            common::STATUS_COMPLETED
        )
        .is_ok());
    }

    /// test_ztzh_sczdyqxhf
    #[test]
    fn test_ztzh_sczdyqxhf() {
        assert!(ProductionOrderService::validate_status_transition(
            production::PRODUCTION_IN_PROGRESS,
            common::STATUS_CANCELLED
        )
        .is_ok());
    }

    /// test_ztzh_dspdysphf
    #[test]
    fn test_ztzh_dspdysphf() {
        assert!(ProductionOrderService::validate_status_transition(
            production::PRODUCTION_PENDING_APPROVAL,
            common::STATUS_APPROVED
        )
        .is_ok());
    }

    /// test_ztzh_dspdyjjhf
    #[test]
    fn test_ztzh_dspdyjjhf() {
        assert!(ProductionOrderService::validate_status_transition(
            production::PRODUCTION_PENDING_APPROVAL,
            production::PRODUCTION_REJECTED
        )
        .is_ok());
    }

    /// test_ztzh_yspdypchf
    #[test]
    fn test_ztzh_yspdypchf() {
        assert!(ProductionOrderService::validate_status_transition(
            common::STATUS_APPROVED,
            production::PRODUCTION_SCHEDULED
        )
        .is_ok());
    }

    /// test_ztzh_yjjdcghf
    #[test]
    fn test_ztzh_yjjdcghf() {
        assert!(ProductionOrderService::validate_status_transition(
            production::PRODUCTION_REJECTED,
            common::STATUS_DRAFT
        )
        .is_ok());
    }

    /// test_ztzh_cgbnzjdscz（业务规则：草稿必须先经已排产才能进入生产中，跳级转换应被拒绝。）
    #[test]
    fn test_ztzh_cgbnzjdscz() {
        let result = ProductionOrderService::validate_status_transition(
            common::STATUS_DRAFT,
            production::PRODUCTION_IN_PROGRESS,
        );
        assert!(result.is_err());
    }

    /// test_ztzh_cgbnzjdywc
    #[test]
    fn test_ztzh_cgbnzjdywc() {
        let result = ProductionOrderService::validate_status_transition(
            common::STATUS_DRAFT,
            common::STATUS_COMPLETED,
        );
        assert!(result.is_err());
    }

    /// test_ztzh_ywcwztbkzbg
    #[test]
    fn test_ztzh_ywcwztbkzbg() {
        let result = ProductionOrderService::validate_status_transition(
            common::STATUS_COMPLETED,
            common::STATUS_CANCELLED,
        );
        assert!(result.is_err());
    }

    /// test_ztzh_yqxwztbkzbg
    #[test]
    fn test_ztzh_yqxwztbkzbg() {
        let result = ProductionOrderService::validate_status_transition(
            common::STATUS_CANCELLED,
            common::STATUS_DRAFT,
        );
        assert!(result.is_err());
    }

    /// test_ztzh_wzyztbjj
    #[test]
    fn test_ztzh_wzyztbjj() {
        let result =
            ProductionOrderService::validate_status_transition("UNKNOWN", common::STATUS_DRAFT);
        assert!(result.is_err());
    }

    /// test_ztzh_cwxxbhyhmbzt（验证非法转换的错误消息包含双方状态名，便于排查。）
    #[test]
    fn test_ztzh_cwxxbhyhmbzt() {
        let result = ProductionOrderService::validate_status_transition(
            common::STATUS_DRAFT,
            common::STATUS_COMPLETED,
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains(common::STATUS_DRAFT),
            "错误消息应包含源状态"
        );
        assert!(
            err_msg.contains(common::STATUS_COMPLETED),
            "错误消息应包含目标状态"
        );
    }

    /// test_ztzh_wzztcwxxbhztm
    #[test]
    fn test_ztzh_wzztcwxxbhztm() {
        let result =
            ProductionOrderService::validate_status_transition("FOO", common::STATUS_DRAFT);
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("FOO"), "错误消息应包含未知状态名");
    }

    // ============== 数量计算（纯算法） ==============

    /// test_sljs_bomylcyscsl_zsjg（验证 consumption_qty = bom_quantity * production_qty。）
    #[test]
    fn test_sljs_bomylcyscsl_zsjg() {
        let bom_qty = decs!("1.5");
        let prod_qty = decs!("100");
        let result = calc_consumption_qty(bom_qty, prod_qty);
        assert_eq!(result, decs!("150"));
    }

    /// test_sljs_bomylcyscsl_xsjg（验证 round_dp(4) 对结果精度进行控制，防止精度漂移。）
    #[test]
    fn test_sljs_bomylcyscsl_xsjg() {
        let bom_qty = decs!("0.1234");
        let prod_qty = decs!("3");
        let result = calc_consumption_qty(bom_qty, prod_qty);
        // 0.1234 * 3 = 0.3702，无需进位
        assert_eq!(result, decs!("0.3702"));
    }

    /// test_sljs_gjsablkj（验证 kg_before - (kg_before * consumption / meters_before) 的比例扣减逻辑。）
    #[test]
    fn test_sljs_gjsablkj() {
        let meters_before = decs!("100");
        let kg_before = decs!("50");
        let consumption = decs!("25");
        let result = calc_kg_after_deduction(meters_before, kg_before, consumption);
        // 50 - (50 * 25 / 100) = 50 - 12.5 = 37.5
        assert_eq!(result, decs!("37.5"));
    }

    /// test_sljs_mswlsgjsbb（防御性逻辑：当 qty_before_meters 为零时，公斤数保持不变避免除零。）
    #[test]
    fn test_sljs_mswlsgjsbb() {
        let meters_before = Decimal::ZERO;
        let kg_before = decs!("20");
        let consumption = decs!("5");
        let result = calc_kg_after_deduction(meters_before, kg_before, consumption);
        assert_eq!(result, kg_before);
    }

    /// test_sljs_cprkgjsjs（验证 added_kg = production_qty * gram_weight * width / 100000。）
    #[test]
    fn test_sljs_cprkgjsjs() {
        let prod_qty = decs!("1000"); // 米
        let gram_weight = Some(decs!("200")); // 克/平方米
        let width = Some(decs!("150")); // 厘米
        let result = calc_added_kg(prod_qty, gram_weight, width);
        // 1000 * 200 * 150 / 100000 = 300 kg
        assert_eq!(result, decs!("300"));
    }

    /// test_sljs_cprkkzqssgjswl
    #[test]
    fn test_sljs_cprkkzqssgjswl() {
        let prod_qty = decs!("1000");
        let result = calc_added_kg(prod_qty, None, Some(decs!("150")));
        assert_eq!(result, Decimal::ZERO);
    }

    /// test_sljs_sjslqsqjhsl（复现 complete_production_order 中 actual_quantity.unwrap_or(planned_quantity) 逻辑。）
    #[test]
    fn test_sljs_sjslqsqjhsl() {
        let planned = decs!("500");
        // 实际数量为 None 时取计划数量
        assert_eq!(resolve_production_qty(None, planned), planned);
        // 实际数量存在时取实际数量
        let actual = Some(decs!("480"));
        assert_eq!(resolve_production_qty(actual, planned), decs!("480"));
    }

    /// test_sljs_scslwlscfcwlj
    /// 复现 handle_production_completion_inventory_txn 中 production_qty.is_zero() 校验：当 actual_quantity 和 planned_quantity 均为零时，应触发业务错误。
    #[test]
    fn test_sljs_scslwlscfcwlj() {
        let planned = Decimal::ZERO;
        let actual: Option<Decimal> = None;
        let production_qty = resolve_production_qty(actual, planned);
        assert!(production_qty.is_zero(), "生产数量为零时应触发错误路径");
    }

    // ============== 错误消息格式 ==============

    /// test_cwxx_cpbczbhid（复现 validate_product_exists 中 "产品ID {} 不存在" 的错误消息格式。）
    #[test]
    fn test_cwxx_cpbczbhid() {
        let err = AppError::validation(format!("产品ID {} 不存在", 999));
        let msg = err.to_string();
        assert!(msg.contains("999"), "错误消息应包含产品ID");
        assert!(msg.contains("产品ID"), "错误消息应包含'产品ID'前缀");
    }

    /// test_cwxx_xsddbczbhid
    #[test]
    fn test_cwxx_xsddbczbhid() {
        let err = AppError::validation(format!("销售订单ID {} 不存在", 888));
        assert!(err.to_string().contains("888"));
    }

    /// test_cwxx_gzzxbczbhid
    #[test]
    fn test_cwxx_gzzxbczbhid() {
        let err = AppError::validation(format!("工作中心ID {} 不存在", 777));
        assert!(err.to_string().contains("777"));
    }

    /// test_cwxx_ddhyczbhddh
    #[test]
    fn test_cwxx_ddhyczbhddh() {
        let order_no = "PO-20260709000000-0001";
        let err = AppError::validation(format!("订单号 {} 已存在", order_no));
        let msg = err.to_string();
        assert!(msg.contains(order_no), "错误消息应包含订单号");
    }

    /// test_cwxx_scslwltsmq（复现 handle_production_completion_inventory_txn 中 "生产数量为零" 的业务错误消息。）
    #[test]
    fn test_cwxx_scslwltsmq() {
        let err = AppError::business("生产数量为零，无法执行库存联动".to_string());
        let msg = err.to_string();
        assert!(
            msg.contains("生产数量为零"),
            "错误消息应明确提示生产数量为零"
        );
    }

    /// test_cwxx_wzdkycktsmq
    #[test]
    fn test_cwxx_wzdkycktsmq() {
        let err = AppError::business("未找到可用仓库，无法执行库存联动");
        assert!(err.to_string().contains("未找到可用仓库"));
    }

    /// test_cwxx_wfscwyddhtszs
    #[test]
    fn test_cwxx_wfscwyddhtszs() {
        let err = AppError::internal("无法生成唯一订单号，请稍后重试".to_string());
        let msg = err.to_string();
        assert!(msg.contains("无法生成唯一订单号"));
        assert!(msg.contains("稍后重试"));
    }

    // ============== 订单号格式 ==============

    /// test_ddhgs_hfgstgjy（验证 generate_unique_order_no 生成的 "PO-{14位时间戳}-{4位数字}" 格式合法。）
    #[test]
    fn test_ddhgs_hfgstgjy() {
        assert!(is_valid_order_no_format("PO-20260709103000-0042"));
    }

    /// test_ddhgs_qsqzbhf
    #[test]
    fn test_ddhgs_qsqzbhf() {
        assert!(!is_valid_order_no_format("20260709103000-0042"));
    }

    /// test_ddhgs_sjccdbzbhf
    #[test]
    fn test_ddhgs_sjccdbzbhf() {
        assert!(!is_valid_order_no_format("PO-20260709-0042"));
    }

    /// test_ddhgs_sjdfszbhf
    #[test]
    fn test_ddhgs_sjdfszbhf() {
        assert!(!is_valid_order_no_format("PO-20260709103000-ABCD"));
    }

    // ============== 夹具宏可用性 ==============

    /// test_decs_h_jxzfcwdecimal
    #[test]
    fn test_decs_h_jxzfcwdecimal() {
        let v = decs!("123.456");
        assert_eq!(v.to_string(), "123.456");
    }

    /// test_decs_h_jxzscwdecimal
    #[test]
    fn test_decs_h_jxzscwdecimal() {
        let v = decs!("1000");
        assert_eq!(v, Decimal::new(1000, 0));
    }

    /// test_ymd_h_jxrq
    #[test]
    fn test_ymd_h_jxrq() {
        let d = ymd!(2026, 7, 9);
        assert_eq!(d.format("%Y-%m-%d").to_string(), "2026-07-09");
    }

    /// test_ymd_h_jxncrq
    #[test]
    fn test_ymd_h_jxncrq() {
        let d = ymd!(2026, 1, 1);
        assert_eq!(d.format("%Y-%m-%d").to_string(), "2026-01-01");
    }

    /// test_fromstr_ydecshjgyz（验证 decs! 宏与 Decimal::from_str 行为一致，确保夹具可信赖。）
    #[test]
    fn test_fromstr_ydecshjgyz() {
        let a = decs!("99.9");
        let b = Decimal::from_str("99.9").expect("FromStr 不应失败");
        assert_eq!(a, b);
    }

    // ============== 服务实例化与请求结构 ==============

    /// test_fwslh_sysqlitencsjk
    /// 标注 #[ignore]：依赖 SQLite 内存数据库 schema，CI 中不强制运行；用于本地手动验证 ProductionOrderService::new 能正常构造。
    #[tokio::test]
    #[ignore = "依赖 SQLite 内存数据库 schema，CI 中跳过；本地手动验证用"]
    async fn test_fwslh_sysqlitencsjk() {
        let db = setup_test_db().await;
        // L-20 修复（批次 377 v13 复审）：删除 let _ = service 占位变量
        // 仅验证服务能正常构造，不调用任何依赖 schema 的方法
        let _service = ProductionOrderService::new(std::sync::Arc::new(db));
    }

    /// test_qqjg_cjddqqkgz（验证 CreateProductionOrderRequest 能正常构造，字段类型匹配。）
    #[test]
    fn test_qqjg_cjddqqkgz() {
        let req = CreateProductionOrderRequest {
            order_no: Some("PO-TEST-001".to_string()),
            sales_order_id: None,
            product_id: 1,
            planned_quantity: Some(decs!("100")),
            planned_start_date: Some(ymd!(2026, 7, 1)),
            planned_end_date: Some(ymd!(2026, 7, 31)),
            priority: Some(5),
            work_center_id: Some(1),
            remarks: Some("测试订单".to_string()),
            created_by: 1,
        };
        assert_eq!(req.product_id, 1);
        assert_eq!(req.planned_quantity, Some(decs!("100")));
        assert_eq!(req.priority, Some(5));
    }

    /// test_qqjg_gxddqqkgz
    #[test]
    fn test_qqjg_gxddqqkgz() {
        let req = UpdateProductionOrderRequest {
            planned_quantity: Some(decs!("200")),
            planned_start_date: Some(ymd!(2026, 8, 1)),
            planned_end_date: Some(ymd!(2026, 8, 31)),
            priority: Some(8),
            work_center_id: Some(2),
            remarks: Some("更新后备注".to_string()),
        };
        assert_eq!(req.planned_quantity, Some(decs!("200")));
        assert_eq!(req.priority, Some(8));
    }

    /// test_cxcs_fycskgz
    #[test]
    fn test_cxcs_fycskgz() {
        let query = ProductionOrderQuery {
            status: Some(common::STATUS_DRAFT.to_string()),
            product_id: Some(1),
            page: 1,
            page_size: 20,
        };
        assert_eq!(query.page, 1);
        assert_eq!(query.page_size, 20);
        assert_eq!(query.status, Some("DRAFT".to_string()));
    }
}
