//! 凭证服务-辅助核算记录子模块（voucher_ops/assist）
//!
//! 批次 488 D10-2a 拆分：从原 `voucher_service.rs` L98-113 + L1052-1255 迁移。
//! 包含 AssistRecordContext 内部 struct + 11 个辅助核算方法：
//! - write_assist_accounting_records_txn（pub(crate)，被 workflow::post 调用）
//! - lookup_voucher / lookup_voucher_items / build_subject_id_map
//! - determine_business_type / determine_business_no
//! - insert_assist_records_for_items / has_assist_dimensions / lookup_subject_id
//! - build_five_dimension_id / build_assist_record
//!
//! 业务规则：
//! - 仅对包含辅助核算维度（客户/供应商/批次/色号/缸号/等级/车间等）的分录写入
//! - 五维 ID 格式：BATCH:{}|COLOR:{}|DYE_LOT:{}|GRADE:{}|WORKSHOP:{}
//! - F-P1-3 修复（批次 359 v13 复审）：原实现仅更新 account_balance 未写入辅助核算记录

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};
use tracing::info;

use crate::models::{account_subject, voucher, voucher_item};
use crate::utils::error::AppError;
use rust_decimal::Decimal;

use crate::services::voucher_service::VoucherService;

/// 辅助核算记录写入上下文（D08 第三梯队修复：消除 too_many_arguments 警告）
///
/// 封装业务关联字段与凭证上下文，避免 insert_assist_records_for_items /
/// build_assist_record 函数签名携带过多参数。
pub(super) struct AssistRecordContext<'a> {
    /// 业务类型
    business_type: &'a str,
    /// 业务单号
    business_no: &'a str,
    /// 业务单据 ID
    business_id: i32,
    /// 凭证 ID
    voucher_id: i32,
    /// 凭证模型
    voucher_model: &'a voucher::Model,
    /// 创建人 ID
    user_id: i32,
    /// 创建时间
    now: chrono::DateTime<chrono::Utc>,
}

impl VoucherService {
    /// F-P1-3 修复（批次 359 v13 复审）：凭证过账时写入辅助核算记录
    ///
    /// 遍历凭证分录，对包含辅助核算维度（客户/供应商/批次/色号/缸号/等级/车间等）
    /// 的分录生成 `assist_accounting_record` 记录，便于辅助核算明细账与汇总表查询。
    /// 原实现仅更新 `account_balance` 表，未写入辅助核算记录，导致辅助核算报表无数据。
    pub(crate) async fn write_assist_accounting_records_txn(
        &self,
        voucher_id: i32,
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        let voucher_model = self.lookup_voucher(txn, voucher_id).await?;
        let items = self.lookup_voucher_items(txn, voucher_id).await?;
        let subject_id_by_code = self.build_subject_id_map(txn, &items).await?;

        let business_type = self.determine_business_type(&voucher_model);
        let business_no = self.determine_business_no(&voucher_model);
        let business_id = voucher_model.source_bill_id.unwrap_or(voucher_id);

        // D08 第三梯队修复：使用 AssistRecordContext 聚合业务上下文参数，
        // 消除 insert_assist_records_for_items / build_assist_record 的 too_many_arguments 警告。
        let ctx = AssistRecordContext {
            business_type: &business_type,
            business_no: &business_no,
            business_id,
            voucher_id,
            voucher_model: &voucher_model,
            user_id,
            now: chrono::Utc::now(),
        };

        self.insert_assist_records_for_items(
            txn,
            &items,
            &subject_id_by_code,
            &ctx,
        ).await?;

        info!("辅助核算记录写入成功 voucher_id={}", voucher_id);
        Ok(())
    }

    async fn lookup_voucher(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        voucher_id: i32,
    ) -> Result<crate::models::voucher::Model, AppError> {
        voucher::Entity::find_by_id(voucher_id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found("凭证不存在"))
    }

    async fn lookup_voucher_items(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        voucher_id: i32,
    ) -> Result<Vec<crate::models::voucher_item::Model>, AppError> {
        voucher_item::Entity::find()
            .filter(voucher_item::Column::VoucherId.eq(voucher_id))
            .all(txn)
            .await
            .map_err(AppError::from)
    }

    async fn build_subject_id_map(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        items: &[crate::models::voucher_item::Model],
    ) -> Result<std::collections::HashMap<String, i32>, AppError> {
        let subject_codes: Vec<String> = items.iter().map(|i| i.subject_code.clone()).collect();

        if subject_codes.is_empty() {
            Ok(std::collections::HashMap::new())
        } else {
            let subjects = account_subject::Entity::find()
                .filter(account_subject::Column::Code.is_in(subject_codes))
                .all(txn)
                .await?;
            Ok(subjects
                .iter()
                .map(|s| (s.code.clone(), s.id))
                .collect())
        }
    }

    fn determine_business_type(&self, voucher_model: &crate::models::voucher::Model) -> String {
        voucher_model
            .source_module
            .clone()
            .or(voucher_model.source_type.clone())
            .unwrap_or_else(|| "VOUCHER".to_string())
    }

    fn determine_business_no(&self, voucher_model: &crate::models::voucher::Model) -> String {
        voucher_model
            .source_bill_no
            .clone()
            .unwrap_or_else(|| voucher_model.voucher_no.clone())
    }

    async fn insert_assist_records_for_items(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        items: &[crate::models::voucher_item::Model],
        subject_id_by_code: &std::collections::HashMap<String, i32>,
        ctx: &AssistRecordContext<'_>,
    ) -> Result<(), AppError> {
        for item in items {
            if !self.has_assist_dimensions(item) {
                continue;
            }

            let account_subject_id = self.lookup_subject_id(subject_id_by_code, item).await?;
            let five_dimension_id = self.build_five_dimension_id(item);

            let record = self.build_assist_record(
                item,
                account_subject_id,
                five_dimension_id,
                ctx,
            );

            record.insert(txn).await?;
        }

        Ok(())
    }

    fn has_assist_dimensions(&self, item: &crate::models::voucher_item::Model) -> bool {
        item.assist_customer_id.is_some()
            || item.assist_supplier_id.is_some()
            || item.assist_batch_id.is_some()
            || item.assist_color_no_id.is_some()
            || item.assist_dye_lot_id.is_some()
            || item.assist_grade.is_some()
            || item.assist_workshop_id.is_some()
            || item.assist_department_id.is_some()
            || item.assist_employee_id.is_some()
            || item.assist_project_id.is_some()
    }

    async fn lookup_subject_id(
        &self,
        subject_id_by_code: &std::collections::HashMap<String, i32>,
        item: &crate::models::voucher_item::Model,
    ) -> Result<i32, AppError> {
        Ok(*subject_id_by_code
            .get(item.subject_code.as_str())
            .ok_or_else(|| AppError::not_found(format!("科目不存在：{}", item.subject_code)))?)
    }

    fn build_five_dimension_id(&self, item: &crate::models::voucher_item::Model) -> String {
        format!(
            "BATCH:{}|COLOR:{}|DYE_LOT:{}|GRADE:{}|WORKSHOP:{}",
            item.assist_batch_id.unwrap_or(0),
            item.assist_color_no_id.unwrap_or(0),
            item.assist_dye_lot_id.unwrap_or(0),
            item.assist_grade.clone().unwrap_or_default(),
            item.assist_workshop_id.unwrap_or(0),
        )
    }

    fn build_assist_record(
        &self,
        item: &crate::models::voucher_item::Model,
        account_subject_id: i32,
        five_dimension_id: String,
        ctx: &AssistRecordContext<'_>,
    ) -> crate::models::assist_accounting_record::ActiveModel {
        use crate::models::assist_accounting_record;

        assist_accounting_record::ActiveModel {
            business_type: sea_orm::Set(ctx.business_type.to_string()),
            business_no: sea_orm::Set(ctx.business_no.to_string()),
            business_id: sea_orm::Set(ctx.business_id),
            account_subject_id: sea_orm::Set(account_subject_id),
            debit_amount: sea_orm::Set(item.debit),
            credit_amount: sea_orm::Set(item.credit),
            five_dimension_id: sea_orm::Set(five_dimension_id),
            product_id: sea_orm::Set(0),
            batch_no: sea_orm::Set(ctx.voucher_model.batch_no.clone().unwrap_or_else(|| {
                tracing::warn!(
                    "凭证 {} 的 batch_no 为 None，辅助核算记录使用空字符串占位",
                    ctx.voucher_id
                );
                String::new()
            })),
            color_no: sea_orm::Set(ctx.voucher_model.color_no.clone().unwrap_or_else(|| {
                tracing::warn!(
                    "凭证 {} 的 color_no 为 None，辅助核算记录使用空字符串占位",
                    ctx.voucher_id
                );
                String::new()
            })),
            dye_lot_no: sea_orm::Set(ctx.voucher_model.dye_lot_no.clone()),
            grade: sea_orm::Set(item.assist_grade.clone().unwrap_or_default()),
            workshop_id: sea_orm::Set(item.assist_workshop_id),
            warehouse_id: sea_orm::Set(0),
            customer_id: sea_orm::Set(item.assist_customer_id),
            supplier_id: sea_orm::Set(item.assist_supplier_id),
            quantity_meters: sea_orm::Set(item.quantity_meters.unwrap_or(Decimal::ZERO)),
            quantity_kg: sea_orm::Set(item.quantity_kg.unwrap_or(Decimal::ZERO)),
            remarks: sea_orm::Set(Some(format!("voucher_id={}", ctx.voucher_id))),
            created_at: sea_orm::Set(ctx.now),
            created_by: sea_orm::Set(Some(ctx.user_id)),
            ..Default::default()
        }
    }
}
