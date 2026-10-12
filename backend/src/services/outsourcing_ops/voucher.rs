//! 委外加工会计分录凭证 Service impl 子模块（outsourcing_ops/voucher）
//!
//! 批次 489 D10-2b 拆分：从原 `outsourcing_service.rs` L1549-1705 迁移。
//! 包含 OutsourcingVoucherService 的 8 个方法：
//! - create / delete / post（CRUD + 过账）
//! - get_by_id / get_by_no / list_by_order / list（查询）
//!
//! 业务规则：
//! - 凭证类型：issue(发料) / fee(加工费) / receipt(入库) / loss(损耗)
//! - 已过账凭证不可删除
//! - 过账：is_posted = true, posted_at = now

use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
    SqlErr,
};

use crate::models::outsourcing_order::{self, Entity as OrderEntity};
use crate::models::outsourcing_voucher::{
    self, ActiveModel as VoucherActiveModel, Entity as VoucherEntity, Model as VoucherModel,
};
use crate::utils::error::AppError;

use crate::services::outsourcing_ops::types::{
    CreateOutsourcingVoucherRequest, OutsourcingVoucherQuery,
};
use crate::services::outsourcing_service::{OutsourcingVoucherService, validate_voucher_type};

impl OutsourcingVoucherService {
    /// 创建委外凭证；建单人取服务端会话身份，请求体不承载身份
    pub async fn create(
        &self,
        req: CreateOutsourcingVoucherRequest,
        user_id: i32,
    ) -> Result<VoucherModel, AppError> {
        validate_voucher_type(&req.voucher_type)?;

        if req.amount < Decimal::ZERO {
            return Err(AppError::business("金额不能为负"));
        }

        // 校验委外订单存在（"缺前置"拒绝属可执行公开规则 → business_displayable
        // 可外显族，回显用户自己提交的 ID；真因落 WARN，不静默）
        if OrderEntity::find_by_id(req.outsourcing_order_id)
            .filter(outsourcing_order::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .is_none()
        {
            tracing::warn!(
                "创建委外凭证被拒：关联订单 outsourcing_order_id={} 不存在或未删除",
                req.outsourcing_order_id
            );
            return Err(AppError::business_displayable(format!(
                "委外订单 {} 不存在，请重新选择委外订单",
                req.outsourcing_order_id
            )));
        }

        // 校验凭证号唯一性 —— "你填的凭证号已存在，换一个再提交"属可执行公开规则，
        // 归 business_displayable 族（先例： 流程编码、chemical_ops 编码族）；
        // 文案只回显用户自己提交的凭证号；真因落 WARN
        if let Some(_existing) = VoucherEntity::find()
            .filter(outsourcing_voucher::Column::VoucherNo.eq(&req.voucher_no))
            .one(&*self.db)
            .await?
        {
            tracing::warn!("创建委外凭证被拒：凭证号 {} 已存在", req.voucher_no);
            return Err(AppError::business_displayable(format!(
                "凭证号 {} 已存在，请更换凭证号后重试",
                req.voucher_no
            )));
        }

        // voucher_no 随后被移入 ActiveModel，先留一份用于 INSERT 竞态兜底文案
        let voucher_no_for_race = req.voucher_no.clone();

        let now = crate::utils::date_utils::utc_now_fixed();

        let active = VoucherActiveModel {
            id: Default::default(),
            voucher_no: Set(req.voucher_no),
            outsourcing_order_id: Set(req.outsourcing_order_id),
            voucher_type: Set(req.voucher_type),
            debit_account: Set(req.debit_account),
            credit_account: Set(req.credit_account),
            amount: Set(req.amount),
            tax_amount: Set(req.tax_amount.unwrap_or(Decimal::ZERO)),
            tax_transfer_amount: Set(Decimal::ZERO),
            voucher_date: Set(req.voucher_date),
            is_posted: Set(false),
            posted_at: Set(None),
            remarks: Set(req.remarks),
            created_by: Set(Some(user_id)),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| {
                // 竞态兜底（照 role_permission/chemical_ops「预校验 + 23505 归类」范式）：
                // 上方凭证号查重通过后、INSERT 落库前，并发请求可能已插入同码凭证。
                // 本表 voucher_no 当前仅 NOT NULL 无 DB UNIQUE（v15/mod.rs:1179，待
                // 数据库专家补全列唯一索引）；索引就位后此处将以 23505 显式拒绝——
                // 单语句 INSERT 原子失败、不留半行，归类必须与预检同口径的业务拒绝，
                // 禁止 map_err 自造 database() 把撞号拍平成 500 吞真因。
                if matches!(e.sql_err(), Some(SqlErr::UniqueConstraintViolation(_))) {
                    tracing::error!(
                        "委外凭证创建撞凭证号唯一约束（并发同码，voucher_no={}）：行未写入，整语句回滚，底层错误={}",
                        voucher_no_for_race,
                        e
                    );
                    AppError::business_displayable(format!(
                        "凭证号 {} 已存在，请更换凭证号后重试",
                        voucher_no_for_race
                    ))
                } else {
                    tracing::error!(voucher_no = %voucher_no_for_race, "委外凭证创建落库失败");
                    AppError::from(e)
                }
            })?;
        Ok(result)
    }

    /// 删除委外凭证（物理删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        if model.is_posted {
            return Err(AppError::business("已过账凭证不可删除"));
        }
        VoucherEntity::delete_by_id(id)
            .exec(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("委外凭证删除失败: {}", e)))?;
        Ok(())
    }

    /// 过账：is_posted = true, posted_at = now
    pub async fn post(&self, id: i32) -> Result<VoucherModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.is_posted {
            return Err(AppError::business("凭证已过账，不可重复过账"));
        }
        let now = crate::utils::date_utils::utc_now_fixed();
        let mut active: VoucherActiveModel = model.into();
        active.is_posted = Set(true);
        active.posted_at = Set(Some(now));
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<VoucherModel, AppError> {
        VoucherEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("委外凭证 {} 不存在", id)))
    }

    /// 按凭证号查询
    pub async fn get_by_no(&self, voucher_no: &str) -> Result<VoucherModel, AppError> {
        VoucherEntity::find()
            .filter(outsourcing_voucher::Column::VoucherNo.eq(voucher_no))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("凭证号 {} 不存在", voucher_no)))
    }

    /// 按委外订单查询凭证列表
    pub async fn list_by_order(&self, order_id: i32) -> Result<Vec<VoucherModel>, AppError> {
        let items = VoucherEntity::find()
            .filter(outsourcing_voucher::Column::OutsourcingOrderId.eq(order_id))
            .order_by_desc(outsourcing_voucher::Column::Id)
            .all(&*self.db)
            .await?;
        Ok(items)
    }

    /// 分页查询
    pub async fn list(
        &self,
        query: OutsourcingVoucherQuery,
    ) -> Result<(Vec<VoucherModel>, u64), AppError> {
        let mut q = VoucherEntity::find();
        if let Some(v) = query.outsourcing_order_id {
            q = q.filter(outsourcing_voucher::Column::OutsourcingOrderId.eq(v));
        }
        if let Some(v) = query.voucher_type {
            q = q.filter(outsourcing_voucher::Column::VoucherType.eq(v));
        }
        if let Some(v) = query.is_posted {
            q = q.filter(outsourcing_voucher::Column::IsPosted.eq(v));
        }
        if let Some(v) = query.voucher_date_from {
            q = q.filter(outsourcing_voucher::Column::VoucherDate.gte(v));
        }
        if let Some(v) = query.voucher_date_to {
            q = q.filter(outsourcing_voucher::Column::VoucherDate.lte(v));
        }

        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_desc(outsourcing_voucher::Column::Id)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
    }
}
