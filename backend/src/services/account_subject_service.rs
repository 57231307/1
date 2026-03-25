//! 会计科目 Service
//!
//! 会计科目业务逻辑层

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, ModelTrait,
    Order, PaginatorTrait, QueryFilter, QueryOrder,
};
use std::sync::Arc;
use tracing::{info, warn};

use crate::models::{account_balance, account_subject, voucher_item};
use crate::utils::error::AppError;
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
            return Err(AppError::BadRequest(format!(
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
                return Err(AppError::BadRequest(format!("父科目不存在")));
            }
        }

        // 生成完整编码
        let full_code = if let Some(parent_id) = req.parent_id {
            let parent = account_subject::Entity::find_by_id(parent_id)
                .one(&*self.db)
                .await?
                .unwrap();
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
                if subject_map.contains_key(&subject.id) {
                    let node = subject_map.remove(&subject.id).unwrap();
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
            query = query.filter(
                account_subject::Column::Code
                    .like(format!("%{}%", keyword))
                    .or(account_subject::Column::Name.like(format!("%{}%", keyword))),
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
            .ok_or_else(|| AppError::NotFound(format!("会计科目不存在：{}", id)))?;

        Ok(subject)
    }

    /// 更新会计科目
    pub async fn update(
        &self,
        id: i32,
        req: UpdateSubjectRequest,
        _user_id: i32,
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

        let result = active_model.update(&*self.db).await?;
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
            return Err(AppError::BadRequest("不能删除有子科目的科目".to_string()));
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
            return Err(AppError::BadRequest(format!(
                "科目已被 {} 张凭证使用，不能删除",
                used_in_vouchers
            )));
        }

        subject.delete(&*self.db).await?;

        info!("会计科目删除成功：id={}", id);
        Ok(())
    }

    /// 查询科目余额
    pub async fn get_balance(
        &self,
        subject_id: i32,
        period: &str,
    ) -> Result<SubjectBalance, AppError> {
        info!("查询科目余额 subject_id={}, period={}", subject_id, period);

        let balance = account_balance::Entity::find()
            .filter(account_balance::Column::SubjectId.eq(subject_id))
            .filter(account_balance::Column::Period.eq(period))
            .one(&*self.db)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        if let Some(b) = balance {
            Ok(SubjectBalance {
                subject_id: b.subject_id,
                period: b.period,
                initial_balance_debit: b.initial_balance_debit,
                initial_balance_credit: b.initial_balance_credit,
                current_period_debit: b.current_period_debit,
                current_period_credit: b.current_period_credit,
                ending_balance_debit: b.ending_balance_debit,
                ending_balance_credit: b.ending_balance_credit,
            })
        } else {
            Ok(SubjectBalance {
                subject_id,
                period: period.to_string(),
                initial_balance_debit: rust_decimal::Decimal::ZERO,
                initial_balance_credit: rust_decimal::Decimal::ZERO,
                current_period_debit: rust_decimal::Decimal::ZERO,
                current_period_credit: rust_decimal::Decimal::ZERO,
                ending_balance_debit: rust_decimal::Decimal::ZERO,
                ending_balance_credit: rust_decimal::Decimal::ZERO,
            })
        }
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
#[allow(dead_code)]
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
