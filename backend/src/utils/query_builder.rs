use crate::utils::error::AppError;
use crate::utils::response::PaginatedResponse;
use sea_orm::{
    DatabaseConnection, EntityTrait, PaginatorTrait, QueryOrder,
    QuerySelect, Select,
};

#[allow(dead_code)]
/// 分页查询结果
pub struct PaginatedResult<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// 适用于后台管理系统等小数据量的标准 offset 分页
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

/// 适用于核心大表（如日志、库存流水）的高性能游标分页 (基于 Sea-ORM Cursor)
/// 要求表具有主键（通常为 id），且使用主键作为游标基准
#[allow(dead_code)]
pub async fn paginate_cursor<E, M>(
    db: &DatabaseConnection,
    query: Select<E>,
    _cursor_id: Option<i32>,
    page_size: u64,
) -> Result<PaginatedResponse<M>, AppError>
where
    E: EntityTrait<Model = M>,
    M: Send + Sync + Clone + 'static,
    <E as EntityTrait>::Column: std::convert::From<sea_orm::sea_query::Alias>,
    M: sea_orm::FromQueryResult,
{
    // FIXME: 简化示例中仅用硬编码模拟，实际业务中应该使用强类型游标，如 `E::Column::Id`
    // 这里暂时注释掉未使用的泛型游标实现，以防编译报错。
    /*
    let mut cursor = query.cursor_by(sea_orm::sea_query::Alias::new("id"));
    if let Some(id) = cursor_id {
        cursor.after(id);
    }
    */

    // 这里仅做简单的占位以编译通过，在具体业务接口如 `audit_log_service` 时，需要使用:
    // AuditLog::find().cursor_by(audit_log::Column::Id)
    let items = query.limit(page_size).all(db).await?;

    Ok(PaginatedResponse::new(
        items, 0, // 游标分页通常不返回总数，以节省 COUNT(*) 开销
        1, page_size,
    ))
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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
