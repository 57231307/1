//! 客户查询（带数据权限过滤）impl 子模块（customer_ops/query）
//!
//! 拆分：从原 `customer_service.rs` 迁移 CustomerService 的查询方法：
//! - apply_customer_list_filters（静态：应用筛选条件到 query）
//! - fetch_customers_with_permission_filter（按权限过滤器选择字段查询）
//! - fetch_customers_all_fields_json（全字段查询并转 JSON 行）
//! - delegate_list_customers_to_json（无权限过滤时委托给 list_customers）
//! - list_customers_with_filter（带数据权限过滤的列表查询）
//! - get_customer_with_filter（带数据权限过滤的详情查询，复用 get_customer 行级校验）

use sea_orm::{
    ColumnTrait, EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
};

use crate::models::customer::{self, Entity as CustomerEntity};
use crate::models::dto::PageRequest;
use crate::services::customer_ops::types::build_select_only_query;
use crate::services::customer_service::CustomerService;
use crate::utils::data_permission::DataPermissionFilter;
use crate::utils::data_scope::{apply_data_scope, DataScopeContext};
use crate::utils::error::AppError;
use crate::utils::PaginatedResponse;

impl CustomerService {
    /// 无权限过滤时委托给 list_customers 并转换为 JSON 分页响应
    async fn delegate_list_customers_to_json(
        &self,
        page_req: &PageRequest,
        status: &Option<String>,
        customer_type: &Option<String>,
        keyword: &Option<String>,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<Option<PaginatedResponse<serde_json::Value>>, AppError> {
        let paged = self
            .list_customers(
                page_req.clone(),
                status.clone(),
                customer_type.clone(),
                keyword.clone(),
                data_scope,
            )
            .await?;
        let items: Vec<serde_json::Value> = paged
            .items
            .into_iter()
            .map(|c| {
                serde_json::to_value(c)
                    .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(PaginatedResponse::new(
            items,
            paged.total,
            paged.page,
            paged.page_size,
        )))
    }

    /// 应用客户列表筛选条件（数据权限/状态/类型/关键词）
    fn apply_customer_list_filters(
        mut query: sea_orm::Select<CustomerEntity>,
        status: &Option<String>,
        customer_type: &Option<String>,
        keyword: &Option<String>,
        data_scope: Option<&DataScopeContext>,
    ) -> sea_orm::Select<CustomerEntity> {
        if let Some(ctx) = data_scope {
            query = apply_data_scope(
                query,
                ctx,
                customer::Column::CreatedBy,
                customer::Column::CreatedBy,
            );
        }
        if let Some(status) = status {
            query = query.filter(customer::Column::Status.eq(status));
        }
        if let Some(customer_type) = customer_type {
            query = query.filter(customer::Column::CustomerType.eq(customer_type));
        }
        if let Some(keyword) = keyword {
            query = query.filter(
                customer::Column::CustomerName
                    .contains(keyword)
                    .or(customer::Column::CustomerCode.contains(keyword)),
            );
        }
        query
    }

    /// 按权限过滤器选择字段查询客户列表（JSON 行）
    async fn fetch_customers_with_permission_filter(
        &self,
        query: sea_orm::Select<CustomerEntity>,
        filter: &DataPermissionFilter,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<serde_json::Value>, AppError> {
        let select_query = build_select_only_query(query, filter);
        let rows = select_query
            .order_by(customer::Column::CreatedAt, Order::Desc)
            .offset(offset)
            .limit(limit)
            .into_json()
            .all(&*self.db)
            .await?;
        Ok(rows)
    }

    /// 查询全字段客户列表并转为 JSON 行
    async fn fetch_customers_all_fields_json(
        &self,
        query: sea_orm::Select<CustomerEntity>,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<serde_json::Value>, AppError> {
        let rows = query
            .order_by(customer::Column::CreatedAt, Order::Desc)
            .offset(offset)
            .limit(limit)
            .all(&*self.db)
            .await?;
        let json_rows: Vec<serde_json::Value> = rows
            .into_iter()
            .map(|c| {
                serde_json::to_value(c)
                    .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(json_rows)
    }

    /// 获取客户列表（带数据权限过滤）
    pub async fn list_customers_with_filter(
        &self,
        page_req: PageRequest,
        status: Option<String>,
        customer_type: Option<String>,
        keyword: Option<String>,
        permission_filter: Option<DataPermissionFilter>,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<PaginatedResponse<serde_json::Value>, AppError> {
        if permission_filter.is_none() {
            if let Some(paged) = self
                .delegate_list_customers_to_json(
                    &page_req,
                    &status,
                    &customer_type,
                    &keyword,
                    data_scope,
                )
                .await?
            {
                return Ok(paged);
            }
        }
        let query = CustomerEntity::find();
        let query =
            Self::apply_customer_list_filters(query, &status, &customer_type, &keyword, data_scope);
        let total = query.clone().count(&*self.db).await?;
        let offset = page_req.page.saturating_sub(1) * page_req.page_size;
        let customers = if let Some(filter) = permission_filter {
            self.fetch_customers_with_permission_filter(query, &filter, offset, page_req.page_size)
                .await?
        } else {
            self.fetch_customers_all_fields_json(query, offset, page_req.page_size)
                .await?
        };
        Ok(PaginatedResponse::new(
            customers,
            total,
            page_req.page,
            page_req.page_size,
        ))
    }

    /// 获取客户详情（带数据权限过滤）
    pub async fn get_customer_with_filter(
        &self,
        customer_id: i32,
        permission_filter: Option<DataPermissionFilter>,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<serde_json::Value, AppError> {
        // V15 P0-S01：先校验行级数据权限（IDOR 防护），复用 get_customer 的校验逻辑
        let model = self.get_customer(customer_id, data_scope).await?;

        let customer = if let Some(filter) = permission_filter {
            // 基于已校验的 model id 重新查询字段过滤版本
            let query = CustomerEntity::find_by_id(customer_id);
            let select_query = build_select_only_query(query, &filter);
            select_query
                .into_json()
                .one(&*self.db)
                .await?
                .ok_or_else(|| AppError::not_found(format!("客户 {} 未找到", customer_id)))?
        } else {
            serde_json::to_value(model)
                .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?
        };

        Ok(customer)
    }
}
