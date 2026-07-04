#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
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
    /// 创建分页请求
    pub fn new(page: u64, page_size: u64) -> Self {
        Self { page, page_size }
    }

    /// 批次 98 P2 修复（v5 复审）：获取经 clamp 的页码
    /// 上限 1000 防止深度分页 DoS（page=u64::MAX 触发 offset 溢出导致 DB 全表扫描）
    pub fn page_clamped(&self) -> u64 {
        self.page.clamp(1, 1000)
    }

    /// 获取偏移量（基于 clamp 后的页码计算）
    pub fn offset(&self) -> u64 {
        (self.page_clamped().saturating_sub(1)) * self.limit()
    }

    /// 获取每页数量（限制最大 100）
    pub fn limit(&self) -> u64 {
        self.page_size.max(1).min(100)
    }
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
