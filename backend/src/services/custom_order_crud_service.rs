//! 定制订单 CRUD 服务
//!
//! 提供定制订单基础 CRUD 业务：create / list / get_by_id / update / cancel
//! 工艺推进由 custom_order_state_service 处理
//! 创建时间: 2026-06-17

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};
use std::sync::Arc;
use thiserror::Error;

use crate::models::custom_order::{self, ActiveModel as CustomOrderActive, Entity as CustomOrderEntity};
use crate::models::custom_order_create_dto::{CancelCustomOrderDto, CreateCustomOrderDto, UpdateCustomOrderDto};
use crate::models::process_node::{self, ActiveModel as NodeActive, Entity as NodeEntity};
use crate::models::status::custom_order as co_status;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;
use crate::utils::process_state_machine::default_process_nodes;

/// 业务错误
#[derive(Debug, Error)]
pub enum CrudError {
    #[error("定制订单不存在")]
    NotFound,
    #[error("当前状态不允许此操作")]
    InvalidState,
    #[error("参数校验失败: {0}")]
    Validation(String),
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
    /// 批次 263：接入 paginate_with_total（返回 AppError）所需的错误转换
    #[error("应用错误: {0}")]
    App(#[from] AppError),
}

/// 定制订单 CRUD 服务
pub struct CustomOrderCrudService {
    db: Arc<DatabaseConnection>,
}

impl CustomOrderCrudService {
    /// 从数据库连接构造
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 从 AppState 构造
    pub fn from_state(state: &AppState) -> Self {
        Self {
            db: state.db.clone(),
        }
    }

    /// 创建定制订单草稿（自动生成 5 阶段工艺节点）
    pub async fn create_draft(
        &self,
        dto: CreateCustomOrderDto,
        user_id: i64,
    ) -> Result<custom_order::Model, CrudError> {
        // 1. 业务校验
        self.validate_create(&dto)?;

        // 2. 生成 order_no（CO + YYYYMMDD + 4 位序号）
        let order_no = self.generate_order_no().await?;

        // 3. 开始事务
        let txn = self.db.begin().await?;

        // 4. 插入主表
        let now = Utc::now();
        let active = Self::build_custom_order_active(order_no, dto, user_id, now);
        let result = active.insert(&txn).await?;

        // 5. 自动生成 5 阶段工艺节点
        for (node_type, node_name, sequence) in default_process_nodes() {
            let node = Self::build_process_node_active(result.id, node_type, node_name, sequence, now);
            node.insert(&txn).await?;
        }

        // 6. 提交事务
        txn.commit().await?;
        Ok(result)
    }

    /// 构建定制订单主表 ActiveModel（draft 状态）
    fn build_custom_order_active(
        order_no: String,
        dto: CreateCustomOrderDto,
        user_id: i64,
        now: chrono::DateTime<chrono::Utc>,
    ) -> CustomOrderActive {
        CustomOrderActive {
            id: Default::default(),
            order_no: Set(order_no),
            customer_id: Set(dto.customer_id),
            product_id: Set(dto.product_id),
            color_id: Set(dto.color_id),
            spec: Set(dto.spec),
            quantity: Set(dto.quantity),
            unit: Set(dto.unit),
            custom_requirements: Set(dto
                .custom_requirements
                .clone()
                .unwrap_or(serde_json::json!({}))),
            yarn_spec: Set(dto.yarn_spec),
            dye_method: Set(dto.dye_method),
            finishing_method: Set(dto.finishing_method),
            status: Set(co_status::DRAFT.to_string()),
            expected_delivery_date: Set(dto.expected_delivery_date),
            actual_delivery_date: Set(None),
            sales_order_id: Set(dto.sales_order_id),
            total_amount: Set(dto.total_amount),
            currency: Set(dto.currency.unwrap_or_else(|| crate::constants::DEFAULT_CURRENCY.to_string())),
            created_by: Set(Some(user_id)),
            created_at: Set(now),
            updated_at: Set(now),
            // 批次 88 PH-1 占位符实现：消费 DTO notes 字段，持久化到 custom_orders.notes
            notes: Set(dto.notes),
            // V15 P0-B11：打样和报价关联字段初始化为 None（draft 阶段尚未关联）
            lab_dip_request_id: Set(None),
            quotation_id: Set(None),
        }
    }

    /// 构建工艺节点 ActiveModel（pending 状态）
    fn build_process_node_active(
        custom_order_id: i64,
        node_type: &str,
        node_name: &str,
        sequence: i32,
        now: chrono::DateTime<chrono::Utc>,
    ) -> NodeActive {
        NodeActive {
            id: Default::default(),
            custom_order_id: Set(custom_order_id),
            node_type: Set(node_type.to_string()),
            node_name: Set(node_name.to_string()),
            sequence: Set(sequence),
            status: Set(co_status::PENDING.to_string()),
            planned_start_date: Set(None),
            planned_end_date: Set(None),
            actual_start_date: Set(None),
            actual_end_date: Set(None),
            operator_id: Set(None),
            notes: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
    }

    /// 列表查询（分页 + 过滤）
    ///
    /// 批次 263 修复：接入 paginate_with_total 工具函数，消除手写 num_items + fetch_page 重复。
    /// paginate_with_total 内部已做 page.saturating_sub(1) 偏移，调用方不可再减 1。
    /// 补 clamp(1, 1000) 防 DoS（恶意请求 page=999999 不会导致超大偏移查询）。
    pub async fn list(
        &self,
        page: u64,
        page_size: u64,
        status: Option<String>,
        customer_id: Option<i64>,
        keyword: Option<String>,
    ) -> Result<(Vec<custom_order::Model>, u64), CrudError> {
        let mut query = CustomOrderEntity::find();

        if let Some(s) = status {
            query = query.filter(custom_order::Column::Status.eq(s));
        }
        if let Some(c) = customer_id {
            query = query.filter(custom_order::Column::CustomerId.eq(c));
        }
        if let Some(k) = keyword {
            let pattern = format!("%{}%", k);
            query = query.filter(custom_order::Column::OrderNo.like(pattern));
        }

        let paginator = query
            .order_by_desc(custom_order::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;

        Ok((items, total))
    }

    /// 按 ID 查询
    pub async fn get_by_id(
        &self,
        id: i64,
    ) -> Result<custom_order::Model, CrudError> {
        CustomOrderEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or(CrudError::NotFound)
    }

    /// 更新定制订单（仅 draft 状态可更新）
    ///
    /// 批次 85 v2 复审 P1-1 修复：状态门 + update 移入单一事务 + lock_exclusive 串行化
    /// 原实现状态门在 self.db 查询、update 也在 self.db，无 txn 无 lock，存在 TOCTOU
    /// （并发 update 与 cancel 会基于过期状态通过检查后重复写入）
    pub async fn update(
        &self,
        id: i64,
        dto: UpdateCustomOrderDto,
    ) -> Result<custom_order::Model, CrudError> {
        let txn = self.db.begin().await?;

        // 加 lock_exclusive 串行化并发状态变更
        let existing = CustomOrderEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(CrudError::NotFound)?;

        if existing.status != co_status::DRAFT {
            return Err(CrudError::InvalidState);
        }

        let mut active: CustomOrderActive = existing.into();
        if let Some(v) = dto.spec {
            active.spec = Set(v);
        }
        if let Some(v) = dto.quantity {
            active.quantity = Set(v);
        }
        if let Some(v) = dto.unit {
            active.unit = Set(v);
        }
        if let Some(v) = dto.custom_requirements {
            active.custom_requirements = Set(v);
        }
        if let Some(v) = dto.yarn_spec {
            active.yarn_spec = Set(Some(v));
        }
        if let Some(v) = dto.dye_method {
            active.dye_method = Set(Some(v));
        }
        if let Some(v) = dto.finishing_method {
            active.finishing_method = Set(Some(v));
        }
        if let Some(v) = dto.expected_delivery_date {
            active.expected_delivery_date = Set(Some(v));
        }
        if let Some(v) = dto.total_amount {
            active.total_amount = Set(Some(v));
        }
        if let Some(v) = dto.notes {
            // 批次 88 PH-1 占位符实现：持久化 notes 到 custom_orders.notes 列
            active.notes = Set(Some(v));
        }
        active.updated_at = Set(Utc::now());
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// 取消定制订单（任意非终态可取消）
    pub async fn cancel(
        &self,
        id: i64,
        dto: CancelCustomOrderDto,
        _user_id: i64,
    ) -> Result<custom_order::Model, CrudError> {
        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原实现无事务、无行锁，并发 cancel 会基于过期状态通过状态检查后重复写入。
        let txn = self.db.begin().await?;

        // 加 lock_exclusive 串行化并发状态变更
        let existing = CustomOrderEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(CrudError::NotFound)?;

        if existing.status == co_status::COMPLETED || existing.status == co_status::CANCELLED {
            return Err(CrudError::InvalidState);
        }

        let existing_notes = existing.notes.clone();
        let mut active: CustomOrderActive = existing.into();
        active.status = Set(co_status::CANCELLED.to_string());
        active.updated_at = Set(Utc::now());
        // 批次 94 P2-14 修复：将 dto.reason 记录到 notes 字段（原 let _ = dto.reason 占位丢弃）
        // 追加取消原因到现有 notes（保留原有备注，避免覆盖）
        let new_notes = match existing_notes {
            Some(n) if !n.is_empty() => format!("{}\n取消原因: {}", n, dto.reason),
            _ => format!("取消原因: {}", dto.reason),
        };
        active.notes = Set(Some(new_notes));
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// 列出指定订单的工艺节点
    pub async fn list_process_nodes(
        &self,
        custom_order_id: i64,
    ) -> Result<Vec<process_node::Model>, CrudError> {
        let nodes = NodeEntity::find()
            .filter(process_node::Column::CustomOrderId.eq(custom_order_id))
            .order_by_asc(process_node::Column::Sequence)
            .all(&*self.db)
            .await?;
        Ok(nodes)
    }

    // ----------------------------------------------------------------------
    // 私有辅助
    // ----------------------------------------------------------------------

    async fn generate_order_no(&self) -> Result<String, CrudError> {
        let today = Utc::now().format("%Y%m%d").to_string();
        let pattern = format!("CO{}%", today);
        let count = CustomOrderEntity::find()
            .filter(custom_order::Column::OrderNo.like(pattern))
            .count(&*self.db)
            .await?;
        Ok(format!("CO{}{:04}", today, count + 1))
    }

    fn validate_create(&self, dto: &CreateCustomOrderDto) -> Result<(), CrudError> {
        if dto.quantity <= rust_decimal::Decimal::ZERO {
            return Err(CrudError::Validation("数量必须大于 0".to_string()));
        }
        if dto.spec.is_empty() {
            return Err(CrudError::Validation("规格不能为空".to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    /// 辅助函数：构造最小有效 CreateCustomOrderDto，便于测试 notes 字段透传
    fn make_test_dto(notes: Option<String>) -> CreateCustomOrderDto {
        CreateCustomOrderDto {
            customer_id: 1,
            product_id: 1,
            color_id: None,
            spec: "测试规格".to_string(),
            quantity: Decimal::from(100),
            unit: "m".to_string(),
            custom_requirements: None,
            yarn_spec: None,
            dye_method: None,
            finishing_method: None,
            expected_delivery_date: None,
            sales_order_id: None,
            total_amount: None,
            currency: None,
            notes,
        }
    }

    /// 测试 CreateCustomOrderDto 的 notes 字段类型为 Option<String> 且透传正确
    ///
    /// DTO 字段透传验证，完整 create_draft 流程需集成测试
    /// （create_draft line 96 `notes: Set(dto.notes)` 将 DTO notes 持久化到 custom_orders.notes）
    #[test]
    fn test_notes_field_in_create_dto() {
        let dto = make_test_dto(Some("客户备注内容".to_string()));
        // 显式标注 Option<String> 类型，编译期验证 notes 字段类型正确
        let notes: Option<String> = dto.notes;
        assert_eq!(notes, Some("客户备注内容".to_string()));
    }

    /// 测试 notes=None 时 DTO 字段为 None，create_draft 会将 None 写入 custom_orders.notes
    ///
    /// DTO 字段透传验证，完整 create_draft 流程需集成测试
    #[test]
    fn test_notes_default_when_none() {
        let dto = make_test_dto(None);
        assert_eq!(dto.notes, None);
    }
}
