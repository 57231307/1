//! 数据传输对象（DTO）模块
//!
//! 提供通用的 API 响应和分页请求结构

pub mod bpm_dto;
pub mod budget_dto;
pub mod crm_dto;
pub mod fund_dto;

use serde::{Deserialize, Serialize};

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

/// 分页响应结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageResponse<T> {
    /// 总记录数
    pub total: u64,
    /// 当前页码
    pub page: u64,
    /// 每页数量
    pub page_size: u64,
    /// 总页数
    pub total_pages: u64,
    /// 数据列表
    pub data: Vec<T>,
}

impl<T> PageResponse<T> {
    /// 创建分页响应
    pub fn new(data: Vec<T>, total: u64, page: u64, page_size: u64) -> Self {
        let total_pages = if total == 0 {
            0
        } else {
            total.div_ceil(page_size)
        };
        Self {
            total,
            page,
            page_size,
            total_pages,
            data,
        }
    }
}
