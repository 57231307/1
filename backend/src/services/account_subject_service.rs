//! 会计科目 Service
//!
//! 会计科目业务逻辑层

use sea_orm::{
    sea_query::Expr, ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait,
    IntoActiveModel, JoinType, ModelTrait, Order, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, RelationTrait, Set,
};
use std::sync::Arc;
use tracing::{info, warn};

use crate::models::{account_balance, account_subject, voucher, voucher_item};
use crate::utils::error::AppError;
use crate::utils::sql_escape::safe_like_pattern;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 创建科目请求
#[derive(Debug, Clone)]
pub struct CreateSubjectRequest {
    pub code: String,
    pub name: String,
    pub level: i32,
    pub parent_id: Option<i32>,
    pub balance_direction: Option<String>,
    pub assist_customer: bool,
    pub assist_supplier: bool,
    pub assist_batch: bool,
    pub assist_color_no: bool,
    pub enable_dual_unit: bool,
}

/// 更新科目请求
#[derive(Debug, Clone)]
pub struct UpdateSubjectRequest {
    pub name: Option<String>,
    pub balance_direction: Option<String>,
    pub assist_customer: bool,
    pub assist_supplier: bool,
    pub assist_batch: bool,
    pub assist_color_no: bool,
    pub enable_dual_unit: bool,
}

/// 科目查询参数
#[derive(Debug, Clone)]
pub struct SubjectQueryParams {
    pub level: Option<i32>,
    pub parent_id: Option<i32>,
    pub status: Option<String>,
    pub keyword: Option<String>,
}

/// 会计科目 Service
pub struct AccountSubjectService {
    db: Arc<DatabaseConnection>,
}

impl AccountSubjectService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建会计科目
    pub async fn create(
        &self,
        req: CreateSubjectRequest,
        _user_id: i32,
    ) -> Result<account_subject::Model, AppError> {
        info!("创建会计科目：code={}, name={}", req.code, req.name);

        // 检查科目编码是否已存在
        let existing = account_subject::Entity::find()
            .filter(account_subject::Column::Code.eq(&req.code))
            .one(&*self.db)
            .await?;

        if existing.is_some() {
            warn!("科目编码已存在：{}", req.code);
            return Err(AppError::bad_request(format!(
                "科目编码 {} 已存在",
                req.code
            )));
        }

        // 如果是子科目，检查父科目是否存在
        if let Some(parent_id) = req.parent_id {
            let parent = account_subject::Entity::find_by_id(parent_id)
                .one(&*self.db)
                .await?;
            if parent.is_none() {
                warn!("父科目不存在：{}", parent_id);
                return Err(AppError::bad_request("父科目不存在"));
            }
        }

        // 生成完整编码
        let full_code = if let Some(parent_id) = req.parent_id {
            let parent = account_subject::Entity::find_by_id(parent_id)
                .one(&*self.db)
                .await?
                .ok_or_else(|| AppError::bad_request("父科目不存在"))?;
            format!("{}.{}", parent.full_code.unwrap_or(parent.code), req.code)
        } else {
            req.code.clone()
        };

        // 创建科目
        let active_model = account_subject::ActiveModel {
            code: sea_orm::Set(req.code),
            name: sea_orm::Set(req.name),
            level: sea_orm::Set(req.level),
            parent_id: sea_orm::Set(req.parent_id),
            full_code: sea_orm::Set(Some(full_code)),
            balance_direction: sea_orm::Set(req.balance_direction),
            assist_customer: sea_orm::Set(req.assist_customer),
            assist_supplier: sea_orm::Set(req.assist_supplier),
            assist_batch: sea_orm::Set(req.assist_batch),
            assist_color_no: sea_orm::Set(req.assist_color_no),
            enable_dual_unit: sea_orm::Set(req.enable_dual_unit),
            ..Default::default()
        };

        let result = active_model.insert(&*self.db).await?;
        info!("会计科目创建成功：id={}", result.id);

        Ok(result)
    }

    /// 查询科目树
    pub async fn get_tree(&self) -> Result<Vec<SubjectTreeNode>, AppError> {
        info!("查询会计科目树");

        let all_subjects = account_subject::Entity::find()
            .order_by(account_subject::Column::Code, Order::Asc)
            .all(&*self.db)
            .await?;

        // 构建树形结构
        let mut tree = Vec::new();
        let mut subject_map = std::collections::HashMap::new();

        // 先创建所有节点
        for subject in &all_subjects {
            let node = SubjectTreeNode {
                id: subject.id,
                code: subject.code.clone(),
                name: subject.name.clone(),
                level: subject.level,
                children: Vec::new(),
            };
            subject_map.insert(subject.id, node);
        }

        // 构建树
        for subject in &all_subjects {
            if let Some(parent_id) = subject.parent_id {
                if let Some(node) = subject_map.remove(&subject.id) {
                    if let Some(parent_node) = subject_map.get_mut(&parent_id) {
                        parent_node.children.push(node);
                    } else {
                        tree.push(node);
                    }
                }
            } else {
                // 根节点
                if let Some(node) = subject_map.remove(&subject.id) {
                    tree.push(node);
                }
            }
        }

        info!("会计科目树查询成功，共 {} 个根节点", tree.len());
        Ok(tree)
    }

    /// 查询科目列表
    pub async fn get_list(
        &self,
        params: SubjectQueryParams,
    ) -> Result<Vec<account_subject::Model>, AppError> {
        info!("查询会计科目列表");

        let mut query = account_subject::Entity::find();

        if let Some(level) = params.level {
            query = query.filter(account_subject::Column::Level.eq(level));
        }

        if let Some(parent_id) = params.parent_id {
            query = query.filter(account_subject::Column::ParentId.eq(parent_id));
        }

        if let Some(status) = params.status {
            query = query.filter(account_subject::Column::Status.eq(status));
        }

        if let Some(keyword) = params.keyword {
            let pattern = safe_like_pattern(&keyword);
            query = query.filter(
                account_subject::Column::Code
                    .like(&pattern)
                    .or(account_subject::Column::Name.like(&pattern)),
            );
        }

        let subjects = query
            .order_by(account_subject::Column::Code, Order::Asc)
            .all(&*self.db)
            .await?;

        info!("会计科目列表查询成功，共 {} 条", subjects.len());
        Ok(subjects)
    }

    /// 查询科目详情
    pub async fn get_by_id(&self, id: i32) -> Result<account_subject::Model, AppError> {
        info!("查询会计科目详情 ID: {}", id);

        let subject = account_subject::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("会计科目不存在：{}", id)))?;

        Ok(subject)
    }

    /// 更新会计科目
    pub async fn update(
        &self,
        id: i32,
        req: UpdateSubjectRequest,
        user_id: i32,
    ) -> Result<account_subject::Model, AppError> {
        info!("更新会计科目 ID: {}", id);

        let subject = self.get_by_id(id).await?;

        let mut active_model: account_subject::ActiveModel = subject.into_active_model();

        if let Some(name) = req.name {
            active_model.name = sea_orm::Set(name);
        }

        if let Some(balance_direction) = req.balance_direction {
            active_model.balance_direction = sea_orm::Set(Some(balance_direction));
        }

        active_model.assist_customer = sea_orm::Set(req.assist_customer);
        active_model.assist_supplier = sea_orm::Set(req.assist_supplier);
        active_model.assist_batch = sea_orm::Set(req.assist_batch);
        active_model.assist_color_no = sea_orm::Set(req.assist_color_no);
        active_model.enable_dual_unit = sea_orm::Set(req.enable_dual_unit);

        let result = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            active_model,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;
        info!("会计科目更新成功：id={}", result.id);

        Ok(result)
    }

    /// 删除会计科目
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        info!("删除会计科目 ID: {}", id);

        // 检查是否有子科目
        let children_count = account_subject::Entity::find()
            .filter(account_subject::Column::ParentId.eq(id))
            .count(&*self.db)
            .await?;

        if children_count > 0 {
            warn!("不能删除有子科目的科目：{}", id);
            return Err(AppError::bad_request("不能删除有子科目的科目"));
        }

        // 检查是否已被凭证分录使用
        let subject = self.get_by_id(id).await?;
        let subject_code = subject.code.clone();

        let used_in_vouchers = voucher_item::Entity::find()
            .filter(voucher_item::Column::SubjectCode.eq(&subject_code))
            .count(&*self.db)
            .await?;

        if used_in_vouchers > 0 {
            warn!(
                "不能删除已被凭证使用的科目：{}，被引用次数：{}",
                id, used_in_vouchers
            );
            return Err(AppError::bad_request(format!(
                "科目已被 {} 张凭证使用，不能删除",
                used_in_vouchers
            )));
        }

        // 检查是否有科目余额记录
        let balance_count = account_balance::Entity::find()
            .filter(account_balance::Column::SubjectId.eq(id))
            .count(&*self.db)
            .await?;

        if balance_count > 0 {
            warn!("不能删除有余额记录的科目：{}", id);
            return Err(AppError::bad_request(format!(
                "科目有 {} 条余额记录，不能删除",
                balance_count
            )));
        }

        subject.delete(&*self.db).await?;

        info!("会计科目删除成功：id={}", id);
        Ok(())
    }

    /// 刷新科目余额（F-P1-4 修复，批次 358 v13 复审）
    ///
    /// v13 复审 F-P1-4 发现：`account_subject` 模型有 6 个余额字段
    /// （`initial_balance_debit/credit`、`current_period_debit/credit`、`ending_balance_debit/credit`），
    /// 但本 Service 缺少 `refresh_balance` 方法，导致科目主数据的余额字段无法独立重算，
    /// 仅依赖 `voucher_service.update_account_balances` 在凭证过账时同步写入 `account_balance` 表。
    /// 当出现凭证反审核、外部数据导入、余额漂移等场景时，科目主数据的余额字段无法纠正。
    ///
    /// 本方法从已过账凭证分录重新聚合指定期间的借贷发生额，按余额方向计算期末余额，
    /// 写回 `account_subject` 的 `current_period_debit/credit` 和 `ending_balance_debit/credit`。
    ///
    /// 计算规则（与 `voucher_service.update_account_balances` 一致）：
    /// - 借方科目：期末余额 = 期初余额(借) + 本期借方发生 - 本期贷方发生
    /// - 贷方科目：期末余额 = 期初余额(贷) + 本期贷方发生 - 本期借方发生
    pub async fn refresh_balance(
        &self,
        subject_id: i32,
        period: &str,
    ) -> Result<account_subject::Model, AppError> {
        info!(
            "刷新科目余额：subject_id={}, period={}",
            subject_id, period
        );

        // 1. 查询科目信息（含期初余额和余额方向）
        let subject = account_subject::Entity::find_by_id(subject_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("会计科目 {}", subject_id)))?;

        // 2. 解析期间字符串 "YYYY-MM" 为日期范围
        let year: i32 = period.get(0..4).ok_or_else(|| AppError::bad_request("期间格式错误，应为 YYYY-MM"))?
            .parse()
            .map_err(|_| AppError::bad_request("期间年份解析失败，应为 YYYY-MM"))?;
        let month: u32 = period.get(5..7).ok_or_else(|| AppError::bad_request("期间格式错误，应为 YYYY-MM"))?
            .parse()
            .map_err(|_| AppError::bad_request("期间月份解析失败，应为 YYYY-MM"))?;
        let start_date = chrono::NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or_else(|| AppError::bad_request("期间起始日期无效"))?;
        let (next_year, next_month) = if month == 12 {
            (year + 1, 1u32)
        } else {
            (year, month + 1)
        };
        let next_month_first = chrono::NaiveDate::from_ymd_opt(next_year, next_month, 1)
            .ok_or_else(|| AppError::bad_request("期间结束日期无效"))?;

        // 3. 联表查询已过账凭证分录的借贷汇总
        let (total_debit_opt, total_credit_opt): (Option<Decimal>, Option<Decimal>) =
            voucher_item::Entity::find()
                .join(JoinType::InnerJoin, voucher_item::Relation::Voucher.def())
                .filter(voucher_item::Column::SubjectCode.eq(&subject.code))
                .filter(voucher::Column::Status.eq(crate::models::status::VOUCHER_POSTED))
                .filter(voucher::Column::VoucherDate.gte(start_date))
                .filter(voucher::Column::VoucherDate.lt(next_month_first))
                .select_only()
                .column_as(
                    Expr::col(voucher_item::Column::Debit).sum(),
                    "total_debit",
                )
                .column_as(
                    Expr::col(voucher_item::Column::Credit).sum(),
                    "total_credit",
                )
                .into_tuple()
                .one(&*self.db)
                .await?;

        let current_period_debit = total_debit_opt.unwrap_or(Decimal::ZERO);
        let current_period_credit = total_credit_opt.unwrap_or(Decimal::ZERO);

        // 4. 根据余额方向计算期末余额
        let balance_direction = subject.balance_direction.as_deref().unwrap_or("借");
        let initial_debit = subject.initial_balance_debit;
        let initial_credit = subject.initial_balance_credit;

        let (ending_balance_debit, ending_balance_credit) = if balance_direction == "借" {
            // 借方科目：期末余额 = 期初借方 + 本期借方 - 本期贷方
            let ending_balance = initial_debit + current_period_debit - current_period_credit;
            if ending_balance >= Decimal::ZERO {
                (ending_balance, Decimal::ZERO)
            } else {
                (Decimal::ZERO, ending_balance.abs())
            }
        } else {
            // 贷方科目：期末余额 = 期初贷方 + 本期贷方 - 本期借方
            let ending_balance = initial_credit + current_period_credit - current_period_debit;
            if ending_balance >= Decimal::ZERO {
                (Decimal::ZERO, ending_balance)
            } else {
                (ending_balance.abs(), Decimal::ZERO)
            }
        };

        // 5. 写回科目主数据的余额字段
        let mut active_model: account_subject::ActiveModel = subject.into();
        active_model.current_period_debit = Set(current_period_debit);
        active_model.current_period_credit = Set(current_period_credit);
        active_model.ending_balance_debit = Set(ending_balance_debit);
        active_model.ending_balance_credit = Set(ending_balance_credit);
        active_model.updated_at = Set(chrono::Utc::now());

        let updated = active_model.update(&*self.db).await?;

        info!(
            "科目余额刷新成功：subject_id={}, period={}, 本期借={}, 本期贷={}, 期末借={}, 期末贷={}",
            updated.id,
            period,
            current_period_debit,
            current_period_credit,
            ending_balance_debit,
            ending_balance_credit
        );

        Ok(updated)
    }
}

/// 科目树节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubjectTreeNode {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub level: i32,
    pub children: Vec<SubjectTreeNode>,
}

/// 科目余额
#[derive(Debug, Clone)]
pub struct SubjectBalance {
    pub subject_id: i32,
    pub period: String,
    pub initial_balance_debit: rust_decimal::Decimal,
    pub initial_balance_credit: rust_decimal::Decimal,
    pub current_period_debit: rust_decimal::Decimal,
    pub current_period_credit: rust_decimal::Decimal,
    pub ending_balance_debit: rust_decimal::Decimal,
    pub ending_balance_credit: rust_decimal::Decimal,
}

