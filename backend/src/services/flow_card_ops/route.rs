//! 工序路线模板 Service impl 子模块（flow_card_ops/route）
//!
//! D10 第 5 批拆分：从原 flow_card_service.rs 迁移 ProcessRouteService 的 5 个业务方法
//!（create / update / delete / get_by_id / list_active）。new 构造函数保留在 facade。

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use crate::models::process_route::{
    self, ActiveModel as RouteActiveModel, Entity as RouteEntity, Model as RouteModel,
};
use crate::services::flow_card_service::{
    CreateProcessRouteRequest, ProcessRouteService, UpdateProcessRouteRequest,
};
use crate::utils::error::AppError;

impl ProcessRouteService {
    /// 创建工序路线
    pub async fn create(&self, req: CreateProcessRouteRequest) -> Result<RouteModel, AppError> {
        // 业务校验：工序编码格式
        let code = req.route_code.trim().to_uppercase();
        if code.is_empty() {
            return Err(AppError::business("工序编码不能为空"));
        }
        if !(1..=32).contains(&code.len()) {
            return Err(AppError::business("工序编码长度 1-32"));
        }

        // 业务校验：工序序号必须为正
        if req.seq < 1 {
            return Err(AppError::business("工序序号必须 >= 1"));
        }

        // 业务校验：工序类型合法
        let valid_types = ["pretreat", "dye", "print", "finish", "inspect", "other"];
        if !valid_types.contains(&req.process_type.as_str()) {
            return Err(AppError::business(format!(
                "工序类型必须是 {:?} 之一",
                valid_types
            )));
        }

        // 校验编码唯一
        let existing = RouteEntity::find()
            .filter(process_route::Column::RouteCode.eq(code.clone()))
            .filter(process_route::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?;
        if existing.is_some() {
            return Err(AppError::business(format!("工序编码 {} 已存在", code)));
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        let active = RouteActiveModel {
            id: Default::default(),
            route_code: Set(code),
            route_name: Set(req.route_name),
            seq: Set(req.seq),
            process_type: Set(req.process_type),
            default_duration_minutes: Set(req.default_duration_minutes),
            require_scan: Set(req.require_scan.unwrap_or(true)),
            is_active: Set(true),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("工序路线创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新工序路线
    pub async fn update(
        &self,
        id: i32,
        req: UpdateProcessRouteRequest,
    ) -> Result<RouteModel, AppError> {
        let model = self.get_by_id(id).await?;
        let mut active: RouteActiveModel = model.into();

        if let Some(v) = req.route_name {
            active.route_name = Set(v);
        }
        if let Some(v) = req.seq {
            if v < 1 {
                return Err(AppError::business("工序序号必须 >= 1"));
            }
            active.seq = Set(v);
        }
        if let Some(v) = req.process_type {
            let valid_types = ["pretreat", "dye", "print", "finish", "inspect", "other"];
            if !valid_types.contains(&v.as_str()) {
                return Err(AppError::business("工序类型不合法"));
            }
            active.process_type = Set(v);
        }
        if let Some(v) = req.default_duration_minutes {
            if v < 0 {
                return Err(AppError::business("默认工时不能为负"));
            }
            active.default_duration_minutes = Set(Some(v));
        }
        if let Some(v) = req.require_scan {
            active.require_scan = Set(v);
        }
        if let Some(v) = req.is_active {
            active.is_active = Set(v);
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(Some(v));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 软删除工序路线
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        let mut active: RouteActiveModel = model.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<RouteModel, AppError> {
        let model = RouteEntity::find_by_id(id)
            .filter(process_route::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("工序路线 {} 不存在", id)))?;
        Ok(model)
    }

    /// 查询所有启用的工序路线（按序号排序）
    pub async fn list_active(&self) -> Result<Vec<RouteModel>, AppError> {
        let list = RouteEntity::find()
            .filter(process_route::Column::IsDeleted.eq(false))
            .filter(process_route::Column::IsActive.eq(true))
            .order_by_asc(process_route::Column::Seq)
            .all(&*self.db)
            .await?;
        Ok(list)
    }
}
