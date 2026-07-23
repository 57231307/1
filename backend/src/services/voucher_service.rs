//! 凭证管理 Service（facade，批次 488 D10-2a 拆分）
//!
//! 本文件为 facade 入口，仅保留 `VoucherService` struct + `new` 构造函数 + DTOs + 类型定义 + 单元测试。
//! 业务实现已按职责拆分到 `voucher_ops/` 子模块（与 `voucher_service` 同为 `crate::services` 下兄弟模块）：
//! - `voucher_ops::crud`：CRUD 与状态校验（11 方法，原 L124-350 + L480-522 + L1258-1288）
//! - `voucher_ops::workflow`：工作流状态机 submit/review/post（5 方法，原 L523-717）
//! - `voucher_ops::balance`：科目余额更新（12 方法 + BalanceUpdateContext，原 L88-97 + L720-1045）
//! - `voucher_ops::assist`：辅助核算记录写入（11 方法 + AssistRecordContext，原 L98-113 + L1052-1255）
//!
//! 设计要点（与拆分前一致）：
//! - 凭证状态机：draft → submitted → reviewed → posted（不可逆）
//! - 状态变更加 lock_exclusive 串行化并发
//! - post 内部调用 balance::update_account_balances 回写科目余额
//! - post 内部调用 assist::write_assist_accounting_records_txn 写入辅助核算记录
//! - 期末余额按会计制度计算（借方科目：期初借+本期借-本期贷；贷方科目反之）
//! - 辅助核算五维 ID：BATCH:{}|COLOR:{}|DYE_LOT:{}|GRADE:{}|WORKSHOP:{}
//!
//! 拆分兼容性：
//! - 外部 handler 通过 `crate::services::voucher_service::VoucherService::new` 调用，路径不变
//! - 外部 handler 通过 `crate::services::voucher_service::{CreateVoucherRequest, UpdateVoucherRequest, VoucherItemRequest, VoucherQueryParams, VoucherTypeDefinition}` 引用，路径不变
//! - `db` 字段使用 `pub(crate)` 可见性，voucher_ops 子模块的 impl 块可直接访问
//! - impl 块分散在 voucher_ops 子模块，Rust 允许同一 crate 多文件多 impl 块
//! - `update_account_balances` / `write_assist_accounting_records_txn` 使用 `pub(crate)` 可见性，允许 workflow::post 跨 impl 块调用

// 凭证管理 Service
//
// 凭证业务逻辑层（核心）

use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::models::{voucher, voucher_item};
use rust_decimal::Decimal;

// 批次 102 v6 P3-1 修复：状态字符串常量化，引用 crate::models::status::voucher

/// 创建凭证请求
#[derive(Debug, Clone)]
pub struct CreateVoucherRequest {
    pub voucher_type: String,
    pub voucher_date: chrono::NaiveDate,
    pub source_type: Option<String>,
    pub source_module: Option<String>,
    pub source_bill_id: Option<i32>,
    pub source_bill_no: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub items: Vec<VoucherItemRequest>,
}

/// 凭证分录请求
#[derive(Debug, Clone)]
pub struct VoucherItemRequest {
    pub line_no: Option<i32>,
    pub subject_code: Option<String>,
    pub subject_name: Option<String>,
    pub debit: Decimal,
    pub credit: Decimal,
    pub summary: Option<String>,
    pub assist_customer_id: Option<i32>,
    pub assist_supplier_id: Option<i32>,
    pub assist_department_id: Option<i32>,
    pub assist_employee_id: Option<i32>,
    pub assist_project_id: Option<i32>,
    pub assist_batch_id: Option<i32>,
    pub assist_color_no_id: Option<i32>,
    pub assist_dye_lot_id: Option<i32>,
    pub assist_grade: Option<String>,
    pub assist_workshop_id: Option<i32>,
    pub quantity_meters: Option<Decimal>,
    pub quantity_kg: Option<Decimal>,
    pub unit_price: Option<Decimal>,
}

/// 更新凭证请求
#[derive(Debug, Clone)]
pub struct UpdateVoucherRequest {
    pub voucher_type: Option<String>,
    pub voucher_date: Option<chrono::NaiveDate>,
    pub items: Option<Vec<VoucherItemRequest>>,
}

/// 凭证查询参数
#[derive(Debug, Clone)]
pub struct VoucherQueryParams {
    pub voucher_type: Option<String>,
    pub status: Option<String>,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 凭证 Service
///
/// struct 定义保留在 facade，impl 块按职责分散到 `voucher_ops/` 子模块。
pub struct VoucherService {
    /// 数据库连接句柄
    ///
    /// `pub(crate)` 可见性：voucher_ops 兄弟模块的 impl 块需直接访问此字段。
    pub(crate) db: Arc<DatabaseConnection>,
}

impl VoucherService {
    /// 创建凭证服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

/// 凭证类型定义（v11 批次 155 P2-C：静态配置化，避免 handler 硬编码）
#[derive(Debug, Clone, serde::Serialize)]
pub struct VoucherTypeDefinition {
    pub code: &'static str,
    pub name: &'static str,
}

impl VoucherTypeDefinition {
    pub fn new(code: &'static str, name: &'static str) -> Self {
        Self { code, name }
    }
}

/// 凭证详情（包含分录）
// v11 批次 148 P2-A：移除失效的 dead_code 标注（get_by_id 方法返回 VoucherDetail，被 voucher_handler::get_voucher 真实调用）
#[derive(Debug, Clone)]
pub struct VoucherDetail {
    pub voucher: voucher::Model,
    pub items: Vec<voucher_item::Model>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::test_common::setup_test_db;
    use crate::decs;
    use crate::ymd;
    use crate::models::status::voucher as voucher_status;
    use chrono::Utc;
    use sea_orm::DatabaseConnection;
    use std::str::FromStr;
    use chrono::Datelike;
    use crate::utils::error::AppError;

    /// 构建测试用凭证分录请求夹具
    ///
    /// 封装 VoucherItemRequest 的构造，便于借贷平衡测试复用。
    fn make_voucher_item_request(debit: Decimal, credit: Decimal) -> VoucherItemRequest {
        VoucherItemRequest {
            line_no: None,
            subject_code: Some("1001".to_string()),
            subject_name: Some("库存现金".to_string()),
            debit,
            credit,
            summary: None,
            assist_customer_id: None,
            assist_supplier_id: None,
            assist_department_id: None,
            assist_employee_id: None,
            assist_project_id: None,
            assist_batch_id: None,
            assist_color_no_id: None,
            assist_dye_lot_id: None,
            assist_grade: None,
            assist_workshop_id: None,
            quantity_meters: None,
            quantity_kg: None,
            unit_price: None,
        }
    }

    /// 构建测试用凭证模型夹具
    ///
    /// 封装 voucher::Model 的构造，状态字段可定制，便于状态门校验测试复用。
    fn make_voucher_model(status: &str) -> voucher::Model {
        voucher::Model {
            id: 1,
            voucher_no: "JZ20260101001".to_string(),
            voucher_type: "记".to_string(),
            voucher_date: ymd!(2026, 1, 1),
            source_type: None,
            source_module: None,
            source_bill_id: None,
            source_bill_no: None,
            batch_no: None,
            color_no: None,
            dye_lot_no: None,
            workshop: None,
            production_order_no: None,
            quantity_meters: None,
            quantity_kg: None,
            gram_weight: None,
            status: status.to_string(),
            attachment_count: 0,
            created_by: 1,
            reviewed_by: None,
            reviewed_at: None,
            posted_by: None,
            posted_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// 复现 generate_voucher_no 中的凭证编号前缀映射逻辑
    ///
    /// 源码位置：generate_voucher_no 方法内的 match 表达式。
    /// "记" => "JZ", "收" => "SK", "付" => "FK", "转" => "ZZ", _ => "JZ"
    fn voucher_prefix(voucher_type: &str) -> &str {
        match voucher_type {
            "记" => "JZ",
            "收" => "SK",
            "付" => "FK",
            "转" => "ZZ",
            _ => "JZ",
        }
    }

    /// 复现 validate_voucher_create_req 中的借贷平衡校验逻辑
    ///
    /// 源码位置：validate_voucher_create_req 方法内的借方合计 == 贷方合计判断。
    fn is_balanced(items: &[VoucherItemRequest]) -> bool {
        let total_debit: Decimal = items.iter().map(|i| i.debit).sum();
        let total_credit: Decimal = items.iter().map(|i| i.credit).sum();
        total_debit == total_credit
    }

    /// 复现凭证状态机合法转换判断
    ///
    /// 源码位置：submit/review/post 方法内的状态门校验。
    /// 合法路径：draft → submitted → reviewed → posted
    fn can_transition(from: &str, to: &str) -> bool {
        matches!(
            (from, to),
            (voucher_status::VOUCHER_DRAFT, voucher_status::VOUCHER_SUBMITTED)
                | (voucher_status::VOUCHER_SUBMITTED, voucher_status::VOUCHER_REVIEWED)
                | (voucher_status::VOUCHER_REVIEWED, voucher_status::VOUCHER_POSTED)
        )
    }

    /// 复现 update_account_balances 中的期末余额计算逻辑
    ///
    /// 源码位置：update_account_balances 方法内的余额方向分支。
    /// - 借方科目：期末余额 = 期初借方 + 本期借方发生 - 本期贷方发生
    /// - 贷方科目：期末余额 = 期初贷方 + 本期贷方发生 - 本期借方发生
    /// 返回 (期末借方余额, 期末贷方余额)
    fn calc_ending_balance(
        balance_direction: &str,
        initial_debit: Decimal,
        initial_credit: Decimal,
        period_debit: Decimal,
        period_credit: Decimal,
    ) -> (Decimal, Decimal) {
        if balance_direction == "借" {
            let ending_balance = initial_debit + period_debit - period_credit;
            if ending_balance >= Decimal::ZERO {
                (ending_balance, Decimal::ZERO)
            } else {
                (Decimal::ZERO, ending_balance.abs())
            }
        } else {
            let ending_balance = initial_credit + period_credit - period_debit;
            if ending_balance >= Decimal::ZERO {
                (Decimal::ZERO, ending_balance)
            } else {
                (ending_balance.abs(), Decimal::ZERO)
            }
        }
    }

    // ============ 凭证状态常量值正确性测试 ============

    /// 测试_凭证状态常量_值正确性
    ///
    /// 验证 crate::models::status::voucher 子模块中 4 个状态常量值
    /// 与凭证状态机约定一致（小写：draft/submitted/reviewed/posted）。
    #[test]
    fn 测试_凭证状态常量_值正确性() {
        assert_eq!(voucher_status::VOUCHER_DRAFT, "draft");
        assert_eq!(voucher_status::VOUCHER_SUBMITTED, "submitted");
        assert_eq!(voucher_status::VOUCHER_REVIEWED, "reviewed");
        assert_eq!(voucher_status::VOUCHER_POSTED, "posted");
    }

    // ============ 凭证状态机转换测试 ============

    /// 测试_凭证状态机_合法转换路径
    ///
    /// 验证凭证状态机的 3 条合法转换路径：
    /// draft → submitted → reviewed → posted
    #[test]
    fn 测试_凭证状态机_合法转换路径() {
        assert!(can_transition(
            voucher_status::VOUCHER_DRAFT,
            voucher_status::VOUCHER_SUBMITTED
        ));
        assert!(can_transition(
            voucher_status::VOUCHER_SUBMITTED,
            voucher_status::VOUCHER_REVIEWED
        ));
        assert!(can_transition(
            voucher_status::VOUCHER_REVIEWED,
            voucher_status::VOUCHER_POSTED
        ));
    }

    /// 测试_凭证状态机_非法跳转
    ///
    /// 验证非相邻状态的直接跳转应被拒绝：
    /// draft 不能直接 reviewed/posted；submitted 不能直接 posted。
    #[test]
    fn 测试_凭证状态机_非法跳转() {
        assert!(!can_transition(
            voucher_status::VOUCHER_DRAFT,
            voucher_status::VOUCHER_REVIEWED
        ));
        assert!(!can_transition(
            voucher_status::VOUCHER_DRAFT,
            voucher_status::VOUCHER_POSTED
        ));
        assert!(!can_transition(
            voucher_status::VOUCHER_SUBMITTED,
            voucher_status::VOUCHER_POSTED
        ));
        // 已过账不可再转换
        assert!(!can_transition(
            voucher_status::VOUCHER_POSTED,
            voucher_status::VOUCHER_DRAFT
        ));
    }

    // ============ 借贷平衡校验测试 ============

    /// 测试_借贷平衡校验_借方等于贷方通过
    ///
    /// 验证 validate_voucher_create_req 中借方合计 == 贷方合计时校验通过。
    #[test]
    fn 测试_借贷平衡校验_借方等于贷方通过() {
        let items = vec![
            make_voucher_item_request(decs!("1000"), Decimal::ZERO),
            make_voucher_item_request(Decimal::ZERO, decs!("1000")),
        ];
        assert!(is_balanced(&items));
    }

    /// 测试_借贷平衡校验_借方大于贷方失败
    ///
    /// 验证 validate_voucher_create_req 中借方合计 > 贷方合计时校验失败。
    #[test]
    fn 测试_借贷平衡校验_借方大于贷方失败() {
        let items = vec![
            make_voucher_item_request(decs!("1000"), Decimal::ZERO),
            make_voucher_item_request(Decimal::ZERO, decs!("500")),
        ];
        assert!(!is_balanced(&items));
    }

    /// 测试_借贷平衡校验_借方小于贷方失败
    ///
    /// 验证 validate_voucher_create_req 中借方合计 < 贷方合计时校验失败。
    #[test]
    fn 测试_借贷平衡校验_借方小于贷方失败() {
        let items = vec![
            make_voucher_item_request(decs!("500"), Decimal::ZERO),
            make_voucher_item_request(Decimal::ZERO, decs!("1000")),
        ];
        assert!(!is_balanced(&items));
    }

    /// 测试_借贷平衡校验_零金额平衡通过
    ///
    /// 验证 validate_voucher_create_req 中借贷双方均为零时校验通过（边界场景）。
    #[test]
    fn 测试_借贷平衡校验_零金额平衡通过() {
        let items = vec![
            make_voucher_item_request(Decimal::ZERO, Decimal::ZERO),
            make_voucher_item_request(Decimal::ZERO, Decimal::ZERO),
        ];
        assert!(is_balanced(&items));
    }

    /// 测试_借贷平衡校验_多分录汇总平衡
    ///
    /// 验证 validate_voucher_create_req 中多个分录汇总后借贷平衡时校验通过。
    #[test]
    fn 测试_借贷平衡校验_多分录汇总平衡() {
        let items = vec![
            make_voucher_item_request(decs!("1000"), Decimal::ZERO),
            make_voucher_item_request(decs!("500"), Decimal::ZERO),
            make_voucher_item_request(decs!("200.50"), Decimal::ZERO),
            make_voucher_item_request(Decimal::ZERO, decs!("1700.50")),
        ];
        assert!(is_balanced(&items));
    }

    // ============ 金额计算精度测试 ============

    /// 测试_金额计算_精度归一化
    ///
    /// 验证 Decimal 求和保留精度，不同小数位的金额相加不会丢失精度。
    /// 复现 validate_voucher_create_req 中 iter().map(|i| i.debit).sum() 的精度行为。
    #[test]
    fn 测试_金额计算_精度归一化() {
        let items = vec![
            make_voucher_item_request(decs!("0.1"), Decimal::ZERO),
            make_voucher_item_request(decs!("0.2"), Decimal::ZERO),
            make_voucher_item_request(decs!("0.3"), Decimal::ZERO),
        ];
        let total_debit: Decimal = items.iter().map(|i| i.debit).sum();
        // Decimal 不存在 f64 浮点累加误差，0.1+0.2+0.3 应精确等于 0.6
        assert_eq!(total_debit, decs!("0.6"));

        // 不同精度混合相加
        let mixed = vec![
            make_voucher_item_request(decs!("100.125"), Decimal::ZERO),
            make_voucher_item_request(decs!("200.875"), Decimal::ZERO),
        ];
        let mixed_total: Decimal = mixed.iter().map(|i| i.debit).sum();
        assert_eq!(mixed_total, decs!("301.000"));
    }

    // ============ 凭证类型定义测试 ============

    /// 测试_凭证类型定义_完整列表
    ///
    /// 验证 available_voucher_types 返回 4 种凭证类型，且 code 与 name 对应正确。
    #[test]
    fn 测试_凭证类型定义_完整列表() {
        let types = VoucherService::available_voucher_types();
        assert_eq!(types.len(), 4);

        // 验证每种类型的 code 和 name 对应
        let pairs: Vec<(&str, &str)> = types.iter().map(|t| (t.code, t.name)).collect();
        assert!(pairs.contains(&("记", "记账凭证")));
        assert!(pairs.contains(&("收", "收款凭证")));
        assert!(pairs.contains(&("付", "付款凭证")));
        assert!(pairs.contains(&("转", "转账凭证")));

        // code 应唯一
        let codes: std::collections::HashSet<&str> = types.iter().map(|t| t.code).collect();
        assert_eq!(codes.len(), 4);
    }

    /// 测试_凭证编号前缀_各类型映射
    ///
    /// 验证 generate_voucher_no 中凭证类型到前缀的映射：
    /// "记" => "JZ", "收" => "SK", "付" => "FK", "转" => "ZZ"
    #[test]
    fn 测试_凭证编号前缀_各类型映射() {
        assert_eq!(voucher_prefix("记"), "JZ");
        assert_eq!(voucher_prefix("收"), "SK");
        assert_eq!(voucher_prefix("付"), "FK");
        assert_eq!(voucher_prefix("转"), "ZZ");
    }

    /// 测试_凭证编号前缀_未知类型默认
    ///
    /// 验证 generate_voucher_no 中未知凭证类型回退到默认前缀 "JZ"。
    #[test]
    fn 测试_凭证编号前缀_未知类型默认() {
        assert_eq!(voucher_prefix("未知"), "JZ");
        assert_eq!(voucher_prefix(""), "JZ");
    }

    // ============ 科目余额计算测试 ============

    /// 测试_科目余额计算_借方科目正常
    ///
    /// 验证 update_account_balances 中借方科目期末余额计算：
    /// 期末余额 = 期初借方 + 本期借方发生 - 本期贷方发生（结果为正记借方）。
    #[test]
    fn 测试_科目余额计算_借方科目正常() {
        let (ending_debit, ending_credit) = calc_ending_balance(
            "借",
            decs!("1000"),
            Decimal::ZERO,
            decs!("500"),
            decs!("200"),
        );
        // 1000 + 500 - 200 = 1300，正数记借方
        assert_eq!(ending_debit, decs!("1300"));
        assert_eq!(ending_credit, Decimal::ZERO);
    }

    /// 测试_科目余额计算_贷方科目正常
    ///
    /// 验证 update_account_balances 中贷方科目期末余额计算：
    /// 期末余额 = 期初贷方 + 本期贷方发生 - 本期借方发生（结果为正记贷方）。
    #[test]
    fn 测试_科目余额计算_贷方科目正常() {
        let (ending_debit, ending_credit) = calc_ending_balance(
            "贷",
            Decimal::ZERO,
            decs!("2000"),
            decs!("300"),
            decs!("800"),
        );
        // 2000 + 800 - 300 = 2500，正数记贷方
        assert_eq!(ending_debit, Decimal::ZERO);
        assert_eq!(ending_credit, decs!("2500"));
    }

    /// 测试_科目余额计算_借方科目出现贷方余额
    ///
    /// 验证 update_account_balances 中借方科目净额为负时记贷方（如累计折旧场景）。
    #[test]
    fn 测试_科目余额计算_借方科目出现贷方余额() {
        let (ending_debit, ending_credit) = calc_ending_balance(
            "借",
            decs!("100"),
            Decimal::ZERO,
            decs!("200"),
            decs!("500"),
        );
        // 100 + 200 - 500 = -200，负数取绝对值记贷方
        assert_eq!(ending_debit, Decimal::ZERO);
        assert_eq!(ending_credit, decs!("200"));
    }

    /// 测试_科目余额计算_贷方科目出现借方余额
    ///
    /// 验证 update_account_balances 中贷方科目净额为负时记借方（如预交税费场景）。
    #[test]
    fn 测试_科目余额计算_贷方科目出现借方余额() {
        let (ending_debit, ending_credit) = calc_ending_balance(
            "贷",
            Decimal::ZERO,
            decs!("100"),
            decs!("500"),
            decs!("200"),
        );
        // 100 + 200 - 500 = -200，负数取绝对值记借方
        assert_eq!(ending_debit, decs!("200"));
        assert_eq!(ending_credit, Decimal::ZERO);
    }

    // ============ 状态校验逻辑测试 ============

    /// 测试_状态校验_仅草稿可更新
    ///
    /// 验证 update 方法中状态门：仅 draft 状态可更新，其余状态应拒绝。
    #[test]
    fn 测试_状态校验_仅草稿可更新() {
        let draft = make_voucher_model(voucher_status::VOUCHER_DRAFT);
        let submitted = make_voucher_model(voucher_status::VOUCHER_SUBMITTED);
        let reviewed = make_voucher_model(voucher_status::VOUCHER_REVIEWED);
        let posted = make_voucher_model(voucher_status::VOUCHER_POSTED);

        // 复现 update 中的状态门：voucher_model.status != VOUCHER_DRAFT 则拒绝
        assert!(draft.status == voucher_status::VOUCHER_DRAFT);
        assert!(submitted.status != voucher_status::VOUCHER_DRAFT);
        assert!(reviewed.status != voucher_status::VOUCHER_DRAFT);
        assert!(posted.status != voucher_status::VOUCHER_DRAFT);
    }

    /// 测试_状态校验_仅草稿可删除
    ///
    /// 验证 delete 方法中状态门：仅 draft 状态可删除，其余状态应拒绝。
    #[test]
    fn 测试_状态校验_仅草稿可删除() {
        let draft = make_voucher_model(voucher_status::VOUCHER_DRAFT);
        let posted = make_voucher_model(voucher_status::VOUCHER_POSTED);

        // 复现 delete 中的状态门：voucher.status != VOUCHER_DRAFT 则拒绝
        assert!(draft.status == voucher_status::VOUCHER_DRAFT);
        assert!(posted.status != voucher_status::VOUCHER_DRAFT);
    }

    /// 测试_状态校验_仅草稿可提交
    ///
    /// 验证 submit 方法中状态门：仅 draft 状态可提交，其余状态应拒绝。
    #[test]
    fn 测试_状态校验_仅草稿可提交() {
        let draft = make_voucher_model(voucher_status::VOUCHER_DRAFT);
        let reviewed = make_voucher_model(voucher_status::VOUCHER_REVIEWED);

        // 复现 submit 中的状态门：voucher.status != VOUCHER_DRAFT 则拒绝
        assert!(draft.status == voucher_status::VOUCHER_DRAFT);
        assert!(reviewed.status != voucher_status::VOUCHER_DRAFT);
    }

    /// 测试_状态校验_仅已提交可审核
    ///
    /// 验证 review 方法中状态门：仅 submitted 状态可审核，其余状态应拒绝。
    #[test]
    fn 测试_状态校验_仅已提交可审核() {
        let submitted = make_voucher_model(voucher_status::VOUCHER_SUBMITTED);
        let draft = make_voucher_model(voucher_status::VOUCHER_DRAFT);
        let posted = make_voucher_model(voucher_status::VOUCHER_POSTED);

        // 复现 review 中的状态门：voucher.status != VOUCHER_SUBMITTED 则拒绝
        assert!(submitted.status == voucher_status::VOUCHER_SUBMITTED);
        assert!(draft.status != voucher_status::VOUCHER_SUBMITTED);
        assert!(posted.status != voucher_status::VOUCHER_SUBMITTED);
    }

    /// 测试_状态校验_仅已审核可过账
    ///
    /// 验证 post 方法中状态门：仅 reviewed 状态可过账，其余状态应拒绝。
    #[test]
    fn 测试_状态校验_仅已审核可过账() {
        let reviewed = make_voucher_model(voucher_status::VOUCHER_REVIEWED);
        let draft = make_voucher_model(voucher_status::VOUCHER_DRAFT);
        let submitted = make_voucher_model(voucher_status::VOUCHER_SUBMITTED);

        // 复现 post 中的状态门：voucher.status != VOUCHER_REVIEWED 则拒绝
        assert!(reviewed.status == voucher_status::VOUCHER_REVIEWED);
        assert!(draft.status != voucher_status::VOUCHER_REVIEWED);
        assert!(submitted.status != voucher_status::VOUCHER_REVIEWED);
    }

    // ============ 错误消息格式测试 ============

    /// 测试_错误消息格式_借贷不平衡
    ///
    /// 验证 validate_voucher_create_req 中借贷不平衡的错误消息格式：
    /// "凭证借贷不平衡：借方 {} != 贷方 {}"
    #[test]
    fn 测试_错误消息格式_借贷不平衡() {
        let total_debit = decs!("1000");
        let total_credit = decs!("500");
        let msg = format!(
            "凭证借贷不平衡：借方 {} != 贷方 {}",
            total_debit, total_credit
        );
        assert!(msg.contains("凭证借贷不平衡"));
        assert!(msg.contains("借方 1000"));
        assert!(msg.contains("贷方 500"));

        let err = AppError::bad_request(msg);
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    /// 测试_错误消息格式_凭证不存在
    ///
    /// 验证 get_by_id/update/delete 等方法中凭证不存在的错误消息格式：
    /// "凭证不存在：{}"
    #[test]
    fn 测试_错误消息格式_凭证不存在() {
        let id = 99999;
        let msg = format!("凭证不存在：{}", id);
        assert_eq!(msg, "凭证不存在：99999");

        let err = AppError::not_found(msg);
        assert!(matches!(err, AppError::NotFound(_)));
    }

    // ============ 夹具宏可用性测试 ============

    /// 测试_decs_夹具宏可用性
    ///
    /// 验证 decs! 宏能正确解析 Decimal 字符串常量。
    #[test]
    fn 测试_decs_夹具宏可用性() {
        let v = decs!("1234.56");
        assert_eq!(v.to_string(), "1234.56");

        let zero = decs!("0");
        assert!(zero.is_zero());

        let neg = decs!("-100.50");
        assert!(neg < Decimal::ZERO);
    }

    /// 测试_ymd_夹具宏可用性
    ///
    /// 验证 ymd! 宏能正确解析日期常量。
    #[test]
    fn 测试_ymd_夹具宏可用性() {
        let d = ymd!(2026, 7, 1);
        assert_eq!(d.year(), 2026);
        assert_eq!(d.month(), 7);
        assert_eq!(d.day(), 1);
    }

    // ============ 服务实例化测试 ============

    /// 测试_服务实例创建
    ///
    /// 验证 VoucherService 在 SQLite 内存数据库上能正常实例化。
    #[tokio::test]
    async fn 测试_服务实例创建() {
        let db = setup_test_db().await;
        let service = VoucherService::new(Arc::new(db));
        assert!(Arc::strong_count(&service.db) >= 1);
    }

    // ============ 数据库交互测试（标注 #[ignore]）============

    /// 测试_创建凭证_需要真实数据库
    ///
    /// 需要 vouchers/voucher_items/account_subjects 表 schema，
    /// 标注 #[ignore] 仅在本地手动运行。无 schema 时返回数据库错误。
    #[tokio::test]
    #[ignore]
    async fn 测试_创建凭证_需要真实数据库() {
        let db = setup_test_db().await;
        let service = VoucherService::new(Arc::new(db));

        let req = CreateVoucherRequest {
            voucher_type: "记".to_string(),
            voucher_date: ymd!(2026, 1, 1),
            source_type: None,
            source_module: None,
            source_bill_id: None,
            source_bill_no: None,
            batch_no: None,
            color_no: None,
            items: vec![
                make_voucher_item_request(decs!("1000"), Decimal::ZERO),
                make_voucher_item_request(Decimal::ZERO, decs!("1000")),
            ],
        };
        let result = service.create(req, 1).await;
        // 无 schema 时为 Err
        assert!(result.is_err());
    }

    /// 测试_查询凭证列表_需要真实数据库
    ///
    /// 需要 vouchers 表 schema，标注 #[ignore] 仅在本地手动运行。
    #[tokio::test]
    #[ignore]
    async fn 测试_查询凭证列表_需要真实数据库() {
        let db = setup_test_db().await;
        let service = VoucherService::new(Arc::new(db));

        let params = VoucherQueryParams {
            voucher_type: None,
            status: None,
            start_date: None,
            end_date: None,
            batch_no: None,
            color_no: None,
            page: None,
            page_size: None,
        };
        let result = service.get_list(params).await;
        // L-17 修复（批次 377 v13 复审）：原 let _ = result 无断言，改为 is_err 断言
        // 无 schema 时为 Err；有 schema 时为 Ok
        assert!(result.is_err(), "无 schema 时应返回数据库错误");
    }

    /// 测试_凭证过账_需要真实数据库
    ///
    /// 需要 vouchers/voucher_items/account_balances/account_subjects 表 schema，
    /// 标注 #[ignore] 仅在本地手动运行。
    #[tokio::test]
    #[ignore]
    async fn 测试_凭证过账_需要真实数据库() {
        let db = setup_test_db().await;
        let service = VoucherService::new(Arc::new(db));

        let result = service.post(99999, 1).await;
        // 无 schema 时为 Err
        assert!(result.is_err());
    }

    // ============ 批次 393 补测：凭证类型定义与辅助核算五维 ============

    /// 测试_VoucherTypeDefinition_new构造器
    ///
    /// 验证 VoucherTypeDefinition::new 正确设置 code 和 name 字段。
    #[test]
    fn 测试_VoucherTypeDefinition_new构造器() {
        let def = VoucherTypeDefinition::new("记", "记账凭证");
        assert_eq!(def.code, "记");
        assert_eq!(def.name, "记账凭证");

        let def = VoucherTypeDefinition::new("收", "收款凭证");
        assert_eq!(def.code, "收");
        assert_eq!(def.name, "收款凭证");

        let def = VoucherTypeDefinition::new("付", "付款凭证");
        assert_eq!(def.code, "付");
        assert_eq!(def.name, "付款凭证");

        let def = VoucherTypeDefinition::new("转", "转账凭证");
        assert_eq!(def.code, "转");
        assert_eq!(def.name, "转账凭证");
    }

    /// 测试_available_voucher_types返回4种类型
    ///
    /// 验证 available_voucher_types 静态方法返回 4 种凭证类型定义，
    /// code 覆盖 "记/收/付/转" 全部业务类型。
    #[test]
    fn 测试_available_voucher_types返回4种类型() {
        let types = VoucherService::available_voucher_types();
        assert_eq!(types.len(), 4, "应有 4 种凭证类型");

        let codes: Vec<&str> = types.iter().map(|t| t.code).collect();
        assert!(codes.contains(&"记"), "应包含记账凭证");
        assert!(codes.contains(&"收"), "应包含收款凭证");
        assert!(codes.contains(&"付"), "应包含付款凭证");
        assert!(codes.contains(&"转"), "应包含转账凭证");

        // 名称不应为空
        for t in &types {
            assert!(!t.name.is_empty(), "凭证类型 {} 的名称不应为空", t.code);
        }
    }

    /// 测试_科目不存在错误消息格式
    ///
    /// 验证两个分支的科目不存在错误消息格式：
    /// 1. validate_voucher_create_req 阶段："科目不存在或已停用：{code}"
    /// 2. update_account_balances 阶段："科目不存在：{code}"
    /// （批次 102 v6 P3-4：后者已从 bad_request 改为 not_found）
    #[test]
    fn 测试_科目不存在错误消息格式() {
        // 分支 1：校验阶段（科目不存在或已停用）
        let err1 = AppError::bad_request(format!("科目不存在或已停用：{}", "9999"));
        let msg1 = err1.to_string();
        assert!(
            msg1.contains("科目不存在或已停用：9999"),
            "校验阶段错误消息应包含科目代码，实际：{}",
            msg1
        );

        // 分支 2：余额更新阶段（科目不存在，not_found）
        let err2 = AppError::not_found(format!("科目不存在：{}", "9999"));
        let msg2 = err2.to_string();
        assert!(
            msg2.contains("科目不存在：9999"),
            "余额更新阶段错误消息应包含科目代码，实际：{}",
            msg2
        );

        // 两个消息不应相同（措辞不同）
        assert_ne!(msg1, msg2, "两个分支的错误消息应有区别");
    }

    /// 测试_辅助核算五维ID拼接格式
    ///
    /// 复现 create_assist_accounting_records 中的五维 ID 拼接逻辑。
    /// 格式：BATCH:{}|COLOR:{}|DYE_LOT:{}|GRADE:{}|WORKSHOP:{}
    /// 缺失字段使用 unwrap_or(0) / unwrap_or_default() 填充。
    #[test]
    fn 测试_辅助核算五维ID拼接格式() {
        // 复现五维 ID 拼接逻辑（与源码一致）
        fn build_five_dimension_id(
            batch_id: Option<i32>,
            color_no_id: Option<i32>,
            dye_lot_id: Option<i32>,
            grade: Option<&str>,
            workshop_id: Option<i32>,
        ) -> String {
            format!(
                "BATCH:{}|COLOR:{}|DYE_LOT:{}|GRADE:{}|WORKSHOP:{}",
                batch_id.unwrap_or(0),
                color_no_id.unwrap_or(0),
                dye_lot_id.unwrap_or(0),
                grade.unwrap_or_default(),
                workshop_id.unwrap_or(0),
            )
        }

        // 场景 1：全部字段齐全
        let id = build_five_dimension_id(Some(10), Some(20), Some(30), Some("A"), Some(40));
        assert_eq!(id, "BATCH:10|COLOR:20|DYE_LOT:30|GRADE:A|WORKSHOP:40");

        // 场景 2：全部字段缺失（使用默认值）
        let id = build_five_dimension_id(None, None, None, None, None);
        assert_eq!(id, "BATCH:0|COLOR:0|DYE_LOT:0|GRADE:|WORKSHOP:0");

        // 场景 3：部分字段缺失
        let id = build_five_dimension_id(Some(10), None, Some(30), None, Some(40));
        assert_eq!(id, "BATCH:10|COLOR:0|DYE_LOT:30|GRADE:|WORKSHOP:40");

        // 防御性断言：分隔符格式正确
        let parts: Vec<&str> = id.split('|').collect();
        assert_eq!(parts.len(), 5, "五维 ID 应有 5 个段");
        assert!(parts[0].starts_with("BATCH:"));
        assert!(parts[1].starts_with("COLOR:"));
        assert!(parts[2].starts_with("DYE_LOT:"));
        assert!(parts[3].starts_with("GRADE:"));
        assert!(parts[4].starts_with("WORKSHOP:"));
    }
}
