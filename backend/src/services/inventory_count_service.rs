use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, TransactionTrait,
};
use std::sync::Arc;

use crate::models::dto::PageRequest;
use crate::models::inventory_count::{self, Entity as InventoryCountEntity};
use crate::models::inventory_count_item::{self, Entity as InventoryCountItemEntity};
use crate::models::inventory_stock::{self, Entity as InventoryStockEntity};
use crate::utils::PaginatedResponse;
use serde::{Deserialize, Serialize};

/// 库存盘点详情响应
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InventoryCountDetail {
    pub id: i32,
    pub count_no: String,
    pub warehouse_id: i32,
    pub count_date: chrono::DateTime<chrono::Utc>,
    pub status: String,
    pub total_items: i32,
    pub counted_items: i32,
    pub variance_items: i32,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub items: Vec<InventoryCountItemDetail>,
}

/// 库存盘点明细项详情
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InventoryCountItemDetail {
    pub id: i32,
    pub count_id: i32,
    pub product_id: i32,
    pub stock_id: i32,
    pub warehouse_id: i32,
    pub quantity_before: rust_decimal::Decimal,
    pub quantity_actual: rust_decimal::Decimal,
    pub quantity_difference: rust_decimal::Decimal,
    pub unit_cost: rust_decimal::Decimal,
    pub total_cost: rust_decimal::Decimal,
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 创建库存盘点请求
#[derive(Debug, Deserialize)]
pub struct CreateInventoryCountRequest {
    pub warehouse_id: i32,
    pub count_date: Option<chrono::DateTime<chrono::Utc>>,
    pub status: String,
    pub notes: Option<String>,
    pub items: Option<Vec<InventoryCountItemRequest>>,
}

#[derive(Debug, Deserialize)]
pub struct InventoryCountItemRequest {
    pub product_id: i32,
    pub stock_id: i32,
    pub warehouse_id: i32,
    pub quantity_actual: rust_decimal::Decimal,
    pub unit_cost: rust_decimal::Decimal,
    pub notes: Option<String>,
}

/// 更新库存盘点请求
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct UpdateInventoryCountRequest {
    pub status: Option<String>,
    pub notes: Option<String>,
    pub items: Option<Vec<InventoryCountItemRequest>>,
}

/// 库存盘点服务
pub struct InventoryCountService {
    db: Arc<DatabaseConnection>,
}

impl InventoryCountService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取库存盘点列表
    pub async fn list_counts(
        &self,
        page_req: PageRequest,
        status: Option<String>,
        warehouse_id: Option<i32>,
        count_no: Option<String>,
    ) -> Result<PaginatedResponse<InventoryCountDetail>, sea_orm::DbErr> {
        let mut query =
            InventoryCountEntity::find().order_by(inventory_count::Column::CreatedAt, Order::Desc);

        // 应用过滤条件
        if let Some(s) = status {
            query = query.filter(inventory_count::Column::Status.eq(s));
        }
        if let Some(wid) = warehouse_id {
            query = query.filter(inventory_count::Column::WarehouseId.eq(wid));
        }
        if let Some(no) = count_no {
            query = query.filter(inventory_count::Column::CountNo.contains(&no));
        }

        // 分页
        let paginator = query.paginate(&*self.db, page_req.page_size);
        let total = paginator.num_items().await?;
        let counts: Vec<inventory_count::Model> = paginator.fetch_page(page_req.page - 1).await?;

        // 转换为响应格式
        let count_details: Vec<InventoryCountDetail> = counts
            .into_iter()
            .map(|count| InventoryCountDetail {
                id: count.id,
                count_no: count.count_no,
                warehouse_id: count.warehouse_id,
                count_date: count.count_date,
                status: count.status,
                total_items: count.total_items,
                counted_items: count.counted_items,
                variance_items: count.variance_items,
                notes: count.notes,
                created_by: count.created_by,
                approved_by: count.approved_by,
                approved_at: count.approved_at,
                completed_at: count.completed_at,
                created_at: count.created_at,
                updated_at: count.updated_at,
                items: vec![], // 列表接口不返回明细项
            })
            .collect();

        Ok(PaginatedResponse::new(
            count_details,
            total,
            page_req.page,
            page_req.page_size,
        ))
    }

    /// 获取库存盘点详情（包含明细项）
    pub async fn get_count_detail(
        &self,
        count_id: i32,
    ) -> Result<InventoryCountDetail, sea_orm::DbErr> {
        // 获取盘点主表数据
        let count = InventoryCountEntity::find_by_id(count_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                sea_orm::DbErr::RecordNotFound(format!("库存盘点单 {} 未找到", count_id))
            })?;

        // 获取盘点明细项
        let items = InventoryCountItemEntity::find()
            .filter(inventory_count_item::Column::CountId.eq(count_id))
            .order_by(inventory_count_item::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;

        // 转换为响应格式
        let item_details: Vec<InventoryCountItemDetail> = items
            .into_iter()
            .map(|item| InventoryCountItemDetail {
                id: item.id,
                count_id: item.count_id,
                product_id: item.product_id,
                stock_id: item.stock_id,
                warehouse_id: item.warehouse_id,
                quantity_before: item.quantity_before,
                quantity_actual: item.quantity_actual,
                quantity_difference: item.quantity_difference,
                unit_cost: item.unit_cost,
                total_cost: item.total_cost,
                notes: item.notes,
                created_at: item.created_at,
                updated_at: item.updated_at,
            })
            .collect();

        Ok(InventoryCountDetail {
            id: count.id,
            count_no: count.count_no,
            warehouse_id: count.warehouse_id,
            count_date: count.count_date,
            status: count.status,
            total_items: count.total_items,
            counted_items: count.counted_items,
            variance_items: count.variance_items,
            notes: count.notes,
            created_by: count.created_by,
            approved_by: count.approved_by,
            approved_at: count.approved_at,
            completed_at: count.completed_at,
            created_at: count.created_at,
            updated_at: count.updated_at,
            items: item_details,
        })
    }

    /// 创建库存盘点
    pub async fn create_count(
        &self,
        request: CreateInventoryCountRequest,
    ) -> Result<InventoryCountDetail, sea_orm::DbErr> {
        // 开启事务
        let txn = (*self.db).begin().await?;

        // 生成盘点单号并检查唯一性
        let count_no = self.generate_count_no().await?;

        // 再次检查盘点单号是否已存在（防止并发冲突）
        let existing_count = InventoryCountEntity::find()
            .filter(inventory_count::Column::CountNo.eq(&count_no))
            .one(&txn)
            .await?;

        if existing_count.is_some() {
            txn.rollback().await?;
            return Err(sea_orm::DbErr::Custom("盘点单号已存在，请重试".to_string()));
        }

        // 创建盘点主表
        let count = inventory_count::ActiveModel {
            id: Default::default(),
            count_no: sea_orm::ActiveValue::Set(count_no),
            warehouse_id: sea_orm::ActiveValue::Set(request.warehouse_id),
            count_date: sea_orm::ActiveValue::Set(
                request.count_date.unwrap_or_else(chrono::Utc::now),
            ),
            status: sea_orm::ActiveValue::Set(request.status),
            total_items: sea_orm::ActiveValue::Set(0),
            counted_items: sea_orm::ActiveValue::Set(0),
            variance_items: sea_orm::ActiveValue::Set(0),
            notes: sea_orm::ActiveValue::Set(request.notes),
            created_by: sea_orm::ActiveValue::NotSet,
            approved_by: sea_orm::ActiveValue::NotSet,
            approved_at: sea_orm::ActiveValue::NotSet,
            completed_at: sea_orm::ActiveValue::NotSet,
            created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
        };

        let count_entity = count.insert(&txn).await?;

        // 如果需要创建明细项
        let mut total_items = 0;
        let has_items = request.items.is_some();
        if let Some(items) = request.items {
            for item_req in items {
                total_items += 1;

                // 创建明细项
                let item = inventory_count_item::ActiveModel {
                    id: Default::default(),
                    count_id: sea_orm::ActiveValue::Set(count_entity.id),
                    product_id: sea_orm::ActiveValue::Set(item_req.product_id),
                    stock_id: sea_orm::ActiveValue::Set(item_req.stock_id),
                    warehouse_id: sea_orm::ActiveValue::Set(item_req.warehouse_id),
                    quantity_before: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                    quantity_actual: sea_orm::ActiveValue::Set(item_req.quantity_actual),
                    quantity_difference: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                    unit_cost: sea_orm::ActiveValue::Set(item_req.unit_cost),
                    total_cost: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                    notes: sea_orm::ActiveValue::Set(item_req.notes),
                    created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                    updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                };

                item.insert(&txn).await?;
            }
        }

        // 更新盘点单统计
        let mut count_update: inventory_count::ActiveModel = count_entity.clone().into();
        count_update.total_items = sea_orm::ActiveValue::Set(total_items);
        count_update.counted_items =
            sea_orm::ActiveValue::Set(if has_items { total_items } else { 0 });
        count_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        count_update.update(&txn).await?;

        // 提交事务
        txn.commit().await?;

        // 返回盘点详情
        self.get_count_detail(count_entity.id).await
    }

    /// 更新库存盘点
    pub async fn update_count(
        &self,
        count_id: i32,
        request: UpdateInventoryCountRequest,
    ) -> Result<InventoryCountDetail, sea_orm::DbErr> {
        // 检查盘点单是否存在
        let count = InventoryCountEntity::find_by_id(count_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                sea_orm::DbErr::RecordNotFound(format!("库存盘点单 {} 未找到", count_id))
            })?;

        // 检查状态，已完成的盘点单不允许修改
        if count.status == "completed" {
            return Err(sea_orm::DbErr::Custom(
                "盘点单已完成，不允许修改".to_string(),
            ));
        }

        // 开启事务
        let txn = (*self.db).begin().await?;

        // 更新盘点主表
        let mut count_update: inventory_count::ActiveModel = count.into();
        if let Some(status) = request.status {
            count_update.status = sea_orm::ActiveValue::Set(status);
        }
        if let Some(notes) = request.notes {
            count_update.notes = sea_orm::ActiveValue::Set(Some(notes));
        }
        count_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        count_update.update(&txn).await?;

        // 提交事务
        txn.commit().await?;

        // 返回盘点详情
        self.get_count_detail(count_id).await
    }

    /// 审核库存盘点
    pub async fn approve_count(
        &self,
        count_id: i32,
        approved: bool,
        notes: Option<String>,
    ) -> Result<InventoryCountDetail, sea_orm::DbErr> {
        // 检查盘点单是否存在
        let count = InventoryCountEntity::find_by_id(count_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                sea_orm::DbErr::RecordNotFound(format!("库存盘点单 {} 未找到", count_id))
            })?;

        // 检查状态，只有待审核的盘点单可以审核
        if count.status != "pending" {
            return Err(sea_orm::DbErr::Custom(
                "只有待审核状态的盘点单可以审核".to_string(),
            ));
        }

        // 开启事务
        let txn = (*self.db).begin().await?;

        // 更新盘点单状态
        let mut count_update: inventory_count::ActiveModel = count.into();
        if approved {
            count_update.status = sea_orm::ActiveValue::Set("approved".to_string());
            count_update.approved_by = sea_orm::ActiveValue::NotSet; // 实际应从认证信息获取
            count_update.approved_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now()));
        } else {
            count_update.status = sea_orm::ActiveValue::Set("rejected".to_string());
        }
        if let Some(n) = notes {
            count_update.notes = sea_orm::ActiveValue::Set(Some(n));
        }
        count_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        count_update.update(&txn).await?;

        // 提交事务
        txn.commit().await?;

        // 返回盘点详情
        self.get_count_detail(count_id).await
    }

    /// 完成库存盘点并调整库存
    pub async fn complete_count(
        &self,
        count_id: i32,
    ) -> Result<InventoryCountDetail, sea_orm::DbErr> {
        // 检查盘点单是否存在
        let count = InventoryCountEntity::find_by_id(count_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                sea_orm::DbErr::RecordNotFound(format!("库存盘点单 {} 未找到", count_id))
            })?;

        // 检查状态，只有已审核的盘点单可以完成
        if count.status != "approved" {
            return Err(sea_orm::DbErr::Custom(
                "只有已审核状态的盘点单可以完成".to_string(),
            ));
        }

        // 开启事务
        let txn = (*self.db).begin().await?;

        // 获取盘点明细项
        let items = InventoryCountItemEntity::find()
            .filter(inventory_count_item::Column::CountId.eq(count_id))
            .all(&txn)
            .await?;

        let mut variance_count = 0;

        // 处理每个盘点明细项
        for item in items {
            // 查找对应库存记录
            let stock = InventoryStockEntity::find()
                .filter(inventory_stock::Column::WarehouseId.eq(count.warehouse_id))
                .filter(inventory_stock::Column::ProductId.eq(item.product_id))
                .one(&txn)
                .await?;

            if let Some(stock_model) = stock {
                // 获取账面数量
                let quantity_book = stock_model.quantity_on_hand;

                // 计算差异
                let quantity_variance = item.quantity_actual - quantity_book;

                // 更新明细项
                let mut item_update: inventory_count_item::ActiveModel = item.clone().into();
                item_update.quantity_before = sea_orm::ActiveValue::Set(quantity_book);
                item_update.quantity_difference = sea_orm::ActiveValue::Set(quantity_variance);
                item_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
                let _updated_item = item_update.update(&txn).await?;

                // 统计差异项数量
                if quantity_variance != rust_decimal::Decimal::ZERO {
                    variance_count += 1;

                    // 调整库存
                    let mut stock_update: inventory_stock::ActiveModel = stock_model.clone().into();
                    stock_update.quantity_on_hand = sea_orm::ActiveValue::Set(item.quantity_actual);
                    stock_update.quantity_available =
                        sea_orm::ActiveValue::Set(item.quantity_actual);
                    stock_update.quantity_meters =
                        sea_orm::ActiveValue::Set(stock_model.quantity_meters + quantity_variance);
                    stock_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
                    stock_update.update(&txn).await?;
                }
            } else {
                // 如果库存记录不存在，创建新记录
                let new_stock = inventory_stock::ActiveModel {
                    id: Default::default(),
                    warehouse_id: sea_orm::ActiveValue::Set(count.warehouse_id),
                    product_id: sea_orm::ActiveValue::Set(item.product_id),
                    quantity_on_hand: sea_orm::ActiveValue::Set(item.quantity_actual),
                    quantity_available: sea_orm::ActiveValue::Set(item.quantity_actual),
                    quantity_reserved: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                    quantity_incoming: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                    reorder_point: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                    reorder_quantity: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                    last_count_date: sea_orm::ActiveValue::NotSet,
                    last_movement_date: sea_orm::ActiveValue::NotSet,
                    created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                    updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                    // 面料行业特色字段 - 使用默认值
                    batch_no: sea_orm::ActiveValue::Set(String::new()),
                    color_no: sea_orm::ActiveValue::Set(String::new()),
                    dye_lot_no: sea_orm::ActiveValue::NotSet,
                    grade: sea_orm::ActiveValue::Set("一等品".to_string()),
                    production_date: sea_orm::ActiveValue::NotSet,
                    expiry_date: sea_orm::ActiveValue::NotSet,
                    quantity_meters: sea_orm::ActiveValue::Set(item.quantity_actual),
                    quantity_kg: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                    gram_weight: sea_orm::ActiveValue::NotSet,
                    width: sea_orm::ActiveValue::NotSet,
                    location_id: sea_orm::ActiveValue::NotSet,
                    shelf_no: sea_orm::ActiveValue::NotSet,
                    layer_no: sea_orm::ActiveValue::NotSet,
                    bin_location: sea_orm::ActiveValue::NotSet,
                    stock_status: sea_orm::ActiveValue::Set("正常".to_string()),
                    quality_status: sea_orm::ActiveValue::Set("合格".to_string()),
                };
                new_stock.insert(&txn).await?;
            }
        }

        // 更新盘点单状态
        let mut count_update: inventory_count::ActiveModel = count.into();
        count_update.status = sea_orm::ActiveValue::Set("completed".to_string());
        count_update.variance_items = sea_orm::ActiveValue::Set(variance_count);
        count_update.completed_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now()));
        count_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        count_update.update(&txn).await?;

        // 提交事务
        txn.commit().await?;

        // 返回盘点详情
        self.get_count_detail(count_id).await
    }

    /// 生成盘点单号
    async fn generate_count_no(&self) -> Result<String, sea_orm::DbErr> {
        let now = chrono::Utc::now();
        let date_str = now.format("%Y%m%d").to_string();

        // 获取当天最大盘点单号
        let max_count = InventoryCountEntity::find()
            .filter(inventory_count::Column::CountNo.like(format!("IC{}%", date_str)))
            .order_by(inventory_count::Column::CountNo, Order::Desc)
            .one(&*self.db)
            .await?;

        let seq = match max_count {
            Some(count) => {
                // 提取序号部分并加 1
                let seq_str = count
                    .count_no
                    .trim_start_matches(&format!("IC{}", date_str));
                seq_str.parse::<u32>().unwrap_or(0) + 1
            }
            None => 1,
        };

        Ok(format!("IC{}{:04}", date_str, seq))
    }
}
