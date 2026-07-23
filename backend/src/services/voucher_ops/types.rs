//! 凭证服务的内部类型定义（voucher_ops/types）
//!
//! 批次 488 D10-4 拆分：从原 `voucher_service.rs` L88-113 迁移。
//! - `BalanceUpdateContext`：余额更新上下文，balance.rs 使用
//! - `AssistRecordContext`：辅助核算记录写入上下文，assist.rs 使用

use rust_decimal::Decimal;

use crate::models::{account_subject, voucher};

/// 余额更新上下文：封装科目列表、聚合发生额、锁定的现有余额记录
pub(crate) struct BalanceUpdateContext {
    pub(crate) subjects: Vec<account_subject::Model>,
    pub(crate) balance_map: std::collections::HashMap<i32, (Decimal, Decimal)>,
    pub(crate) existing_balances: Vec<crate::models::account_balance::Model>,
}

/// 辅助核算记录写入上下文（D08 第三梯队修复：消除 too_many_arguments 警告）
///
/// 封装业务关联字段与凭证上下文，避免 insert_assist_records_for_items /
/// build_assist_record 函数签名携带过多参数。
pub(crate) struct AssistRecordContext<'a> {
    /// 业务类型
    pub(crate) business_type: &'a str,
    /// 业务单号
    pub(crate) business_no: &'a str,
    /// 业务单据 ID
    pub(crate) business_id: i32,
    /// 凭证 ID
    pub(crate) voucher_id: i32,
    /// 凭证模型
    pub(crate) voucher_model: &'a voucher::Model,
    /// 创建人 ID
    pub(crate) user_id: i32,
    /// 创建时间
    pub(crate) now: chrono::DateTime<chrono::Utc>,
}
