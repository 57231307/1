use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use std::sync::Arc;
use tracing::info;

use crate::models::{
    assist_accounting_dimension, assist_accounting_record, assist_accounting_summary,
};

/// 辅助核算服务
#[derive(Debug, Clone)]
pub struct AssistAccountingService {
    db: Arc<DatabaseConnection>,
}

impl AssistAccountingService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 初始化 8 个辅助核算维度
    #[allow(dead_code)]
    pub async fn initialize_dimensions(&self) -> Result<(), sea_orm::DbErr> {
        let dimensions = vec![
            ("BATCH", "批次核算", "按生产批次进行辅助核算"),
            ("COLOR", "色号核算", "按产品色号进行辅助核算"),
            ("DYE_LOT", "缸号核算", "按染色缸次进行辅助核算"),
            ("GRADE", "等级核算", "按产品质量等级进行辅助核算"),
            ("WORKSHOP", "车间核算", "按生产车间进行辅助核算"),
            ("WAREHOUSE", "仓库核算", "按仓库进行辅助核算"),
            ("CUSTOMER", "客户核算", "按客户进行辅助核算"),
            ("SUPPLIER", "供应商核算", "按供应商进行辅助核算"),
        ];

        for (i, (code, name, desc)) in dimensions.iter().enumerate() {
            let dimension = assist_accounting_dimension::ActiveModel {
                id: Set(0),
                dimension_code: Set(code.to_string()),
                dimension_name: Set(name.to_string()),
                description: Set(Some(desc.to_string())),
                is_active: Set(true),
                sort_order: Set((i + 1) as i32),
                created_at: Set(Utc::now()),
                updated_at: Set(Utc::now()),
            };

            // 如果不存在则插入
            let exists = assist_accounting_dimension::Entity::find()
                .filter(assist_accounting_dimension::Column::DimensionCode.eq(*code))
                .one(&*self.db)
                .await?;

            if exists.is_none() {
                dimension.insert(&*self.db).await?;
            }
        }

        Ok(())
    }

    /// 创建辅助核算记录
    #[allow(clippy::too_many_arguments)]
    #[allow(dead_code)]
    pub async fn create_assist_record(
        &self,
        business_type: String,
        business_no: String,
        business_id: i32,
        account_subject_id: i32,
        debit_amount: Decimal,
        credit_amount: Decimal,
        five_dimension_id: String,
        product_id: i32,
        batch_no: String,
        color_no: String,
        dye_lot_no: Option<String>,
        grade: String,
        warehouse_id: i32,
        quantity_meters: Decimal,
        quantity_kg: Decimal,
        workshop_id: Option<i32>,
        customer_id: Option<i32>,
        supplier_id: Option<i32>,
        remarks: Option<String>,
        created_by: Option<i32>,
    ) -> Result<assist_accounting_record::Model, sea_orm::DbErr> {
        let active_record = assist_accounting_record::ActiveModel {
            id: Set(0),
            business_type: Set(business_type),
            business_no: Set(business_no),
            business_id: Set(business_id),
            account_subject_id: Set(account_subject_id),
            debit_amount: Set(debit_amount),
            credit_amount: Set(credit_amount),
            five_dimension_id: Set(five_dimension_id),
            product_id: Set(product_id),
            batch_no: Set(batch_no),
            color_no: Set(color_no),
            dye_lot_no: Set(dye_lot_no),
            grade: Set(grade),
            workshop_id: Set(workshop_id),
            warehouse_id: Set(warehouse_id),
            customer_id: Set(customer_id),
            supplier_id: Set(supplier_id),
            quantity_meters: Set(quantity_meters),
            quantity_kg: Set(quantity_kg),
            remarks: Set(remarks),
            created_at: Set(Utc::now()),
            created_by: Set(created_by),
        };

        active_record.insert(&*self.db).await
    }

    /// 按业务类型和业务单号查询辅助核算记录
    pub async fn find_by_business(
        &self,
        business_type: &str,
        business_no: &str,
    ) -> Result<Vec<assist_accounting_record::Model>, sea_orm::DbErr> {
        assist_accounting_record::Entity::find()
            .filter(assist_accounting_record::Column::BusinessType.eq(business_type))
            .filter(assist_accounting_record::Column::BusinessNo.eq(business_no))
            .order_by(assist_accounting_record::Column::CreatedAt, Order::Asc)
            .all(&*self.db)
            .await
    }

    /// 按五维 ID 查询辅助核算记录
    pub async fn find_by_five_dimension(
        &self,
        five_dimension_id: &str,
    ) -> Result<Vec<assist_accounting_record::Model>, sea_orm::DbErr> {
        assist_accounting_record::Entity::find()
            .filter(assist_accounting_record::Column::FiveDimensionId.eq(five_dimension_id))
            .order_by(assist_accounting_record::Column::CreatedAt, Order::Desc)
            .all(&*self.db)
            .await
    }

    /// 按会计期间和维度查询汇总
    #[allow(dead_code)]
    pub async fn find_summary_by_period_and_dimension(
        &self,
        accounting_period: &str,
        dimension_code: &str,
    ) -> Result<Vec<assist_accounting_summary::Model>, sea_orm::DbErr> {
        assist_accounting_summary::Entity::find()
            .filter(assist_accounting_summary::Column::AccountingPeriod.eq(accounting_period))
            .filter(assist_accounting_summary::Column::DimensionCode.eq(dimension_code))
            .all(&*self.db)
            .await
    }

    /// 生成会计期间汇总（按月）
    pub async fn generate_monthly_summary(
        &self,
        year: i32,
        month: u32,
    ) -> Result<(), sea_orm::DbErr> {
        use sea_orm::ColumnTrait;

        let accounting_period = format!("{:04}-{:02}", year, month);
        info!("正在生成 {} 的辅助核算月度汇总", accounting_period);

        let start_date = chrono::NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or_else(|| sea_orm::DbErr::Custom("无效的日期".to_string()))?;
        let end_date = if month == 12 {
            chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
        }
        .ok_or_else(|| sea_orm::DbErr::Custom("无效的日期".to_string()))
        .map(|d| d - chrono::Duration::days(1));

        if let Ok(end_date) = end_date {
            let start = chrono::DateTime::<Utc>::from_naive_utc_and_offset(
                start_date
                    .and_hms_opt(0, 0, 0)
                    .unwrap_or_else(|| start_date.and_hms_opt(0, 0, 0).unwrap()),
                chrono::Utc,
            );
            let end = chrono::DateTime::<Utc>::from_naive_utc_and_offset(
                end_date
                    .and_hms_opt(0, 0, 0)
                    .unwrap_or_else(|| end_date.and_hms_opt(0, 0, 0).unwrap()),
                chrono::Utc,
            );
            let records = assist_accounting_record::Entity::find()
                .filter(assist_accounting_record::Column::CreatedAt.gte(start))
                .filter(assist_accounting_record::Column::CreatedAt.lte(end))
                .all(&*self.db)
                .await?;

            let mut grouped: std::collections::HashMap<
                String,
                (Decimal, Decimal, Decimal, Decimal, i64),
            > = std::collections::HashMap::new();

            for record in records {
                let key = format!("{}_{}", record.five_dimension_id, record.account_subject_id);
                let entry = grouped.entry(key).or_insert((
                    Decimal::ZERO,
                    Decimal::ZERO,
                    Decimal::ZERO,
                    Decimal::ZERO,
                    0,
                ));
                entry.0 += record.debit_amount;
                entry.1 += record.credit_amount;
                entry.2 += record.quantity_meters;
                entry.3 += record.quantity_kg;
                entry.4 += 1;
            }

            let dimensions = assist_accounting_dimension::Entity::find()
                .all(&*self.db)
                .await?;

            for (key, (total_debit, total_credit, total_meters, total_kg, record_count)) in grouped
            {
                let parts: Vec<&str> = key.split('_').collect();
                if parts.len() >= 2 {
                    let dimension_id: i32 = parts[0].parse().unwrap_or(0);
                    let subject_id: i32 = parts[1].parse().unwrap_or(0);

                    let dimension_name = dimensions
                        .iter()
                        .find(|d| d.id == dimension_id)
                        .map(|d| d.dimension_name.clone())
                        .unwrap_or_else(|| "未知".to_string());

                    let existing = assist_accounting_summary::Entity::find()
                        .filter(
                            assist_accounting_summary::Column::AccountingPeriod
                                .eq(&accounting_period),
                        )
                        .filter(
                            assist_accounting_summary::Column::DimensionValueId.eq(dimension_id),
                        )
                        .filter(assist_accounting_summary::Column::AccountSubjectId.eq(subject_id))
                        .one(&*self.db)
                        .await?;

                    if let Some(summary) = existing {
                        let mut active: assist_accounting_summary::ActiveModel = summary.into();
                        let current_debit = active.total_debit.take();
                        let current_credit = active.total_credit.take();
                        let current_meters = active.total_quantity_meters.take();
                        let current_kg = active.total_quantity_kg.take();
                        let current_count = active.record_count.take();
                        let new_debit = current_debit.unwrap_or(Decimal::ZERO) + total_debit;
                        let new_credit = current_credit.unwrap_or(Decimal::ZERO) + total_credit;
                        let new_meters = current_meters.unwrap_or(Decimal::ZERO) + total_meters;
                        let new_kg = current_kg.unwrap_or(Decimal::ZERO) + total_kg;
                        let new_count = current_count.unwrap_or(0) + record_count;
                        active.total_debit = sea_orm::Set(new_debit);
                        active.total_credit = sea_orm::Set(new_credit);
                        active.total_quantity_meters = sea_orm::Set(new_meters);
                        active.total_quantity_kg = sea_orm::Set(new_kg);
                        active.record_count = sea_orm::Set(new_count);
                        active.updated_at = sea_orm::Set(chrono::Utc::now());
                        active.update(&*self.db).await?;
                    } else {
                        let new_summary = assist_accounting_summary::ActiveModel {
                            id: sea_orm::Set(0),
                            accounting_period: sea_orm::Set(accounting_period.clone()),
                            dimension_code: sea_orm::Set(
                                dimensions
                                    .iter()
                                    .find(|d| d.id == dimension_id)
                                    .map(|d| d.dimension_code.clone())
                                    .unwrap_or_default(),
                            ),
                            dimension_value_id: sea_orm::Set(dimension_id),
                            dimension_value_name: sea_orm::Set(dimension_name),
                            account_subject_id: sea_orm::Set(subject_id),
                            total_debit: sea_orm::Set(total_debit),
                            total_credit: sea_orm::Set(total_credit),
                            total_quantity_meters: sea_orm::Set(total_meters),
                            total_quantity_kg: sea_orm::Set(total_kg),
                            record_count: sea_orm::Set(record_count),
                            created_at: sea_orm::Set(chrono::Utc::now()),
                            updated_at: sea_orm::Set(chrono::Utc::now()),
                        };
                        new_summary.insert(&*self.db).await?;
                    }
                }
            }

            info!("辅助核算月度汇总生成成功，期间：{}", accounting_period);
        }

        Ok(())
    }

    /// 查询辅助核算明细（带过滤）
    pub async fn query_assist_records(
        &self,
        accounting_period: Option<&str>,
        dimension_code: Option<&str>,
        business_type: Option<&str>,
        warehouse_id: Option<i32>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<assist_accounting_record::Model>, u64), sea_orm::DbErr> {
        use sea_orm::ColumnTrait;

        let mut query = assist_accounting_record::Entity::find();

        // 按会计期间过滤（将期间转换为日期范围）
        if let Some(period) = accounting_period {
            if let Ok((year, month)) = parse_period(period) {
                let start_date = chrono::NaiveDate::from_ymd_opt(year, month, 1).map(|d| {
                    d.and_hms_opt(0, 0, 0)
                        .unwrap_or_else(|| d.and_hms_opt(0, 0, 0).unwrap())
                        .and_utc()
                });
                let end_date = if month == 12 {
                    chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
                } else {
                    chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
                }
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|d| d - chrono::Duration::seconds(1))
                .map(|d| d.and_utc());

                if let Some(start) = start_date {
                    query = query.filter(assist_accounting_record::Column::CreatedAt.gte(start));
                }
                if let Some(end) = end_date {
                    query = query.filter(assist_accounting_record::Column::CreatedAt.lte(end));
                }
            }
        }

        // 按维度过滤
        if let Some(dimension) = dimension_code {
            match dimension {
                "BATCH" => {
                    // 批次过滤通过five_dimension_id或其他方式
                }
                "COLOR" => {
                    // 色号过滤
                }
                "DYE_LOT" => {
                    // 缸号过滤
                }
                "GRADE" => {
                    // 等级过滤
                }
                "WORKSHOP" => {
                    // 车间过滤
                }
                "WAREHOUSE" => {
                    // 仓库过滤（已有warehouse_id过滤）
                }
                "CUSTOMER" => {
                    // 客户过滤
                }
                "SUPPLIER" => {
                    // 供应商过滤
                }
                _ => {}
            }
        }

        if let Some(biz_type) = business_type {
            query = query.filter(assist_accounting_record::Column::BusinessType.eq(biz_type));
        }

        if let Some(wid) = warehouse_id {
            query = query.filter(assist_accounting_record::Column::WarehouseId.eq(wid));
        }

        // 分页查询
        let paginator = query.paginate(&*self.db, page_size);
        let total = paginator.num_items().await?;
        let records = paginator.fetch_page(page).await?;

        Ok((records, total))
    }

    /// 删除辅助核算记录（通常用于冲销）
    #[allow(dead_code)]
    pub async fn delete_assist_record(&self, id: i32) -> Result<(), sea_orm::DbErr> {
        assist_accounting_record::Entity::delete_many()
            .filter(assist_accounting_record::Column::Id.eq(id))
            .exec(&*self.db)
            .await?;
        Ok(())
    }

    /// 查询所有启用的辅助核算维度
    pub async fn list_dimensions(
        &self,
    ) -> Result<Vec<assist_accounting_dimension::Model>, sea_orm::DbErr> {
        assist_accounting_dimension::Entity::find()
            .filter(assist_accounting_dimension::Column::IsActive.eq(true))
            .order_by(assist_accounting_dimension::Column::SortOrder, Order::Asc)
            .all(&*self.db)
            .await
    }
}

fn parse_period(period: &str) -> Result<(i32, u32), String> {
    let parts: Vec<&str> = period.split('-').collect();
    if parts.len() != 2 {
        return Err("期间格式错误，应为 YYYY-MM".to_string());
    }
    let year: i32 = parts[0].parse().map_err(|_| "年份解析错误")?;
    let month: u32 = parts[1].parse().map_err(|_| "月份解析错误")?;
    if month < 1 || month > 12 {
        return Err("月份必须在1-12之间".to_string());
    }
    Ok((year, month))
}
