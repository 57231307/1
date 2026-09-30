use crate::models::sales_contract;
use crate::models::sales_contract_item;
// 批次 210 P2-5 修复（v12 复审）：合同状态字符串替换为 contract 常量
use crate::models::status::contract;
use crate::utils::error::AppError;
use crate::utils::sql_escape::safe_like_pattern;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ExprTrait, Order,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
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
///
/// - `delivery_date` 为 Option：真实列 sales_contracts.delivery_date 可空（m0011 DDL）。
/// - 表头真实列 signed_date/effective_date/expiry_date/payment_method/delivery_location
///   与 remark 全量落库（remark 列由 m0016 迁移补齐）。
#[derive(Debug, Clone)]
pub struct CreateSalesContractRequest {
    pub contract_no: String,
    pub contract_name: String,
    pub customer_id: i32,
    pub total_amount: Decimal,
    pub contract_type: Option<String>,
    pub payment_terms: Option<String>,
    pub delivery_date: Option<NaiveDate>,
    pub signed_date: Option<NaiveDate>,
    pub effective_date: Option<NaiveDate>,
    pub expiry_date: Option<NaiveDate>,
    pub payment_method: Option<String>,
    pub delivery_location: Option<String>,
    pub remark: Option<String>,
    /// 合同明细行
    pub items: Option<Vec<CreateContractItemRequest>>,
}

/// 更新销售合同请求
///
/// 字段语义 = PATCH 部分更新（Some=覆盖，None=保持原值），items=明细整表替换。
/// 不含 contract_no：合同编号由系统生成（单据号禁手打口径），更新链路不允许改写；
/// 若放开改写，改成已存在编号会直接撞 sales_contracts.contract_no UNIQUE（无 23505→4xx
/// 自动映射，只会裸 500），且篡改单据号本身违反编号即身份的业务规则。
#[derive(Debug, Clone, Default)]
pub struct UpdateSalesContractRequest {
    pub contract_name: Option<String>,
    pub customer_id: Option<i32>,
    pub total_amount: Option<Decimal>,
    pub contract_type: Option<String>,
    pub payment_terms: Option<String>,
    pub delivery_date: Option<NaiveDate>,
    pub signed_date: Option<NaiveDate>,
    pub effective_date: Option<NaiveDate>,
    pub expiry_date: Option<NaiveDate>,
    pub payment_method: Option<String>,
    pub delivery_location: Option<String>,
    pub remark: Option<String>,
    /// Some(items)：整表替换明细行；None：不动明细
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
    /// 交货数量允收容差（百分比，可空）：NULL = 走默认解析，非空 = 行级覆盖。
    pub quantity_tolerance_pct: Option<Decimal>,
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
    fn calculate_stamp_tax(contract_type: Option<&str>, total_amount: Decimal) -> Option<Decimal> {
        let rate = match contract_type? {
            "sale" | "purchase_sale" => Decimal::new(3, 4), // 0.0003 = 0.3‰
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

        let stamp_tax = Self::calculate_stamp_tax(req.contract_type.as_deref(), req.total_amount);

        // 使用事务确保合同和明细行原子创建
        let txn = self.db.begin().await?;

        // P0 契约修复：校验客户存在并回填冗余列 customer_name
        //（原实现既不校验也不回填 ⇒ 悬挂 customer_id 可入库、列表/详情客户名恒空）
        let customer = crate::models::customer::Entity::find_by_id(req.customer_id)
            .one(&txn)
            .await?
            .ok_or_else(|| {
                AppError::business_displayable("创建失败：所选客户不存在，请重新选择客户")
            })?;

        let active_contract = sales_contract::ActiveModel {
            contract_no: Set(req.contract_no),
            contract_name: Set(req.contract_name),
            contract_type: Set(req.contract_type),
            customer_id: Set(req.customer_id),
            customer_name: Set(Some(customer.customer_name)),
            total_amount: Set(Some(req.total_amount)),
            status: Set(contract::DRAFT.to_string()),
            payment_terms: Set(req.payment_terms),
            delivery_date: Set(req.delivery_date),
            // P0 契约修复：补齐真实列表头字段（原三处不一致中被 service 吞掉的一组）
            signed_date: Set(req.signed_date),
            effective_date: Set(req.effective_date),
            expiry_date: Set(req.expiry_date),
            payment_method: Set(req.payment_method),
            delivery_location: Set(req.delivery_location),
            remark: Set(req.remark),
            stamp_tax_amount: Set(stamp_tax),
            created_by: Set(user_id),
            ..Default::default()
        };

        let contract = active_contract.insert(&txn).await?;

        // 创建明细行
        if let Some(items) = &req.items {
            Self::insert_items_txn(&txn, contract.id, items).await?;
        }

        txn.commit().await?;
        info!("销售合同创建成功：{}（含明细行）", contract.contract_no);
        Ok(contract)
    }

    /// 事务内批量插入合同明细行（create 与 update 共用，金额=数量×单价同源）
    async fn insert_items_txn(
        txn: &sea_orm::DatabaseTransaction,
        contract_id: i32,
        items: &[CreateContractItemRequest],
    ) -> Result<(), AppError> {
        for (idx, item) in items.iter().enumerate() {
            let amount = item.quantity * item.unit_price;
            let active_item = sales_contract_item::ActiveModel {
                contract_id: Set(contract_id),
                product_id: Set(item.product_id),
                product_name: Set(item.product_name.clone()),
                product_spec: Set(item.product_spec.clone()),
                unit: Set(item.unit.clone()),
                quantity: Set(item.quantity),
                quantity_tolerance_pct: Set(item.quantity_tolerance_pct),
                unit_price: Set(item.unit_price),
                amount: Set(amount),
                delivery_date: Set(item.delivery_date),
                remarks: Set(item.remarks.clone()),
                sort_order: Set(idx as i32),
                ..Default::default()
            };
            active_item.insert(txn).await?;
        }
        Ok(())
    }

    /// 更新销售合同（表头 + 明细整表替换）
    ///
    /// P0 契约修复（本轮）：原 handler 内联实现只接受 contract_name/payment_terms，
    /// 且 get_by_id 裸查询 + update 无事务边界。现改为：
    /// begin txn + lock_exclusive + DRAFT 状态门 + 表头字段 Some=覆盖 + items 整表替换 + commit。
    pub async fn update(
        &self,
        id: i32,
        req: UpdateSalesContractRequest,
        user_id: i32,
    ) -> Result<sales_contract::Model, AppError> {
        info!("用户 {} 正在更新销售合同 {}", user_id, id);

        let txn = (*self.db).begin().await?;

        let contract = sales_contract::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售合同不存在：{}", id)))?;

        if contract.status != contract::DRAFT {
            return Err(AppError::business_displayable(
                "只有草稿状态的销售合同才能修改",
            ));
        }

        // 客户变更：校验存在并同步冗余列 customer_name
        let customer_name_override = match req.customer_id {
            Some(cid) if cid != contract.customer_id => {
                let c = crate::models::customer::Entity::find_by_id(cid)
                    .one(&txn)
                    .await?
                    .ok_or_else(|| {
                        AppError::business_displayable("更新失败：所选客户不存在，请重新选择客户")
                    })?;
                Some(c.customer_name)
            }
            _ => None,
        };

        // 印花税生效值（在 contract 被 move 前捕获）：类型或金额任一变化即重算
        let stamp_tax_recalc_needed = req.contract_type.is_some() || req.total_amount.is_some();
        let effective_contract_type = req
            .contract_type
            .clone()
            .or_else(|| contract.contract_type.clone());
        let effective_total_amount = req
            .total_amount
            .unwrap_or(contract.total_amount.unwrap_or(Decimal::ZERO));

        let mut active: sales_contract::ActiveModel = contract.into();
        if let Some(v) = req.contract_name {
            active.contract_name = Set(v);
        }
        if let Some(v) = req.customer_id {
            active.customer_id = Set(v);
        }
        if let Some(name) = customer_name_override {
            active.customer_name = Set(Some(name));
        }
        if let Some(v) = req.contract_type {
            active.contract_type = Set(Some(v));
        }
        if let Some(v) = req.total_amount {
            active.total_amount = Set(Some(v));
        }
        if let Some(v) = req.payment_terms {
            active.payment_terms = Set(Some(v));
        }
        if let Some(v) = req.delivery_date {
            active.delivery_date = Set(Some(v));
        }
        if let Some(v) = req.signed_date {
            active.signed_date = Set(Some(v));
        }
        if let Some(v) = req.effective_date {
            active.effective_date = Set(Some(v));
        }
        if let Some(v) = req.expiry_date {
            active.expiry_date = Set(Some(v));
        }
        if let Some(v) = req.payment_method {
            active.payment_method = Set(Some(v));
        }
        if let Some(v) = req.delivery_location {
            active.delivery_location = Set(Some(v));
        }
        if let Some(v) = req.remark {
            active.remark = Set(Some(v));
        }

        // 印花税：合同类型或金额变化时按创建同口径重算（与 calculate_stamp_tax 单一真源）
        if stamp_tax_recalc_needed {
            let stamp_tax = Self::calculate_stamp_tax(
                effective_contract_type.as_deref(),
                effective_total_amount,
            );
            active.stamp_tax_amount = Set(stamp_tax);
        }

        active.updated_at = Set(chrono::Utc::now());

        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active,
            Some(user_id),
        )
        .await?;

        // 明细整表替换（Some 才动；None 保持原明细）
        if let Some(items) = &req.items {
            sales_contract_item::Entity::delete_many()
                .filter(sales_contract_item::Column::ContractId.eq(id))
                .exec(&txn)
                .await?;
            Self::insert_items_txn(&txn, id, items).await?;
        }

        txn.commit().await?;
        info!("销售合同 {} 更新成功", updated.contract_no);
        Ok(updated)
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
                ));
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
