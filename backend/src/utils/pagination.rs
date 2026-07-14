//! 通用分页查询辅助模块
//!
//! 提供统一的分页查询方法，避免各 service 中重复实现相同的分页逻辑
use sea_orm::ConnectionTrait;
use sea_orm::Paginator;

use crate::utils::error::AppError;

/// 通用分页查询辅助函数（自动处理 num_items 和 fetch_page）
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
