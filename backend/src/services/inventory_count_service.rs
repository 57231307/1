#![allow(dead_code)]

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, TransactionTrait,
};
use std::sync::Arc;

use crate::utils::error::AppError;
use crate::models::dto::PageRequest;
use crate::utils::number_generator::DocumentNumberGenerator;
use crate::models::inventory_count::{self, Entity as InventoryCountEntity};
use crate::models::inventory_count_item::{self, Entity as InventoryCountItemEntity};
use crate::models::inventory_stock::{self, Entity as InventoryStockEntity};
use crate::models::inventory_transaction;
use crate::services::inventory_adjustment_service::{
    AdjustmentItemRequest, CreateAdjustmentRequest, InventoryAdjustmentService,
};
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
    pub status: Option<String>,
    pub notes: Option<String>,
    pub items: Option<Vec<InventoryCountItemRequest>>,
}

#[derive(Debug, Deserialize)]
pub struct InventoryCountItemRequest {
    pub product_id: i32,
    pub stock_id: Option<i32>,
    pub warehouse_id: Option<i32>,
    pub quantity_actual: rust_decimal::Decimal,
    pub unit_cost: Option<rust_decimal::Decimal>,
    pub notes: Option<String>,
}

/// 更新库存盘点请求
#[derive(Debug, Deserialize)]
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
    ) -> Result<PaginatedResponse<InventoryCountDetail>, AppError> {
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
        let counts: Vec<inventory_count::Model> =
            paginator.fetch_page(page_req.page - 1).await?;

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
    ) -> Result<InventoryCountDetail, AppError> {
        // 获取盘点主表数据
        let count = InventoryCountEntity::find_by_id(count_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::ResourceNotFound(format!("库存盘点单 {} 未找到", count_id))
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
    ) -> Result<InventoryCountDetail, AppError> {
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
            tracing::error!("Transaction rolled back: 盘点单号 {} 已存在", count_no);
            txn.rollback().await?;
            return Err(AppError::BusinessError("盘点单号已存在，请重试".to_string()));
        }

        // 创建盘点主表
        let count = inventory_count::ActiveModel {
            id: Default::default(),
            count_no: sea_orm::ActiveValue::Set(count_no),
            warehouse_id: sea_orm::ActiveValue::Set(request.warehouse_id),
            count_date: sea_orm::ActiveValue::Set(
                request.count_date.unwrap_or_else(chrono::Utc::now),
            ),
            status: sea_orm::ActiveValue::Set(request.status.unwrap_or_else(|| "pending".to_string())),
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
                    stock_id: sea_orm::ActiveValue::Set(item_req.stock_id.unwrap_or(0)),
                    warehouse_id: sea_orm::ActiveValue::Set(item_req.warehouse_id.unwrap_or(request.warehouse_id)),
                    quantity_before: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                    quantity_actual: sea_orm::ActiveValue::Set(item_req.quantity_actual),
                    quantity_difference: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                    unit_cost: sea_orm::ActiveValue::Set(item_req.unit_cost.unwrap_or(rust_decimal::Decimal::ZERO)),
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
        crate::services::audit_log_service::AuditLogService::update_with_audit(&txn, "auto_audit", count_update, Some(0)).await?;

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
    ) -> Result<InventoryCountDetail, AppError> {
        // 检查盘点单是否存在
        let count = InventoryCountEntity::find_by_id(count_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::ResourceNotFound(format!("库存盘点单 {} 未找到", count_id))
            })?;

        // 检查状态，已完成的盘点单不允许修改
        if count.status == "completed" {
            return Err(AppError::BusinessError(
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
        crate::services::audit_log_service::AuditLogService::update_with_audit(&txn, "auto_audit", count_update, Some(0)).await?;

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
    ) -> Result<InventoryCountDetail, AppError> {
        // 检查盘点单是否存在
        let count = InventoryCountEntity::find_by_id(count_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::ResourceNotFound(format!("库存盘点单 {} 未找到", count_id))
            })?;

        // 检查状态，只有待审核的盘点单可以审核
        if count.status != "pending" {
            return Err(AppError::BusinessError(
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
        crate::services::audit_log_service::AuditLogService::update_with_audit(&txn, "auto_audit", count_update, Some(0)).await?;

        // 提交事务
        txn.commit().await?;

        // 返回盘点详情
        self.get_count_detail(count_id).await
    }

    /// 完成库存盘点并调整库存
    pub async fn complete_count(
        &self,
        count_id: i32,
    ) -> Result<InventoryCountDetail, AppError> {
        // 检查盘点单是否存在
        let count = InventoryCountEntity::find_by_id(count_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::ResourceNotFound(format!("库存盘点单 {} 未找到", count_id))
            })?;

        // 检查状态，只有已审核的盘点单可以完成
        if count.status != "approved" {
            return Err(AppError::BusinessError(
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

        // 批量获取库存记录（优化N+1查询）
        let product_ids: Vec<i32> = items.iter().map(|item| item.product_id).collect();
        let stocks = InventoryStockEntity::find()
            .filter(inventory_stock::Column::WarehouseId.eq(count.warehouse_id))
            .filter(inventory_stock::Column::ProductId.is_in(product_ids))
            .all(&txn)
            .await?;
        let stock_map: std::collections::HashMap<i32, inventory_stock::Model> = stocks.into_iter().map(|s| (s.product_id, s)).collect();

        // 处理每个盘点明细项
        for item in items {
            // 查找对应库存记录
            let stock = stock_map.get(&item.product_id);

            if let Some(stock_model) = stock {
                // 获取账面数量
                let quantity_book = stock_model.quantity_on_hand;
                let stock_model = stock_model.clone();

                // 计算差异
                let quantity_variance = item.quantity_actual - quantity_book;

                // 更新明细项
                let mut item_update: inventory_count_item::ActiveModel = item.clone().into();
                item_update.quantity_before = sea_orm::ActiveValue::Set(quantity_book);
                item_update.quantity_difference = sea_orm::ActiveValue::Set(quantity_variance);
                item_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
                let _updated_item = crate::services::audit_log_service::AuditLogService::update_with_audit(&txn, "auto_audit", item_update, Some(0)).await?;

                // 统计差异项数量
                if quantity_variance != rust_decimal::Decimal::ZERO {
                    variance_count += 1;

                    // 调整库存
                    let mut stock_update: inventory_stock::ActiveModel = stock_model.clone().into();
                    stock_update.quantity_on_hand = sea_orm::ActiveValue::Set(item.quantity_actual);
                    stock_update.quantity_available =
                        sea_orm::ActiveValue::Set(item.quantity_actual);
                    stock_update.quantity_meters = sea_orm::ActiveValue::Set(
                        stock_model.quantity_meters + quantity_variance,
                    );
                    // Update quantity_kg proportionally
                    if stock_model.quantity_meters > rust_decimal::Decimal::ZERO {
                        let kg_ratio = stock_model.quantity_kg / stock_model.quantity_meters;
                        stock_update.quantity_kg = sea_orm::ActiveValue::Set(
                            item.quantity_actual * kg_ratio,
                        );
                    }
                    stock_update.version = sea_orm::ActiveValue::Set(stock_model.version + 1);
                    stock_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
                    crate::services::audit_log_service::AuditLogService::update_with_audit(&txn, "auto_audit", stock_update, Some(0)).await?;

                    // 记录盘点调整流水
                    let transaction = inventory_transaction::ActiveModel {
                        id: sea_orm::ActiveValue::NotSet,
                        transaction_type: sea_orm::ActiveValue::Set("COUNT_ADJUSTMENT".to_string()),
                        product_id: sea_orm::ActiveValue::Set(item.product_id),
                        warehouse_id: sea_orm::ActiveValue::Set(count.warehouse_id),
                        batch_no: sea_orm::ActiveValue::Set(stock_model.batch_no.clone()),
                        color_no: sea_orm::ActiveValue::Set(stock_model.color_no.clone()),
                        dye_lot_no: sea_orm::ActiveValue::Set(stock_model.dye_lot_no.clone()),
                        grade: sea_orm::ActiveValue::Set(stock_model.grade.clone()),
                        quantity_meters: sea_orm::ActiveValue::Set(quantity_variance),
                        quantity_kg: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                        source_bill_type: sea_orm::ActiveValue::Set(Some("inventory_count".to_string())),
                        source_bill_no: sea_orm::ActiveValue::Set(Some(count.count_no.clone())),
                        source_bill_id: sea_orm::ActiveValue::Set(Some(count.id)),
                        quantity_before_meters: sea_orm::ActiveValue::Set(Some(stock_model.quantity_meters)),
                        quantity_before_kg: sea_orm::ActiveValue::Set(Some(stock_model.quantity_kg)),
                        quantity_after_meters: sea_orm::ActiveValue::Set(Some(stock_model.quantity_meters + quantity_variance)),
                        quantity_after_kg: sea_orm::ActiveValue::Set(Some(stock_model.quantity_kg)),
                        notes: sea_orm::ActiveValue::Set(Some("盘点调整".to_string())),
                        created_by: sea_orm::ActiveValue::Set(count.created_by),
                        created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                    };
                    transaction.insert(&txn).await?;
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
                    version: sea_orm::ActiveValue::Set(0),
                };
                new_stock.insert(&txn).await?;

                // 记录盘点调整流水（新建库存记录）
                let transaction = inventory_transaction::ActiveModel {
                    id: sea_orm::ActiveValue::NotSet,
                    transaction_type: sea_orm::ActiveValue::Set("COUNT_ADJUSTMENT".to_string()),
                    product_id: sea_orm::ActiveValue::Set(item.product_id),
                    warehouse_id: sea_orm::ActiveValue::Set(count.warehouse_id),
                    batch_no: sea_orm::ActiveValue::Set(String::new()),
                    color_no: sea_orm::ActiveValue::Set(String::new()),
                    dye_lot_no: sea_orm::ActiveValue::NotSet,
                    grade: sea_orm::ActiveValue::Set("一等品".to_string()),
                    quantity_meters: sea_orm::ActiveValue::Set(item.quantity_actual),
                    quantity_kg: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                    source_bill_type: sea_orm::ActiveValue::Set(Some("inventory_count".to_string())),
                    source_bill_no: sea_orm::ActiveValue::Set(Some(count.count_no.clone())),
                    source_bill_id: sea_orm::ActiveValue::Set(Some(count.id)),
                    quantity_before_meters: sea_orm::ActiveValue::Set(Some(rust_decimal::Decimal::ZERO)),
                    quantity_before_kg: sea_orm::ActiveValue::Set(Some(rust_decimal::Decimal::ZERO)),
                    quantity_after_meters: sea_orm::ActiveValue::Set(Some(item.quantity_actual)),
                    quantity_after_kg: sea_orm::ActiveValue::Set(Some(rust_decimal::Decimal::ZERO)),
                    notes: sea_orm::ActiveValue::Set(Some("盘点调整（新建库存记录）".to_string())),
                    created_by: sea_orm::ActiveValue::Set(count.created_by),
                    created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                };
                transaction.insert(&txn).await?;
            }
        }

        // 更新盘点单状态
        let count_no = count.count_no.clone();
        let warehouse_id = count.warehouse_id;
        let created_by = count.created_by;
        let mut count_update: inventory_count::ActiveModel = count.into();
        count_update.status = sea_orm::ActiveValue::Set("completed".to_string());
        count_update.variance_items = sea_orm::ActiveValue::Set(variance_count);
        count_update.completed_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now()));
        count_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(&txn, "auto_audit", count_update, Some(0)).await?;

        // 提交事务
        txn.commit().await?;

        // 如果存在差异，调用 InventoryAdjustmentService 创建正式的调整单
        if variance_count > 0 {
            let adjustment_service = InventoryAdjustmentService::new(self.db.clone());
            
            // 重新获取盘点明细项以创建调整单
            let items = InventoryCountItemEntity::find()
                .filter(inventory_count_item::Column::CountId.eq(count_id))
                .all(&*self.db)
                .await?;

            // 收集调整项
            let mut adjustment_items = Vec::new();
            for item in items {
                if item.quantity_difference != rust_decimal::Decimal::ZERO {
                    adjustment_items.push(AdjustmentItemRequest {
                        stock_id: item.stock_id,
                        quantity: item.quantity_difference.abs(),
                        unit_cost: Some(item.unit_cost),
                        notes: Some(format!("盘点差异调整 - 盘点单号: {}", count_no)),
                    });
                }
            }

            // 创建调整单
            if !adjustment_items.is_empty() {
                let adjustment_request = CreateAdjustmentRequest {
                    warehouse_id,
                    adjustment_date: chrono::Utc::now(),
                    adjustment_type: if adjustment_items.iter().any(|i| i.quantity > rust_decimal::Decimal::ZERO) {
                        "increase".to_string()
                    } else {
                        "decrease".to_string()
                    },
                    reason_type: "correction".to_string(),
                    reason_description: Some(format!("盘点差异自动调整 - 盘点单号: {}", count_no)),
                    notes: Some("由盘点单自动生成".to_string()),
                    created_by,
                    items: adjustment_items,
                };

                adjustment_service.create_adjustment(adjustment_request).await.map_err(|e| {
                    tracing::warn!("创建盘点差异调整单失败: {}", e);
                    e
                })?;
            }
        }

        // 返回盘点详情
        self.get_count_detail(count_id).await
    }

    /// 生成盘点单号
    async fn generate_count_no(&self) -> Result<String, AppError> {
        DocumentNumberGenerator::generate_no(
            &*self.db,
            "IC",
            inventory_count::Entity,
            inventory_count::Column::CountNo,
        )
        .await
        .map_err(|e| AppError::BusinessError(e.to_string()))
    }
}
