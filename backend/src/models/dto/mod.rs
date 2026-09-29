#![allow(dead_code)]
//! 数据传输对象（DTO）模块
//!
//! 提供通用的 API 响应和分页请求结构

pub mod bpm_dto;
pub mod budget_dto;
pub mod budget_management_dto;
pub mod capacity_dto;
pub mod crm_dto;
pub mod dye_batch_dto;
pub mod finance_report_dto;
pub mod flow_card_dto;
pub mod fund_dto;
pub mod fund_management_dto;
pub mod scheduling_dto;
pub mod sales_analysis_dto;
pub mod tracking_dto;
pub mod wage_dto;

use serde::Deserialize;

/// 分页请求参数
#[derive(Debug, Clone, Deserialize)]
pub struct PageRequest {
    /// 页码（从 1 开始）
    pub page: u64,
    /// 每页数量
    pub page_size: u64,
}

impl Default for PageRequest {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 10,
        }
    }
}

impl PageRequest {
    // 批次 341 v11 复审 P2 修复：删除四个未使用的分页工具方法。
    // 项目已统一接入 paginate_with_total（批次 260），这些历史方法无任何调用点。
    // page_clamped/limit 的 clamp 逻辑已由 paginate_with_total 内部实现覆盖。
}
