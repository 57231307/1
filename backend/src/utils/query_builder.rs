use crate::utils::error::AppError;
use crate::utils::response::PaginatedResponse;
use sea_orm::{DatabaseConnection, EntityTrait, PaginatorTrait, QueryOrder, Select};

/// 分页查询结果
pub struct PaginatedResult<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// 执行分页查询
///
/// # 参数
/// - `query`: SeaORM 查询对象
/// - `db`: 数据库连接
/// - `page`: 页码（从 1 开始）
/// - `page_size`: 每页数量
///
/// # 返回
/// 返回分页结果，包含数据列表、总数、当前页码和每页数量
pub async fn execute_paginated_query<E>(
    query: Select<E>,
    db: &DatabaseConnection,
    page: u64,
    page_size: u64,
) -> Result<PaginatedResult<E::Model>, AppError>
where
    E: EntityTrait,
    E::Model: Send + Sync,
{
    let paginator = query.paginate(db, page_size);
    let total = paginator.num_items().await?;
    let items = paginator.fetch_page(page - 1).await?;

    Ok(PaginatedResult {
        items,
        total,
        page,
        page_size,
    })
}

/// 执行分页查询并返回 PaginatedResponse
///
/// # 参数
/// - `query`: SeaORM 查询对象
/// - `db`: 数据库连接
/// - `page`: 页码（从 1 开始）
/// - `page_size`: 每页数量
///
/// # 返回
/// 返回 PaginatedResponse 结构
pub async fn execute_paginated_query_with_response<E>(
    query: Select<E>,
    db: &DatabaseConnection,
    page: u64,
    page_size: u64,
) -> Result<PaginatedResponse<E::Model>, AppError>
where
    E: EntityTrait,
    E::Model: Send + Sync + Clone,
{
    let result = execute_paginated_query(query, db, page, page_size).await?;
    Ok(PaginatedResponse::new(
        result.items,
        result.total,
        result.page,
        result.page_size,
    ))
}

/// 执行带排序的分页查询
///
/// # 参数
/// - `query`: SeaORM 查询对象
/// - `db`: 数据库连接
/// - `page`: 页码（从 1 开始）
/// - `page_size`: 每页数量
/// - `order_column`: 排序列
/// - `order`: 排序方向（升序/降序）
///
/// # 返回
/// 返回分页结果
pub async fn execute_ordered_paginated_query<E, C>(
    query: Select<E>,
    db: &DatabaseConnection,
    page: u64,
    page_size: u64,
    order_column: C,
    order: sea_orm::Order,
) -> Result<PaginatedResult<E::Model>, AppError>
where
    E: EntityTrait,
    E::Model: Send + Sync,
    C: sea_orm::ColumnTrait,
{
    let ordered_query = query.order_by(order_column, order);
    execute_paginated_query(ordered_query, db, page, page_size).await
}

/// 执行带排序的分页查询并返回 PaginatedResponse
///
/// # 参数
/// - `query`: SeaORM 查询对象
/// - `db`: 数据库连接
/// - `page`: 页码（从 1 开始）
/// - `page_size`: 每页数量
/// - `order_column`: 排序列
/// - `order`: 排序方向（升序/降序）
///
/// # 返回
/// 返回 PaginatedResponse 结构
pub async fn execute_ordered_paginated_query_with_response<E, C>(
    query: Select<E>,
    db: &DatabaseConnection,
    page: u64,
    page_size: u64,
    order_column: C,
    order: sea_orm::Order,
) -> Result<PaginatedResponse<E::Model>, AppError>
where
    E: EntityTrait,
    E::Model: Send + Sync + Clone,
    C: sea_orm::ColumnTrait,
{
    let result =
        execute_ordered_paginated_query(query, db, page, page_size, order_column, order).await?;
    Ok(PaginatedResponse::new(
        result.items,
        result.total,
        result.page,
        result.page_size,
    ))
}

/// 应用可选的过滤条件
///
/// # 参数
/// - `query`: 原始查询
/// - `condition`: 可选条件值
/// - `filter_fn`: 过滤函数，接收条件值并返回过滤后的查询
///
/// # 返回
/// 返回应用过滤条件后的查询
pub fn apply_optional_filter<E, C, F>(
    query: Select<E>,
    condition: Option<C>,
    filter_fn: F,
) -> Select<E>
where
    E: EntityTrait,
    F: FnOnce(Select<E>, C) -> Select<E>,
{
    match condition {
        Some(value) => filter_fn(query, value),
        None => query,
    }
}
