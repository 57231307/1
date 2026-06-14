//! 通用分页查询辅助模块
//!
//! 提供统一的分页查询方法，避免各 service 中重复实现相同的分页逻辑
use crate::utils::error::AppError;
use sea_orm::ConnectionTrait;
use sea_orm::Paginator;
use sea_orm::SelectorTrait;

/// 通用分页查询辅助函数
///
/// 自动处理 num_items() 和 fetch_page() 调用，并行执行以提升性能
///
/// # 参数
/// - `paginator`: SeaORM 分页器
/// - `page`: 页码（从1开始）
///
/// # 返回
/// - `(items, total)`: 包含当前页数据和总记录数
pub async fn paginate_with_total<M>(
    paginator: Paginator<'_, impl ConnectionTrait, M>,
    page: u64,
) -> Result<(Vec<M>, u64), AppError>
where
    M: SelectorTrait,
{
    let page_index = page.saturating_sub(1);

    // 顺序执行：先取当前页数据，再统计总数（避免 Paginator 在并行调用时的借用冲突）
    let items = paginator.fetch_page(page_index).await?;
    let total = paginator.num_items().await?;
    let total = total as u64;

    Ok((items, total))
}

/// 创建标准分页响应
///
/// # 参数
/// - `items`: 数据列表
/// - `total`: 总记录数
/// - `page`: 当前页
/// - `page_size`: 每页大小
pub fn create_paginated_response<T: Clone>(
    items: Vec<T>,
    total: u64,
    page: u64,
    page_size: u64,
) -> crate::utils::response::PaginatedResponse<T> {
    crate::utils::response::PaginatedResponse::new(items, total, page, page_size)
}
