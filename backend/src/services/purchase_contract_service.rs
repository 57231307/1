use crate::models::purchase_contract;
// 批次 210 P2-5 修复（v12 复审）：合同状态字符串替换为 contract 常量
use crate::models::status::contract;
use crate::models::user;
use crate::utils::error::AppError;
use crate::utils::sql_escape::safe_like_pattern;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ExprTrait, FromQueryResult,
    JoinType, Order, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, RelationTrait, Set,
    TransactionTrait,
};
use serde::Serialize;
use std::sync::Arc;
use tracing::info;

/// 采购合同读模型：实体列 + LEFT JOIN 关联出的创建人姓名（实体仅有 created_by 数值）。
///
/// JOIN 名列 `Option<String>`；实体自身列（含真实列 supplier_name）按 `purchase_contract`
/// 约束保持原类型。
#[derive(Debug, Clone, Serialize, FromQueryResult)]
pub struct PurchaseContractView {
    pub id: i32,
    pub contract_no: String,
    pub contract_name: String,
    pub contract_type: Option<String>,
    pub supplier_id: i32,
    pub supplier_name: Option<String>,
    pub total_amount: Option<Decimal>,
    pub signed_date: Option<chrono::NaiveDate>,
    pub effective_date: Option<chrono::NaiveDate>,
    pub expiry_date: Option<chrono::NaiveDate>,
    pub payment_terms: Option<String>,
    pub payment_method: Option<String>,
    pub delivery_date: Option<chrono::NaiveDate>,
    pub delivery_location: Option<String>,
    pub remark: Option<String>,
    pub status: String,
    pub created_by: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub created_by_name: Option<String>,
}

/// 解析日期筛选边界：前端下发 ISO / `YYYY-MM-DD`，取日期部分转 `NaiveDate`；解析失败视为未提供。
fn parse_date_bound(raw: &str) -> Option<chrono::NaiveDate> {
    chrono::NaiveDate::parse_from_str(&raw[..raw.len().min(10)], "%Y-%m-%d").ok()
}

/// 采购合同查询参数
#[derive(Debug, Clone, Default)]
pub struct ContractQueryParams {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub supplier_id: Option<i32>,
    /// 签订日期范围（前端 date_range 数组：[from, to]，取首/末两项）
    pub date_range: Option<Vec<String>>,
    pub page: i64,
    pub page_size: i64,
}

/// 创建采购合同请求
///
/// - `delivery_date` 为 Option：真实列 purchase_contracts.delivery_date 可空（m0009 DDL）。
/// - 表头真实列 contract_type/signed_date/effective_date/expiry_date/payment_method/
///   delivery_location 与 remark 全量落库（remark 列由 m0016 迁移补齐）。
#[derive(Debug, Clone)]
pub struct CreateContractRequest {
    pub contract_no: String,
    pub contract_name: String,
    pub supplier_id: i32,
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
}

/// 更新采购合同请求
///
/// 字段语义 = 显式三态部分更新（对齐 RFC 7386 JSON Merge Patch）：
/// - `None`（键缺席）＝保持原值；
/// - `Some(None)`（显式 null）＝该列清空为 NULL（仅对 DB 可空列开放）；
/// - `Some(Some(v))`（有值）＝覆盖。
/// 可空性逐字段按 m0009 DDL + m0016 补列核实：total_amount/contract_type/payment_terms/
/// delivery_date/signed_date/effective_date/expiry_date/payment_method/delivery_location/
/// remark 为 NULLable 列；contract_name/supplier_id 为 NOT NULL 列，不开 null 清空，
/// update() 在任何 DB 访问前将 Some(None) 判为业务错误拒绝。
/// 不得塌成单层 Option<T>：塌层后"显式 null"与"键缺席"同物，清空操作即被静默丢弃。
/// 不含 contract_no：合同编号由系统生成（单据号禁手打口径），更新链路不允许改写；
/// 若放开改写，改成已存在编号会直接撞 purchase_contracts.contract_no UNIQUE（无 23505→4xx
/// 自动映射，只会裸 500），且篡改单据号本身违反编号即身份的业务规则。
#[derive(Debug, Clone, Default)]
pub struct UpdateContractRequest {
    pub contract_name: Option<Option<String>>,
    pub supplier_id: Option<Option<i32>>,
    pub total_amount: Option<Option<Decimal>>,
    pub contract_type: Option<Option<String>>,
    pub payment_terms: Option<Option<String>>,
    pub delivery_date: Option<Option<NaiveDate>>,
    pub signed_date: Option<Option<NaiveDate>>,
    pub effective_date: Option<Option<NaiveDate>>,
    pub expiry_date: Option<Option<NaiveDate>>,
    pub payment_method: Option<Option<String>>,
    pub delivery_location: Option<Option<String>>,
    pub remark: Option<Option<String>>,
}

/// 合同执行请求
#[derive(Debug, Clone)]
pub struct ExecuteContractRequest {
    pub execution_type: String,
    pub execution_amount: Decimal,
    pub execution_date: chrono::NaiveDate,
    pub related_bill_type: Option<String>,
    pub related_bill_id: Option<i32>,
    pub remark: Option<String>,
}

pub struct PurchaseContractService {
    db: Arc<DatabaseConnection>,
}

impl PurchaseContractService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建采购合同
    pub async fn create(
        &self,
        req: CreateContractRequest,
        user_id: i32,
    ) -> Result<purchase_contract::Model, AppError> {
        info!("用户 {} 正在创建采购合同：{}", user_id, req.contract_no);

        // P0 契约修复：校验供应商存在并回填冗余列 supplier_name
        //（原实现既不校验也不回填 ⇒ 悬挂 supplier_id 可入库、列表「供应商」列恒空）
        let supplier = crate::models::supplier::Entity::find_by_id(req.supplier_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::business_displayable("创建失败：所选供应商不存在，请重新选择供应商")
            })?;

        let active_contract = purchase_contract::ActiveModel {
            contract_no: Set(req.contract_no),
            contract_name: Set(req.contract_name),
            contract_type: Set(req.contract_type),
            supplier_id: Set(req.supplier_id),
            supplier_name: Set(Some(supplier.supplier_name)),
            total_amount: Set(Some(req.total_amount)),
            status: Set(contract::DRAFT.to_string()),
            payment_terms: Set(req.payment_terms),
            payment_method: Set(req.payment_method),
            delivery_date: Set(req.delivery_date),
            delivery_location: Set(req.delivery_location),
            signed_date: Set(req.signed_date),
            effective_date: Set(req.effective_date),
            expiry_date: Set(req.expiry_date),
            remark: Set(req.remark),
            created_by: Set(user_id),
            ..Default::default()
        };

        let contract = active_contract.insert(&*self.db).await?;
        info!("采购合同创建成功：{}", contract.contract_no);
        Ok(contract)
    }

    /// 更新采购合同（表头全集，三态字段语义：None=保持、Some(None)=置 NULL、Some(Some)=覆盖）
    ///
    /// 在事务内以排他锁更新，仅 DRAFT 状态可改；覆盖表头全字段，可空列支持三态置 NULL。
    pub async fn update(
        &self,
        id: i32,
        req: UpdateContractRequest,
        user_id: i32,
    ) -> Result<purchase_contract::Model, AppError> {
        info!("用户 {} 正在更新采购合同 {}", user_id, id);

        // NOT NULL 列门控（purchase_contracts.contract_name / supplier_id 均 NOT NULL，
        // m0009 DDL 核实）：显式 null 是调用方错误，不是"保持原值"；
        // 在任何 DB 访问之前拒绝，按责任模块（请求构造方）归责，错误外显不脱敏。
        if matches!(req.contract_name, Some(None)) {
            return Err(AppError::business_displayable(
                "合同名称不能清空：该字段为必填项",
            ));
        }
        if matches!(req.supplier_id, Some(None)) {
            return Err(AppError::business_displayable(
                "供应商不能清空：请选择有效供应商",
            ));
        }

        let txn = (*self.db).begin().await?;

        let contract = purchase_contract::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购合同不存在：{}", id)))?;

        if contract.status != contract::DRAFT {
            return Err(AppError::business_displayable(
                "只有草稿状态的采购合同才能修改",
            ));
        }

        // 供应商变更：校验存在并同步冗余列 supplier_name
        let supplier_name_override = match req.supplier_id.flatten() {
            Some(sid) if sid != contract.supplier_id => {
                let s = crate::models::supplier::Entity::find_by_id(sid)
                    .one(&txn)
                    .await?
                    .ok_or_else(|| {
                        AppError::business_displayable(
                            "更新失败：所选供应商不存在，请重新选择供应商",
                        )
                    })?;
                Some(s.supplier_name)
            }
            _ => None,
        };

        let mut active: purchase_contract::ActiveModel = contract.into();
        // 三态写入规则（对齐 RFC 7386 JSON Merge Patch，字段类型 Option<Option<T>>）：
        //   None          = 键缺席 → 不 Set 该列（保持 Unset，UPDATE 语句不含该列，原值不动）
        //   Some(None)    = 显式 null → Set(None) → 该列写入 NULL
        //   Some(Some(v)) = 有值 → Set(v)/Set(Some(v)) → 覆盖
        // 三者不可塌成两层：塌成单层 Option<T> 后"清空"与"保持"共用同一表示，
        // 可空列将永远无法置 NULL，用户删掉交货日期/备注保存即被静默丢弃。
        // contract_name/supplier_id 为 NOT NULL 列（Some(None) 已在入口拒绝）：仅覆盖/保持
        if let Some(v) = req.contract_name.flatten() {
            active.contract_name = Set(v);
        }
        if let Some(v) = req.supplier_id.flatten() {
            active.supplier_id = Set(v);
        }
        if let Some(name) = supplier_name_override {
            active.supplier_name = Set(Some(name));
        }
        // 以下均为 DB 可空列（m0009 DDL + m0016 补列逐字段核实）：开放 null 清空
        if let Some(v) = req.total_amount {
            active.total_amount = Set(v);
        }
        if let Some(v) = req.contract_type {
            active.contract_type = Set(v);
        }
        if let Some(v) = req.payment_terms {
            active.payment_terms = Set(v);
        }
        if let Some(v) = req.payment_method {
            active.payment_method = Set(v);
        }
        if let Some(v) = req.delivery_date {
            active.delivery_date = Set(v);
        }
        if let Some(v) = req.delivery_location {
            active.delivery_location = Set(v);
        }
        if let Some(v) = req.signed_date {
            active.signed_date = Set(v);
        }
        if let Some(v) = req.effective_date {
            active.effective_date = Set(v);
        }
        if let Some(v) = req.expiry_date {
            active.expiry_date = Set(v);
        }
        if let Some(v) = req.remark {
            active.remark = Set(v);
        }

        active.updated_at = Set(chrono::Utc::now());

        // 审计：update_with_audit 以更新后回读的 Model 生成 after_snapshot，
        // Set(None) 的列在 UPDATE 真实落 NULL 后进入快照——审计反映变更后真实值，不留假旧值。
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;
        info!("采购合同 {} 更新成功", updated.contract_no);
        Ok(updated)
    }

    /// 获取合同列表（分页）
    pub async fn get_list(
        &self,
        params: ContractQueryParams,
    ) -> Result<(Vec<PurchaseContractView>, u64), AppError> {
        let mut query = purchase_contract::Entity::find();

        // 关键词筛选
        if let Some(keyword) = &params.keyword {
            let keyword_pattern = safe_like_pattern(keyword);
            query = query.filter(
                purchase_contract::Column::ContractNo
                    .like(&keyword_pattern)
                    .or(purchase_contract::Column::ContractName.like(&keyword_pattern)),
            );
        }

        // 状态筛选
        if let Some(status) = &params.status {
            query = query.filter(purchase_contract::Column::Status.eq(status));
        }

        // 供应商筛选
        if let Some(supplier_id) = params.supplier_id {
            query = query.filter(purchase_contract::Column::SupplierId.eq(supplier_id));
        }

        // 签订日期范围（date_range = [from, to]，任一端可缺省）
        if let Some(range) = &params.date_range {
            if let Some(first) = range.first().and_then(|s| parse_date_bound(s)) {
                query = query.filter(purchase_contract::Column::SignedDate.gte(first));
            }
            if let Some(last) = range
                .last()
                .filter(|_| range.len() >= 2)
                .and_then(|s| parse_date_bound(s))
            {
                query = query.filter(purchase_contract::Column::SignedDate.lte(last));
            }
        }

        // 总数在无 JOIN 的基础查询上统计：Creator JOIN 为多对一（不倍增行），单次查询无 N+1。
        let total = query.clone().count(&*self.db).await?;

        // 分页和排序（LEFT JOIN users 补创建人姓名）
        let contracts = query
            .column_as(user::Column::RealName, "created_by_name")
            .join(
                JoinType::LeftJoin,
                purchase_contract::Relation::Creator.def(),
            )
            .order_by(purchase_contract::Column::Id, Order::Desc)
            // 批次 98 P2-A 修复（v5 复审）：page clamp 防 DoS
            .offset((params.page.clamp(1, 1000).saturating_sub(1) * params.page_size) as u64)
            .limit(params.page_size as u64)
            .into_model::<PurchaseContractView>()
            .all(&*self.db)
            .await?;

        Ok((contracts, total))
    }

    /// 获取合同详情
    pub async fn get_by_id(&self, id: i32) -> Result<purchase_contract::Model, AppError> {
        let contract = purchase_contract::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购合同不存在：{}", id)))?;
        Ok(contract)
    }

    /// 执行合同（入库或付款）
    pub async fn execute(
        &self,
        contract_id: i32,
        req: ExecuteContractRequest,
        user_id: i32,
    ) -> Result<(), AppError> {
        info!(
            "用户 {} 正在执行合同 {}，类型：{}",
            user_id, contract_id, req.execution_type
        );

        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原实现先在事务外用 get_by_id 裸查询合同状态，再 begin() 开启事务，
        // 并发 execute 均通过状态检查后基于过期状态写入，导致状态门失效。
        let txn = (*self.db).begin().await?;
        let contract = self
            .lock_and_validate_contract_txn(&txn, contract_id, &req)
            .await?;
        // 批次 27 v7 P1 修复：事务边界内校验已执行金额（防 TOCTOU）
        let total_amount = contract.total_amount.unwrap_or(Decimal::ZERO);
        if total_amount > Decimal::ZERO {
            self.check_remaining_amount_txn(&txn, contract_id, total_amount, req.execution_amount)
                .await?;
        }
        let execution_amount = req.execution_amount;
        let execution = Self::build_execution_active_model(contract_id, req, user_id);
        execution.insert(&txn).await?;
        // 更新合同时间戳
        let mut contract_active: purchase_contract::ActiveModel = contract.into();
        contract_active.updated_at = Set(chrono::Utc::now());
        contract_active.save(&txn).await?;
        txn.commit().await?;
        info!(
            "合同 {} 执行成功，执行金额：{}",
            contract_id, execution_amount
        );
        Ok(())
    }

    /// 事务内锁定合同并校验状态与执行金额
    async fn lock_and_validate_contract_txn(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        contract_id: i32,
        req: &ExecuteContractRequest,
    ) -> Result<purchase_contract::Model, AppError> {
        let contract = purchase_contract::Entity::find_by_id(contract_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购合同不存在：{}", contract_id)))?;
        if contract.status != contract::ACTIVE {
            // 状态门：合同非活跃，执行前置未满足，归业务族；文案纯规则可外显
            return Err(AppError::business_displayable(
                "只有活跃状态的合同才能执行".to_string(),
            ));
        }
        if req.execution_amount <= Decimal::ZERO {
            // 输入校验：用户提交字段取值非法，保持校验族 VALIDATION_ERROR
            return Err(AppError::validation("执行金额必须大于零"));
        }
        Ok(contract)
    }

    /// 校验执行金额不超过合同剩余可执行金额（防 TOCTOU，事务内查询）
    async fn check_remaining_amount_txn(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        contract_id: i32,
        total_amount: Decimal,
        execution_amount: Decimal,
    ) -> Result<(), AppError> {
        let executed_amount = crate::models::purchase_contract_execution::Entity::find()
            .filter(crate::models::purchase_contract_execution::Column::ContractId.eq(contract_id))
            .all(txn)
            .await?
            .iter()
            .map(|e| e.amount)
            .fold(Decimal::ZERO, |acc, x| acc + x);
        let remaining = total_amount - executed_amount;
        if execution_amount > remaining {
            return Err(AppError::validation(format!(
                "执行金额 {} 超过合同剩余可执行金额 {}（合同总额 {}，已执行 {}）",
                execution_amount, remaining, total_amount, executed_amount
            )));
        }
        Ok(())
    }

    /// 构造采购合同执行记录 ActiveModel
    fn build_execution_active_model(
        contract_id: i32,
        req: ExecuteContractRequest,
        user_id: i32,
    ) -> crate::models::purchase_contract_execution::ActiveModel {
        crate::models::purchase_contract_execution::ActiveModel {
            id: Default::default(),
            contract_id: Set(contract_id),
            execution_no: Set(format!(
                "PCE{}{}",
                chrono::Utc::now().format("%Y%m%d%H%M%S"),
                contract_id
            )),
            execution_type: Set(req.execution_type),
            execution_date: Set(req.execution_date),
            quantity: Set(req.execution_amount),
            amount: Set(req.execution_amount),
            status: Set("COMPLETED".to_string()),
            remarks: Set(req.remark),
            created_by: Set(user_id),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        }
    }

    /// 审核合同（通过动作）：draft → active，通过理由真实落 `approval_reason` 列
    pub async fn approve(
        &self,
        contract_id: i32,
        user_id: i32,
        approval_reason: String,
    ) -> Result<(), AppError> {
        info!("用户 {} 正在审核合同 {}", user_id, contract_id);

        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原实现无事务、无行锁，并发审核会基于过期状态通过状态检查后重复写入。
        let txn = (*self.db).begin().await?;

        // 1. 加 lock_exclusive 串行化并发状态变更
        let contract = purchase_contract::Entity::find_by_id(contract_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购合同不存在：{}", contract_id)))?;

        // 2. 检查状态
        if contract.status != contract::DRAFT {
            // 状态门：前置状态未满足（仅草稿可审核），归业务族；文案纯规则可外显
            return Err(AppError::business_displayable(
                "只有草稿状态的合同才能审核".to_string(),
            ));
        }

        // 3. 更新状态 + 审计日志（事务内原子提交）
        let mut contract_active: purchase_contract::ActiveModel = contract.into();
        contract_active.status = Set(contract::ACTIVE.to_string());
        // 通过理由真实落列（handler 侧已保证 trim 非空），不再只进日志
        contract_active.approval_reason = Set(Some(approval_reason));
        contract_active.updated_at = Set(chrono::Utc::now());

        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            contract_active,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        info!("合同 {} 审核成功", contract_id);
        Ok(())
    }

    /// 拒绝合同（拒绝动作）：draft → rejected 终态流转，拒绝理由落 `rejected_reason` 列
    ///
    /// 状态门只允许 draft 起拒：active（权利义务已生效）与 cancelled（已作废）一律拦，
    /// rejected 本身为终态——本批不出 rejected→draft 出边（重开须重新发起新合同）。
    /// 门控值与写入值同源自 `models::status::contract`，禁止字面量。
    pub async fn reject(
        &self,
        contract_id: i32,
        user_id: i32,
        reason: String,
    ) -> Result<(), AppError> {
        info!(
            "用户 {} 正在拒绝合同 {}，拒绝理由：{}",
            user_id, contract_id, reason
        );

        // 与 approve 同形：begin txn + lock_exclusive 串行化并发状态变更 + update_with_audit
        let txn = (*self.db).begin().await?;

        let contract = purchase_contract::Entity::find_by_id(contract_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购合同不存在：{}", contract_id)))?;

        if contract.status != contract::DRAFT {
            // 状态门：前置状态未满足（仅草稿可拒绝），归业务族；文案纯规则可外显
            return Err(AppError::business_displayable(
                "只有草稿状态的合同才能拒绝".to_string(),
            ));
        }

        let mut contract_active: purchase_contract::ActiveModel = contract.into();
        contract_active.status = Set(contract::REJECTED.to_string());
        // 两动作两列：拒绝只落 rejected_reason，不挪用 approval_reason；
        // cancel（作废）动作不使用本列，故 cancelled 行为 NULL 是预期取值域。
        contract_active.rejected_reason = Set(Some(reason));
        contract_active.updated_at = Set(chrono::Utc::now());

        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            contract_active,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        info!("合同 {} 拒绝成功", contract_id);
        Ok(())
    }

    /// 取消合同
    pub async fn cancel(
        &self,
        contract_id: i32,
        user_id: i32,
        reason: String,
    ) -> Result<(), AppError> {
        info!(
            "用户 {} 正在取消合同 {}，原因：{}",
            user_id, contract_id, reason
        );

        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原实现无事务、无行锁，并发取消会基于过期状态通过状态检查后重复写入。
        let txn = (*self.db).begin().await?;

        // 1. 加 lock_exclusive 串行化并发状态变更
        let contract = purchase_contract::Entity::find_by_id(contract_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购合同不存在：{}", contract_id)))?;

        // 2. 检查状态
        if contract.status != contract::ACTIVE && contract.status != contract::DRAFT {
            // 状态门：前置状态未满足（仅活跃/草稿可取消），归业务族；文案纯规则可外显
            return Err(AppError::business_displayable(
                "只能取消活跃或草稿状态的合同".to_string(),
            ));
        }

        // 3. 更新状态 + 审计日志（事务内原子提交）
        let mut contract_active: purchase_contract::ActiveModel = contract.into();
        contract_active.status = Set(contract::CANCELLED.to_string());
        contract_active.updated_at = Set(chrono::Utc::now());

        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            contract_active,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        info!("合同 {} 取消成功", contract_id);
        Ok(())
    }
}
