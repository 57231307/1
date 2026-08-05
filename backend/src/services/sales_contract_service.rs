use crate::models::sales_contract;
use crate::models::sales_contract_item;
// 批次 210 P2-5 修复（v12 复审）：合同状态字符串替换为 contract 常量
use crate::models::status::contract;
use crate::utils::error::AppError;
use crate::utils::sql_escape::safe_like_pattern;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use std::sync::Arc;
use tracing::info;

/// 销售合同查询参数
#[derive(Debug, Clone, Default)]
pub struct SalesContractQueryParams {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub customer_id: Option<i32>,
    pub page: i64,
    pub page_size: i64,
}

/// 创建销售合同请求
#[derive(Debug, Clone)]
pub struct CreateSalesContractRequest {
    pub contract_no: String,
    pub contract_name: String,
    pub customer_id: i32,
    pub total_amount: Decimal,
    pub contract_type: Option<String>,
    pub payment_terms: Option<String>,
    pub delivery_date: NaiveDate,
    pub remark: Option<String>,
    /// 合同明细行
    pub items: Option<Vec<CreateContractItemRequest>>,
}

/// 创建合同明细行请求
#[derive(Debug, Clone)]
pub struct CreateContractItemRequest {
    pub product_id: Option<i32>,
    pub product_name: String,
    pub product_spec: Option<String>,
    pub unit: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub delivery_date: Option<NaiveDate>,
    pub remarks: Option<String>,
}

/// 合同执行请求
#[derive(Debug, Clone)]
pub struct ExecuteSalesContractRequest {
    pub execution_type: String,
    pub execution_amount: Decimal,
    pub related_bill_type: Option<String>,
    pub related_bill_id: Option<i32>,
    pub remark: Option<String>,
}

pub struct SalesContractService {
    db: Arc<DatabaseConnection>,
}

impl SalesContractService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 根据合同类型和金额计算印花税
    /// - 购销合同（sale / purchase_sale）：总金额 * 0.3‰
    /// - 加工承揽合同（processing / processing_contract）：总金额 * 0.5‰
    /// - 其他类型：不计算
    fn calculate_stamp_tax(
        contract_type: Option<&str>,
        total_amount: Decimal,
    ) -> Option<Decimal> {
        let rate = match contract_type? {
            "sale" | "purchase_sale" => Decimal::new(3, 4),      // 0.0003 = 0.3‰
            "processing" | "processing_contract" => Decimal::new(5, 4), // 0.0005 = 0.5‰
            _ => return None,
        };
        Some(total_amount * rate)
    }

    /// 创建销售合同
    pub async fn create(
        &self,
        req: CreateSalesContractRequest,
        user_id: i32,
    ) -> Result<sales_contract::Model, AppError> {
        info!("用户 {} 正在创建销售合同：{}", user_id, req.contract_no);

        let stamp_tax =
            Self::calculate_stamp_tax(req.contract_type.as_deref(), req.total_amount);

        // 使用事务确保合同和明细行原子创建
        let txn = self.db.begin().await?;

        let active_contract = sales_contract::ActiveModel {
            contract_no: Set(req.contract_no),
            contract_name: Set(req.contract_name),
            contract_type: Set(req.contract_type),
            customer_id: Set(req.customer_id),
            total_amount: Set(Some(req.total_amount)),
            status: Set("draft".to_string()),
            payment_terms: Set(req.payment_terms),
            delivery_date: Set(Some(req.delivery_date)),
            stamp_tax_amount: Set(stamp_tax),
            created_by: Set(user_id),
            ..Default::default()
        };

        let contract = active_contract.insert(&txn).await?;

        // 创建明细行
        if let Some(items) = req.items {
            for (idx, item) in items.iter().enumerate() {
                let amount = item.quantity * item.unit_price;
                let active_item = sales_contract_item::ActiveModel {
                    contract_id: Set(contract.id),
                    product_id: Set(item.product_id),
                    product_name: Set(item.product_name.clone()),
                    product_spec: Set(item.product_spec.clone()),
                    unit: Set(item.unit.clone()),
                    quantity: Set(item.quantity),
                    unit_price: Set(item.unit_price),
                    amount: Set(amount),
                    delivery_date: Set(item.delivery_date),
                    remarks: Set(item.remarks.clone()),
                    sort_order: Set(idx as i32),
                    ..Default::default()
                };
                active_item.insert(&txn).await?;
            }
        }

        txn.commit().await?;
        info!("销售合同创建成功：{}（含明细行）", contract.contract_no);
        Ok(contract)
    }

    /// 获取合同列表（分页）
    pub async fn get_list(
        &self,
        params: SalesContractQueryParams,
    ) -> Result<(Vec<sales_contract::Model>, u64), AppError> {
        let mut query = sales_contract::Entity::find();

        // 关键词筛选
        if let Some(keyword) = &params.keyword {
            let keyword_pattern = safe_like_pattern(keyword);
            query = query.filter(
                sales_contract::Column::ContractNo
                    .like(&keyword_pattern)
                    .or(sales_contract::Column::ContractName.like(&keyword_pattern)),
            );
        }

        // 状态筛选
        if let Some(status) = &params.status {
            query = query.filter(sales_contract::Column::Status.eq(status));
        }

        // 客户筛选
        if let Some(customer_id) = &params.customer_id {
            query = query.filter(sales_contract::Column::CustomerId.eq(*customer_id));
        }

        // 获取总数
        let total = query.clone().count(&*self.db).await?;

        // 分页和排序
        // 批次 24 v6 P1-2 修复：分页偏移 off-by-one。
        // 原代码 offset=(page.saturating_sub(1) * page_size)，当 page=1（HTTP 第一页）时 offset=page_size，
        // 跳过第一页数据。改为 ((page - 1) * page_size)，与 production_order_service.rs:279
        // 的 paginator.fetch_page(query.page - 1) 0-indexed 写法一致。
        let contracts = query
            .order_by(sales_contract::Column::Id, Order::Desc)
            // 批次 98 P2-A 修复（v5 复审）：page clamp 防 DoS
            .offset(((params.page.clamp(1, 1000).saturating_sub(1)) * params.page_size) as u64)
            .limit(params.page_size as u64)
            .all(&*self.db)
            .await?;

        Ok((contracts, total))
    }

    /// 获取合同详情
    pub async fn get_by_id(&self, id: i32) -> Result<sales_contract::Model, AppError> {
        let contract = sales_contract::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售合同不存在：{}", id)))?;
        Ok(contract)
    }

    /// 获取合同明细行
    pub async fn get_items(
        &self,
        contract_id: i32,
    ) -> Result<Vec<sales_contract_item::Model>, AppError> {
        let items = sales_contract_item::Entity::find()
            .filter(sales_contract_item::Column::ContractId.eq(contract_id))
            .order_by(sales_contract_item::Column::SortOrder, Order::Asc)
            .all(&*self.db)
            .await?;
        Ok(items)
    }

    /// 执行合同（出库或收款）
    pub async fn execute(
        &self,
        contract_id: i32,
        req: ExecuteSalesContractRequest,
        user_id: i32,
    ) -> Result<(), AppError> {
        info!(
            "用户 {} 正在执行销售合同 {}，类型：{}，金额：{}",
            user_id, contract_id, req.execution_type, req.execution_amount
        );

        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原实现先在事务外用 get_by_id 裸查询合同状态，再 begin() 开启事务，
        // 并发 execute 均通过状态检查后基于过期状态写入，导致状态门失效。
        let txn = (*self.db).begin().await?;

        // 获取合同（加 lock_exclusive 串行化并发状态变更）
        let contract = sales_contract::Entity::find_by_id(contract_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售合同 {}", contract_id)))?;

        // 检查合同状态
        if contract.status != contract::ACTIVE {
            return Err(AppError::validation(
                "只有活跃状态的合同才能执行".to_string(),
            ));
        }

        // 验证执行类型
        match req.execution_type.as_str() {
            "delivery" | "payment" => {}
            _ => {
                return Err(AppError::validation(
                    "无效的执行类型，支持：delivery（出库）、payment（收款）".to_string(),
                ))
            }
        }

        // 更新合同状态
        let mut contract_active: sales_contract::ActiveModel = contract.into();
        contract_active.updated_at = Set(chrono::Utc::now());

        contract_active.save(&txn).await?;

        // 记录执行日志（这里可以扩展为创建执行记录表）
        info!(
            "合同执行记录：合同ID={}，类型={}，金额={}，关联单据类型={}，关联单据ID={:?}",
            contract_id,
            req.execution_type,
            req.execution_amount,
            req.related_bill_type.as_deref().unwrap_or("无"),
            req.related_bill_id
        );

        // 提交事务
        txn.commit().await?;

        info!(
            "销售合同 {} 执行成功，执行金额：{}",
            contract_id, req.execution_amount
        );
        Ok(())
    }

    /// 审核合同
    /// 批次 22（2026-06-28 v5 P0-6）：重构 approve 补全事务边界 + lock_exclusive + update_with_audit；原 `approve` 在 `&*self.db` 上裸查询 + 裸 `save`，无事务边界也无行锁，；并发审核同一合同可能基于过期快照导致状态覆盖；同时未走 update_with_audit 会丢失审计追溯。；改为：begin txn + lock_exclusive 查询 + 状态校验 + update_with_audit(&txn, Some(user_id)) + commit。
    pub async fn approve(&self, contract_id: i32, user_id: i32) -> Result<(), AppError> {
        info!("用户 {} 正在审核销售合同 {}", user_id, contract_id);

        let txn = (*self.db).begin().await?;

        // 状态门查询加 lock_exclusive 串行化并发 approve
        let contract = sales_contract::Entity::find_by_id(contract_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售合同 {}", contract_id)))?;

        if contract.status != contract::DRAFT {
            return Err(AppError::business(format!(
                "合同状态为{}，不可审核（仅草稿状态可审核）",
                contract.status
            )));
        }

        let mut contract_active: sales_contract::ActiveModel = contract.into();
        contract_active.status = Set(contract::ACTIVE.to_string());
        contract_active.updated_at = Set(chrono::Utc::now());

        // 走 update_with_audit 保留审计追溯
        // P2-3 修复（批次 84 v1 复审）：有意忽略返回的 ActiveModel（字段已通过 Set 表达更新意图），仅传播错误
        // 批次 94 P2-11：审计日志为关键路径，错误已通过 ? 传播；去掉 let _ = 直接丢弃 ActiveModel 返回值
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            contract_active,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        info!("销售合同 {} 审核成功", contract_id);
        Ok(())
    }

    /// 取消合同
    /// 批次 22（2026-06-28 v5 P0-6）：重构 cancel 补全事务边界 + lock_exclusive + update_with_audit；原 `cancel` 在 `&*self.db` 上裸查询 + 裸 `save`，无事务边界也无行锁，；并发取消同一合同可能基于过期快照导致状态覆盖；同时未走 update_with_audit 会丢失审计追溯。；改为：begin txn + lock_exclusive 查询 + 状态校验 + update_with_audit(&txn, Some(user_id)) + commit。
    pub async fn cancel(
        &self,
        contract_id: i32,
        user_id: i32,
        reason: String,
    ) -> Result<(), AppError> {
        info!(
            "用户 {} 正在取消销售合同 {}，原因：{}",
            user_id, contract_id, reason
        );

        let txn = (*self.db).begin().await?;

        // 状态门查询加 lock_exclusive 串行化并发 cancel
        let contract = sales_contract::Entity::find_by_id(contract_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售合同 {}", contract_id)))?;

        if contract.status != contract::ACTIVE && contract.status != contract::DRAFT {
            return Err(AppError::business(format!(
                "合同状态为{}，不可取消（仅活跃或草稿状态可取消）",
                contract.status
            )));
        }

        let mut contract_active: sales_contract::ActiveModel = contract.into();
        contract_active.status = Set(contract::CANCELLED.to_string());
        contract_active.updated_at = Set(chrono::Utc::now());

        // 走 update_with_audit 保留审计追溯
        // P2-3 修复（批次 84 v1 复审）：有意忽略返回的 ActiveModel（字段已通过 Set 表达更新意图），仅传播错误
        // 批次 94 P2-11：审计日志为关键路径，错误已通过 ? 传播；去掉 let _ = 直接丢弃 ActiveModel 返回值
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            contract_active,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        info!("销售合同 {} 取消成功", contract_id);
        Ok(())
    }
}
