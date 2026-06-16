//! 销售报价单转销售订单 DTO
//!
//! 用于将已 approved 状态的报价单转换为销售订单。
//! Week 2+ 任务，本 Week 1 仅定义数据结构。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 转单请求 DTO
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ConvertToOrderDto {
    /// 客户 PO 号（可选，覆盖默认）
    pub po_number: Option<String>,
    /// 收货地址
    pub shipping_address: Option<String>,
    /// 收货联系人
    pub shipping_contact: Option<String>,
    /// 收货联系电话
    pub shipping_phone: Option<String>,
    /// 期望交货日期
    pub expected_delivery_date: Option<chrono::NaiveDate>,
    /// 预付定金比例（%）
    pub deposit_rate: Option<Decimal>,
    /// 转单备注
    pub notes: Option<String>,
}
