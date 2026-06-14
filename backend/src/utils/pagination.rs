//! 通用分页查询辅助模块
//!
//! 提供统一的分页查询方法，避免各 service 中重复实现相同的分页逻辑
use sea_orm::ConnectionTrait;
use sea_orm::Paginator;

use crate::utils::error::AppError;

/// 通用分页查询辅助函数
///
/// 自动处理 num_items() 和 fetch_page() 调用，避免在每个 service 中重复实现
///
/// # 参数
/// - `paginator`: SeaORM 分页器
/// - `page`: 页码（从1开始）
///
/// # 返回
/// - `(items, total)`: 包含当前页数据和总记录数
///
/// # 说明
/// sea-orm 2.0 起 Paginator 的元素类型约束从 `ModelTrait` 改为 `SelectorTrait`。
/// 本函数直接约束 `S = SelectModel<M>`，返回 `Vec<M>`，与调用方期望一致。
///
/// `num_items()` 在 sea-orm 2.0 中返回 `u64`，与函数签名一致，不需要重复转换。
pub async fn paginate_with_total<M>(
    paginator: Paginator<'_, impl ConnectionTrait, sea_orm::SelectModel<M>>,
    page: u64,
) -> Result<(Vec<M>, u64), AppError>
where
    M: sea_orm::FromQueryResult + Sized + Send + Sync + 'static,
{
    let page_index = page.saturating_sub(1);

    // 顺序执行：先取当前页数据，再统计总数（避免 Paginator 在并行调用时的借用冲突）
    let items: Vec<M> = paginator.fetch_page(page_index).await?;
    let total = paginator.num_items().await?;

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
