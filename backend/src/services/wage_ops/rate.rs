//! 工序工价 Service impl 子模块（wage_ops/rate）
//!
//! 批次 490 D10-4a 拆分：从原 `wage_service.rs` L265-587 迁移。
//! 包含 WageRateService 的 11 个方法（new 留在 facade）：
//! - generate_rate_no（私有，生成单号 PWR-YYYYMMDDHHMMSS-NNN）
//! - create / update / delete（CRUD + 业务校验）
//! - activate / disable / transition_status（状态机 draft→active→disabled）
//! - get_by_id / get_by_no / get_effective_by_route / list（查询）
//!
//! 业务规则：
//! - 工价类型合法（piece/time/mixed），计件必填 piece_price>0，计时必填 time_price>0
//! - 等级系数范围 [0, 1]，失效日期必须晚于生效日期
//! - 仅 draft 状态可更新/删除，状态流转校验 from→to 合法性
//! - 软删除（is_deleted = true）

use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use crate::models::process_route::Entity as RouteEntity;
use crate::models::process_wage_rate::{
    self, ActiveModel as RateActiveModel, Entity as RateEntity, Model as RateModel,
};
use crate::models::status::wage_rate_status;
use crate::models::status::wage_type;
use crate::utils::error::AppError;

use crate::services::wage_service::{
    CreateWageRateRequest, UpdateWageRateRequest, WageRateQuery, WageRateService,
};

impl WageRateService {
    /// 生成工价单号：PWR-YYYYMMDDHHMMSS-NNN
    fn generate_rate_no() -> String {
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_6_digit() % 1000;
        format!("PWR-{}-{:03}", timestamp, random)
    }

    /// 创建工价
    pub async fn create(&self, req: CreateWageRateRequest) -> Result<RateModel, AppError> {
        // 业务校验：工序路线存在
        let route = RouteEntity::find_by_id(req.process_route_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::business(format!("工序路线 {} 不存在", req.process_route_id))
            })?;

        // 业务校验：工价类型合法
        let wage_type_value = req
            .wage_type
            .unwrap_or_else(|| wage_type::PIECE.to_string());
        if wage_type_value != wage_type::PIECE
            && wage_type_value != wage_type::TIME
            && wage_type_value != wage_type::MIXED
        {
            return Err(AppError::business(format!(
                "工价类型必须是 {} / {} / {}",
                wage_type::PIECE,
                wage_type::TIME,
                wage_type::MIXED
            )));
        }

        // 业务校验：单价非负
        let piece_price = req.piece_price.unwrap_or(Decimal::ZERO);
        let time_price = req.time_price.unwrap_or(Decimal::ZERO);
        if piece_price < Decimal::ZERO {
            return Err(AppError::business("计件单价不能为负"));
        }
        if time_price < Decimal::ZERO {
            return Err(AppError::business("计时单价不能为负"));
        }

        // 业务校验：计件类型必须有计件单价，计时类型必须有计时单价
        if wage_type_value == wage_type::PIECE && piece_price <= Decimal::ZERO {
            return Err(AppError::business("计件工价必须设置计件单价 > 0"));
        }
        if wage_type_value == wage_type::TIME && time_price <= Decimal::ZERO {
            return Err(AppError::business("计时工价必须设置计时单价 > 0"));
        }
        if wage_type_value == wage_type::MIXED
            && piece_price <= Decimal::ZERO
            && time_price <= Decimal::ZERO
        {
            return Err(AppError::business("混合工价必须设置计件或计时单价 > 0"));
        }

        // 业务校验：等级系数范围 [0, 1]
        let grade_a_ratio = req.grade_a_ratio.unwrap_or_else(|| Decimal::new(10, 1)); // 1.0
        let grade_b_ratio = req.grade_b_ratio.unwrap_or_else(|| Decimal::new(8, 1)); // 0.8
        let grade_c_ratio = req.grade_c_ratio.unwrap_or(Decimal::ZERO);
        for (name, value) in [
            ("A 级", grade_a_ratio),
            ("B 级", grade_b_ratio),
            ("C 级", grade_c_ratio),
        ] {
            if value < Decimal::ZERO || value > Decimal::new(10, 1) {
                return Err(AppError::business(format!(
                    "{} 等级系数必须在 [0, 1] 范围内",
                    name
                )));
            }
        }

        // 业务校验：失效日期必须晚于生效日期
        if let Some(expiry) = req.expiry_date {
            if expiry <= req.effective_date {
                return Err(AppError::business("失效日期必须晚于生效日期"));
            }
        }

        let rate_no = Self::generate_rate_no();
        let now = crate::utils::date_utils::utc_now_fixed();

        let active = RateActiveModel {
            id: Default::default(),
            rate_no: Set(rate_no),
            process_route_id: Set(req.process_route_id),
            route_code: Set(route.route_code.clone()),
            route_name: Set(route.route_name.clone()),
            wage_type: Set(wage_type_value),
            piece_price: Set(piece_price),
            time_price: Set(time_price),
            grade_a_ratio: Set(grade_a_ratio),
            grade_b_ratio: Set(grade_b_ratio),
            grade_c_ratio: Set(grade_c_ratio),
            effective_date: Set(req.effective_date),
            expiry_date: Set(req.expiry_date),
            workshop: Set(req.workshop),
            status: Set(wage_rate_status::DRAFT.to_string()),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("工价创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新工价（仅 draft 状态可更新）
    pub async fn update(
        &self,
        id: i32,
        req: UpdateWageRateRequest,
    ) -> Result<RateModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != wage_rate_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可更新，当前状态: {}",
                model.status
            )));
        }

        // 记录原 effective_date（在 model.into() 之前取出，避免 ActiveValue 取值复杂）
        let original_effective_date = model.effective_date;

        let mut active: RateActiveModel = model.into();

        if let Some(v) = req.wage_type {
            if v != wage_type::PIECE && v != wage_type::TIME && v != wage_type::MIXED {
                return Err(AppError::business("工价类型不合法"));
            }
            active.wage_type = Set(v);
        }
        if let Some(v) = req.piece_price {
            if v < Decimal::ZERO {
                return Err(AppError::business("计件单价不能为负"));
            }
            active.piece_price = Set(v);
        }
        if let Some(v) = req.time_price {
            if v < Decimal::ZERO {
                return Err(AppError::business("计时单价不能为负"));
            }
            active.time_price = Set(v);
        }
        if let Some(v) = req.grade_a_ratio {
            if v < Decimal::ZERO || v > Decimal::new(10, 1) {
                return Err(AppError::business("A 级等级系数必须在 [0, 1] 范围内"));
            }
            active.grade_a_ratio = Set(v);
        }
        if let Some(v) = req.grade_b_ratio {
            if v < Decimal::ZERO || v > Decimal::new(10, 1) {
                return Err(AppError::business("B 级等级系数必须在 [0, 1] 范围内"));
            }
            active.grade_b_ratio = Set(v);
        }
        if let Some(v) = req.grade_c_ratio {
            if v < Decimal::ZERO || v > Decimal::new(10, 1) {
                return Err(AppError::business("C 级等级系数必须在 [0, 1] 范围内"));
            }
            active.grade_c_ratio = Set(v);
        }
        if let Some(v) = req.effective_date {
            active.effective_date = Set(v);
        }
        if let Some(v) = req.expiry_date {
            // 失效日期必须晚于生效日期（用原始 effective_date 或请求中的新 effective_date 比较）
            let effective = req.effective_date.unwrap_or(original_effective_date);
            if v <= effective {
                return Err(AppError::business("失效日期必须晚于生效日期"));
            }
            active.expiry_date = Set(Some(v));
        }
        if let Some(v) = req.workshop {
            active.workshop = Set(Some(v));
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(Some(v));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 软删除工价（仅 draft 状态可删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != wage_rate_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可删除，当前状态: {}",
                model.status
            )));
        }
        let mut active: RateActiveModel = model.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 启用工价（draft → active）
    pub async fn activate(&self, id: i32) -> Result<RateModel, AppError> {
        self.transition_status(id, wage_rate_status::DRAFT, wage_rate_status::ACTIVE)
            .await
    }

    /// 停用工价（active → disabled）
    pub async fn disable(&self, id: i32) -> Result<RateModel, AppError> {
        self.transition_status(id, wage_rate_status::ACTIVE, wage_rate_status::DISABLED)
            .await
    }

    /// 状态流转通用方法
    async fn transition_status(
        &self,
        id: i32,
        from: &str,
        to: &str,
    ) -> Result<RateModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != from {
            return Err(AppError::business(format!(
                "状态流转非法：当前 {}，期望 {}",
                model.status, from
            )));
        }
        let mut active: RateActiveModel = model.into();
        active.status = Set(to.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<RateModel, AppError> {
        let model = RateEntity::find_by_id(id)
            .filter(process_wage_rate::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("工价 {} 不存在", id)))?;
        Ok(model)
    }

    /// 按单号查询
    pub async fn get_by_no(&self, rate_no: &str) -> Result<RateModel, AppError> {
        let model = RateEntity::find()
            .filter(process_wage_rate::Column::RateNo.eq(rate_no))
            .filter(process_wage_rate::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("工价单号 {} 不存在", rate_no)))?;
        Ok(model)
    }

    /// 查询工序当前生效的工价
    pub async fn get_effective_by_route(
        &self,
        process_route_id: i32,
        on_date: chrono::NaiveDate,
    ) -> Result<Option<RateModel>, AppError> {
        let model = RateEntity::find()
            .filter(process_wage_rate::Column::ProcessRouteId.eq(process_route_id))
            .filter(process_wage_rate::Column::Status.eq(wage_rate_status::ACTIVE))
            .filter(process_wage_rate::Column::IsDeleted.eq(false))
            .filter(process_wage_rate::Column::EffectiveDate.lte(on_date))
            .filter(
                sea_orm::Condition::any()
                    .add(process_wage_rate::Column::ExpiryDate.is_null())
                    .add(process_wage_rate::Column::ExpiryDate.gt(on_date)),
            )
            .order_by_desc(process_wage_rate::Column::EffectiveDate)
            .one(&*self.db)
            .await?;
        Ok(model)
    }

    /// 分页查询
    pub async fn list(
        &self,
        query: WageRateQuery,
    ) -> Result<(Vec<RateModel>, u64), AppError> {
        let page = query.page.unwrap_or(1).clamp(1, 1000);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

        let mut q = RateEntity::find()
            .filter(process_wage_rate::Column::IsDeleted.eq(false));

        if let Some(v) = query.route_code {
            q = q.filter(process_wage_rate::Column::RouteCode.eq(v));
        }
        if let Some(v) = query.process_route_id {
            q = q.filter(process_wage_rate::Column::ProcessRouteId.eq(v));
        }
        if let Some(v) = query.workshop {
            q = q.filter(process_wage_rate::Column::Workshop.eq(v));
        }
        if let Some(v) = query.status {
            q = q.filter(process_wage_rate::Column::Status.eq(v));
        }

        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_desc(process_wage_rate::Column::CreatedAt)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
    }
}
