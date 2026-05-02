use chrono::Utc;
use sea_orm::prelude::*;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, NotSet, Order, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use std::sync::Arc;

use crate::models::warehouse::{self, Entity as WarehouseEntity};

/// 仓库服务
pub struct WarehouseService {
    db: Arc<DatabaseConnection>,
}

impl WarehouseService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取仓库列表（支持分页和过滤）
    pub async fn list(
        &self,
        query: crate::handlers::warehouse_handler::WarehouseListQuery,
    ) -> Result<crate::utils::response::PaginatedResponse<warehouse::Model>, sea_orm::DbErr> {
        let mut q = WarehouseEntity::find();

        // 应用过滤条件
        if let Some(s) = query.status {
            q = q.filter(warehouse::Column::IsActive.eq(s == "active"));
        }

        if let Some(keyword) = query.search {
            q = q.filter(
                warehouse::Column::Name
                    .like(format!("%{}%", keyword))
                    .or(warehouse::Column::WarehouseCode.like(format!("%{}%", keyword))),
            );
        }

        // 获取总数
        let total = q.clone().count(&*self.db).await?;

        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(10);

        // 应用分页和排序
        let warehouses = q
            .order_by(warehouse::Column::WarehouseCode, Order::Asc)
            .offset((page - 1) * page_size)
            .limit(page_size)
            .into_model::<warehouse::Model>()
            .all(&*self.db)
            .await?;

        Ok(crate::utils::response::PaginatedResponse::new(warehouses, total, page, page_size))
    }

    /// 获取仓库详情
    pub async fn get(&self, id: i32) -> Result<warehouse::Model, sea_orm::DbErr> {
        WarehouseEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("仓库 ID {} 不存在", id)))
    }

    /// 创建仓库
    pub async fn create(
        &self,
        req: crate::handlers::warehouse_handler::CreateWarehouseRequest,
    ) -> Result<warehouse::Model, sea_orm::DbErr> {
        let active_model = warehouse::ActiveModel {
            id: NotSet,
            warehouse_code: Set(req.code),
            name: Set(req.name),
            address: Set(req.address),
            city: Set(None),
            province: Set(None),
            country: Set(None),
            postal_code: Set(None),
            phone: Set(req.phone),
            email: Set(None),
            manager_id: Set(None),
            is_active: Set(true),
            notes: Set(None),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        let result = active_model.insert(&*self.db).await?;
        Ok(result)
    }

    /// 更新仓库
    pub async fn update(
        &self,
        id: i32,
        req: crate::handlers::warehouse_handler::UpdateWarehouseRequest,
    ) -> Result<warehouse::Model, sea_orm::DbErr> {
        let mut wh: warehouse::ActiveModel = WarehouseEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("仓库 ID {} 不存在", id)))?
            .into();

        if let Some(n) = req.name {
            wh.name = Set(n);
        }
        if let Some(a) = req.address {
            wh.address = Set(Some(a));
        }
        if let Some(m) = req.manager {
            wh.manager_id = Set(Some(m.parse::<i32>().unwrap_or(0)));
        }
        if let Some(p) = req.phone {
            wh.phone = Set(Some(p));
        }
        if let Some(s) = req.status {
            wh.is_active = Set(s == "active");
        }

        wh.updated_at = Set(Utc::now());

        let result = wh.update(&*self.db).await?;
        Ok(result)
    }

    /// 删除仓库
    pub async fn delete(&self, id: i32) -> Result<(), sea_orm::DbErr> {
        let result = WarehouseEntity::delete_by_id(id).exec(&*self.db).await?;
        if result.rows_affected == 0 {
            return Err(sea_orm::DbErr::RecordNotFound(format!(
                "仓库 ID {} 不存在",
                id
            )));
        }
        Ok(())
    }

    /// 根据仓库编码查询仓库
    pub async fn find_by_code(&self, code: &str) -> Result<warehouse::Model, sea_orm::DbErr> {
        warehouse::Entity::find()
            .filter(warehouse::Column::WarehouseCode.eq(code))
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("仓库编码 {} 不存在", code)))
    }
}
