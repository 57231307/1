//! 工资计算 Service impl 子模块（wage_ops/calculation）
//!
//! 批次 490 D10-4a 拆分：从原 `wage_service.rs` L857-1180 迁移。
//! 包含 WageCalculationService 的 8 个方法（new 留在 facade）+ 3 个私有 helper struct：
//! - calculate（触发计算：查工序记录 → 匹配工价 → 计算工资 → 生成明细 + 汇总）
//! - validate_wage_record（私有，校验仅 draft 状态可计算）
//! - clear_or_check_details（私有，recalculate 删旧/检查已有明细）
//! - load_step_records（私有，查询周期内 completed 工序记录）
//! - find_wage_rate_for_step（私有，按工序路线匹配生效工价）
//! - process_step_and_accumulate（私有，计算单工序工资 + 派发按工人创建明细）
//! - create_wage_details_for_workers（私有，为每个工人创建明细 + 累计）
//! - update_wage_record_summary（私有，更新工资记录汇总）
//!
//! Helper struct（仅本模块内部使用）：
//! - WageTotals：计算累计汇总（总额/合格产量/工时/工人集合/工序集合/明细数）
//! - StepWageComputed：单工序工资计算结果（等级/系数/计件/计时/总额）
//! - WorkerDetailContext：工人明细创建上下文（消除 too_many_arguments）

use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use std::collections::HashSet;

use crate::models::process_step_record::{self, Entity as StepEntity, Model as StepModel};
use crate::models::process_wage_rate::{self, Entity as RateEntity, Model as RateModel};
use crate::models::status::wage_rate_status;
use crate::models::status::wage_record_status;
use crate::models::wage_record::{
    self, ActiveModel as RecordActiveModel, Entity as RecordEntity, Model as RecordModel,
};
use crate::models::wage_record_detail::{self, ActiveModel as DetailActiveModel};
use crate::utils::error::AppError;

// 复用 facade 的纯函数（保持单一来源，避免逻辑重复）
use crate::services::wage_service::{
    calculate_wage_for_step, compute_qualification_rate, naive_date_to_date_time_tz,
    naive_date_to_end_of_day_tz, parse_worker_ids, parse_worker_names,
    split_wage_among_workers, CalculateWageRequest, WageCalculationService,
};

/// 工资计算累计：用于 calculate 拆分时在 helper 间传递汇总
struct WageTotals {
    total_amount: Decimal,
    total_qualified: Decimal,
    total_minutes: i64,
    worker_set: HashSet<i32>,
    step_set: HashSet<i32>,
    detail_count: u64,
}

/// 单工序工资计算结果：用于在 helper 间传递 5 个返回值
struct StepWageComputed {
    grade: String,
    grade_ratio: Decimal,
    piece_wage: Decimal,
    time_wage: Decimal,
    wage_amount: Decimal,
}

/// 工人工资明细创建上下文：封装 step/rate/computed 等参数消除 too_many_arguments 警告
struct WorkerDetailContext<'a> {
    step: &'a StepModel,
    rate: &'a RateModel,
    computed: &'a StepWageComputed,
    wage_record_id: i32,
    now: chrono::DateTime<chrono::FixedOffset>,
    worker_ids: &'a [i32],
    worker_names: &'a [String],
}

impl WageCalculationService {
    /// 触发工资计算
    ///
    /// 业务流程：查询周期内 completed 工序记录 → 按工序匹配生效工价
    /// → 计算每个工人工资 → 生成明细 + 汇总工资记录
    pub async fn calculate(
        &self,
        wage_record_id: i32,
        req: CalculateWageRequest,
    ) -> Result<RecordModel, AppError> {
        let record = self.validate_wage_record(wage_record_id).await?;
        self.clear_or_check_details(wage_record_id, req.recalculate.unwrap_or(false))
            .await?;
        let step_records = self.load_step_records(&record).await?;

        // 2. 按工序路线匹配生效工价，生成工资明细
        let now = crate::utils::date_utils::utc_now_fixed();
        let mut totals = WageTotals {
            total_amount: Decimal::ZERO,
            total_qualified: Decimal::ZERO,
            total_minutes: 0,
            worker_set: HashSet::new(),
            step_set: HashSet::new(),
            detail_count: 0,
        };

        for step in &step_records {
            // 查找工价（按工序路线匹配）；无 route_id 或无生效工价时跳过
            let rate = match self.find_wage_rate_for_step(step, &record).await? {
                Some(r) => r,
                None => continue,
            };
            self.process_step_and_accumulate(step, &rate, wage_record_id, now, &mut totals)
                .await?;
        }

        if totals.detail_count == 0 {
            return Err(AppError::business(
                "未生成任何工资明细，请检查工价方案配置或工序记录状态",
            ));
        }

        let updated = self.update_wage_record_summary(record, totals, now).await?;
        Ok(updated)
    }

    /// 加载工资记录并校验：仅 draft 状态可计算
    async fn validate_wage_record(
        &self,
        wage_record_id: i32,
    ) -> Result<RecordModel, AppError> {
        let record = RecordEntity::find_by_id(wage_record_id)
            .filter(wage_record::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("工资记录 {} 不存在", wage_record_id)))?;

        // 业务校验：仅 draft 状态可计算
        if record.status != wage_record_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可触发计算，当前状态: {}",
                record.status
            )));
        }
        Ok(record)
    }

    /// recalculate=true 删除旧明细 / 否则检查已有明细
    async fn clear_or_check_details(
        &self,
        wage_record_id: i32,
        recalculate: bool,
    ) -> Result<(), AppError> {
        // 重新计算时先删除旧明细
        if recalculate {
            wage_record_detail::Entity::delete_many()
                .filter(wage_record_detail::Column::WageRecordId.eq(wage_record_id))
                .exec(&*self.db)
                .await?;
        } else {
            // 不重新计算时，检查是否已有明细
            let existing = wage_record_detail::Entity::find()
                .filter(wage_record_detail::Column::WageRecordId.eq(wage_record_id))
                .filter(wage_record_detail::Column::IsDeleted.eq(false))
                .count(&*self.db)
                .await?;
            if existing > 0 {
                return Err(AppError::business(format!(
                    "工资记录已有 {} 条明细，如需重新计算请设置 recalculate=true",
                    existing
                )));
            }
        }
        Ok(())
    }

    /// 查询周期内 completed 工序记录
    async fn load_step_records(&self, record: &RecordModel) -> Result<Vec<StepModel>, AppError> {
        // 1. 查询周期内 completed 状态的工序记录
        // 工序记录无 workshop 字段，车间维度在工价匹配阶段通过 process_wage_rate.workshop 间接关联
        let period_start_tz = naive_date_to_date_time_tz(record.period_start);
        let period_end_tz = naive_date_to_end_of_day_tz(record.period_end);
        let step_query = StepEntity::find()
            .filter(process_step_record::Column::IsDeleted.eq(false))
            .filter(process_step_record::Column::Status.eq("completed"))
            // 周期内：start_at 在 [period_start 00:00, period_end 23:59]
            .filter(process_step_record::Column::StartAt.gte(period_start_tz))
            .filter(process_step_record::Column::StartAt.lte(period_end_tz));

        let step_records: Vec<StepModel> = step_query.all(&*self.db).await?;
        if step_records.is_empty() {
            return Err(AppError::business(format!(
                "周期 {} ~ {} 内无已完成的工序记录，无法计算工资",
                record.period_start, record.period_end
            )));
        }
        Ok(step_records)
    }

    /// 按工序路线匹配生效工价（route_id 缺失或无匹配返回 None，调用方 continue）
    async fn find_wage_rate_for_step(
        &self,
        step: &StepModel,
        record: &RecordModel,
    ) -> Result<Option<RateModel>, AppError> {
        // 查找工价（按工序路线匹配）
        let route_id = match step.process_route_id {
            Some(id) => id,
            None => return Ok(None), // 无工序路线的记录跳过
        };

        // 工价匹配条件：工序路线 + 当前生效 + 车间过滤
        // effective_date 和 expiry_date 是 NaiveDate 类型，直接用 NaiveDate 比较
        let mut rate_query = RateEntity::find()
            .filter(process_wage_rate::Column::ProcessRouteId.eq(route_id))
            .filter(process_wage_rate::Column::Status.eq(wage_rate_status::ACTIVE))
            .filter(process_wage_rate::Column::IsDeleted.eq(false))
            .filter(process_wage_rate::Column::EffectiveDate.lte(record.period_end))
            .filter(
                sea_orm::Condition::any()
                    .add(process_wage_rate::Column::ExpiryDate.is_null())
                    .add(process_wage_rate::Column::ExpiryDate.gt(record.period_start)),
            );

        if let Some(ref workshop) = record.workshop {
            rate_query = rate_query.filter(
                sea_orm::Condition::any()
                    .add(process_wage_rate::Column::Workshop.is_null())
                    .add(process_wage_rate::Column::Workshop.eq(workshop)),
            );
        }

        let rate = rate_query
            .order_by_desc(process_wage_rate::Column::EffectiveDate)
            .one(&*self.db)
            .await?;
        Ok(rate) // None 表示无生效工价，调用方 continue
    }

    /// 计算单工序工资 + 派发按工人创建明细
    async fn process_step_and_accumulate(
        &self,
        step: &StepModel,
        rate: &RateModel,
        wage_record_id: i32,
        now: chrono::DateTime<chrono::FixedOffset>,
        totals: &mut WageTotals,
    ) -> Result<(), AppError> {
        // 3. 计算工资
        let (grade, grade_ratio, piece_wage, time_wage, wage_amount) = calculate_wage_for_step(
            rate,
            step.actual_quantity,
            step.qualified_quantity,
            step.duration_minutes,
        );
        let computed = StepWageComputed {
            grade,
            grade_ratio,
            piece_wage,
            time_wage,
            wage_amount,
        };

        // 4. 按工人 IDs 分配工资（多人共同完成时按人均分配）
        let worker_ids = parse_worker_ids(&step.worker_ids);
        if worker_ids.is_empty() {
            return Ok(()); // 无工人的记录跳过
        }
        let worker_names = parse_worker_names(&step.worker_names);

        self.create_wage_details_for_workers(
            WorkerDetailContext {
                step,
                rate,
                computed: &computed,
                wage_record_id,
                now,
                worker_ids: &worker_ids,
                worker_names: &worker_names,
            },
            totals,
        )
        .await
    }

    /// 为每个工人创建工资明细 + 累计（多人按人均分配）
    async fn create_wage_details_for_workers(
        &self,
        ctx: WorkerDetailContext<'_>,
        totals: &mut WageTotals,
    ) -> Result<(), AppError> {
        let worker_count = ctx.worker_ids.len();
        let per_worker_piece = split_wage_among_workers(ctx.computed.piece_wage, worker_count);
        let per_worker_time = split_wage_among_workers(ctx.computed.time_wage, worker_count);
        let per_worker_amount = split_wage_among_workers(ctx.computed.wage_amount, worker_count);

        for (idx, &worker_id) in ctx.worker_ids.iter().enumerate() {
            let worker_name = ctx.worker_names.get(idx).cloned();

            let detail = DetailActiveModel {
                id: Default::default(),
                wage_record_id: Set(ctx.wage_record_id),
                step_record_id: Set(ctx.step.id),
                flow_card_id: Set(Some(ctx.step.flow_card_id)),
                dye_lot_no: Set(None), // dye_lot_no 在 production_flow_card 上，这里留空
                process_route_id: Set(ctx.step.process_route_id),
                route_code: Set(Some(ctx.step.route_code.clone())),
                route_name: Set(Some(ctx.step.route_name.clone())),
                process_type: Set(Some(ctx.step.process_type.clone())),
                worker_id: Set(worker_id),
                worker_name: Set(worker_name),
                equipment_id: Set(ctx.step.equipment_id),
                equipment_name: Set(ctx.step.equipment_name.clone()),
                wage_type: Set(ctx.rate.wage_type.clone()),
                grade: Set(ctx.computed.grade.clone()),
                actual_quantity: Set(ctx.step.actual_quantity.unwrap_or(Decimal::ZERO)
                    / Decimal::from(worker_count)),
                qualified_quantity: Set(ctx.step.qualified_quantity.unwrap_or(Decimal::ZERO)
                    / Decimal::from(worker_count)),
                qualification_rate: Set(compute_qualification_rate(
                    ctx.step.actual_quantity,
                    ctx.step.qualified_quantity,
                )),
                piece_price: Set(ctx.rate.piece_price),
                time_price: Set(ctx.rate.time_price),
                grade_ratio: Set(ctx.computed.grade_ratio),
                duration_minutes: Set(ctx.step.duration_minutes.unwrap_or(0) / worker_count as i32),
                piece_wage: Set(per_worker_piece),
                time_wage: Set(per_worker_time),
                wage_amount: Set(per_worker_amount),
                remarks: Set(None),
                is_deleted: Set(false),
                created_at: Set(ctx.now),
                updated_at: Set(ctx.now),
            };
            detail.insert(&*self.db).await?;
            totals.detail_count += 1;
            totals.total_amount += per_worker_amount;
            totals.total_qualified += ctx.step.qualified_quantity.unwrap_or(Decimal::ZERO)
                / Decimal::from(worker_count);
            totals.total_minutes += (ctx.step.duration_minutes.unwrap_or(0) as i64) / worker_count as i64;
            totals.worker_set.insert(worker_id);
            totals.step_set.insert(ctx.step.id);
        }
        Ok(())
    }

    /// 更新工资记录汇总（工人数 / 工序数 / 合格产量 / 工时 / 总额）
    async fn update_wage_record_summary(
        &self,
        record: RecordModel,
        totals: WageTotals,
        now: chrono::DateTime<chrono::FixedOffset>,
    ) -> Result<RecordModel, AppError> {
        // 5. 更新工资记录汇总
        let mut active: RecordActiveModel = record.into();
        active.total_workers = Set(totals.worker_set.len() as i32);
        active.total_step_records = Set(totals.step_set.len() as i32);
        active.total_qualified_quantity = Set(totals.total_qualified);
        active.total_duration_minutes = Set(totals.total_minutes as i32);
        active.total_amount = Set(totals.total_amount);
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }
}
