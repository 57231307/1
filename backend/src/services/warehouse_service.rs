use chrono::Utc;
use sea_orm::prelude::*;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, NotSet, Order, QueryFilter,
    QueryOrder, Set,
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
    #[allow(unused_variables)]
    pub async fn list_warehouses(
        &self,
        page: u64,
        page_size: u64,
        status: Option<String>,
        search: Option<String>,
    ) -> Result<(Vec<warehouse::Model>, u64), sea_orm::DbErr> {
        let mut query = WarehouseEntity::find();

        // 应用过滤条件
        if let Some(s) = status {
            query = query.filter(warehouse::Column::Status.eq(s));
        }

        if let Some(keyword) = search {
            query = query.filter(
                warehouse::Column::Name
                    .like(format!("%{}%", keyword))
                    .or(warehouse::Column::Code.like(format!("%{}%", keyword))),
            );
        }

        // 获取总数
        let total = query.clone().count(&*self.db).await?;

        // 应用分页和排序
        let warehouses = query
            .order_by(warehouse::Column::Code, Order::Asc)
            .into_model::<warehouse::Model>()
            .all(&*self.db)
            .await?;

        Ok((warehouses, total))
    }

    /// 获取仓库详情
    pub async fn get_warehouse(&self, id: i32) -> Result<warehouse::Model, sea_orm::DbErr> {
        WarehouseEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("仓库 ID {} 不存在", id)))
    }

    /// 创建仓库
    #[allow(unused_variables)]
    pub async fn create_warehouse(
        &self,
        name: String,
        code: String,
        address: Option<String>,
        manager: Option<String>,
        phone: Option<String>,
        status: String,
        warehouse_type: Option<String>,
        temperature_control: Option<bool>,
        humidity_control: Option<bool>,
        fire_protection_level: Option<String>,
    ) -> Result<warehouse::Model, sea_orm::DbErr> {
        let active_model = warehouse::ActiveModel {
            id: NotSet,
            code: Set(code),
            name: Set(name),
            address: Set(address),
            manager: Set(manager),
            phone: Set(phone),
            status: Set(status),
            warehouse_type: Set(warehouse_type),
            temperature_control: Set(temperature_control),
            humidity_control: Set(humidity_control),
            fire_protection_level: Set(fire_protection_level),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        let result = active_model.insert(&*self.db).await?;
        Ok(result)
    }

    /// 更新仓库
    #[allow(clippy::too_many_arguments)]
    #[allow(unused_variables)]
    pub async fn update_warehouse(
        &self,
        id: i32,
        name: Option<String>,
        address: Option<String>,
        manager: Option<String>,
        phone: Option<String>,
        status: Option<String>,
        warehouse_type: Option<String>,
        temperature_control: Option<bool>,
        humidity_control: Option<bool>,
        fire_protection_level: Option<String>,
    ) -> Result<warehouse::Model, sea_orm::DbErr> {
        let mut wh: warehouse::ActiveModel = WarehouseEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("仓库 ID {} 不存在", id)))?
            .into();

        if let Some(n) = name {
            wh.name = Set(n);
        }
        if let Some(a) = address {
            wh.address = Set(Some(a));
        }
        if let Some(m) = manager {
            wh.manager = Set(Some(m));
        }
        if let Some(p) = phone {
            wh.phone = Set(Some(p));
        }
        if let Some(s) = status {
            wh.status = Set(s);
        }
        if let Some(w_type) = warehouse_type {
            wh.warehouse_type = Set(Some(w_type));
        }
        if let Some(tc) = temperature_control {
            wh.temperature_control = Set(Some(tc));
        }
        if let Some(hc) = humidity_control {
            wh.humidity_control = Set(Some(hc));
        }
        if let Some(fpl) = fire_protection_level {
            wh.fire_protection_level = Set(Some(fpl));
        }

        wh.updated_at = Set(Utc::now());

        let result = wh.update(&*self.db).await?;
        Ok(result)
    }

    /// 删除仓库
    pub async fn delete_warehouse(&self, id: i32) -> Result<(), sea_orm::DbErr> {
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
    #[allow(dead_code)]
    pub async fn find_by_code(&self, code: &str) -> Result<warehouse::Model, sea_orm::DbErr> {
        warehouse::Entity::find()
            .filter(warehouse::Column::Code.eq(code))
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("仓库编码 {} 不存在", code)))
    }
}
