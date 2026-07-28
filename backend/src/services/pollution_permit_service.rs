//! 排污许可证管理 Service
//!
//! V15 P1 batch-08 缺陷 18：排污许可证登记与到期预警
//! 依据：《环境保护法》第45条 + 《排污许可管理条例》第24条
//!
//! 真实业务：
//! - 登记排污许可证信息（编号/类型/有效期/许可排放量）
//! - 到期前 30/60/90 天三级预警（提醒延续申请）
//! - 状态机：active(有效) → expiring_soon(即将到期) → expired(过期) / revoked(吊销)

use crate::models::pollution_permit::{
    self, ActiveModel as PermitActiveModel, Entity as PermitEntity, Model as PermitModel,
};
use crate::utils::error::AppError;
use chrono::NaiveDate;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use serde::Deserialize;
use std::sync::Arc;

/// 创建排污许可证请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreatePollutionPermitRequest {
    pub permit_no: String,
    /// 许可证类型：wastewater(废水) / exhaust(废气) / solid_waste(固废)
    pub permit_type: String,
    pub permit_category: Option<String>,
    pub issue_date: NaiveDate,
    pub expiry_date: NaiveDate,
    pub issuing_authority: String,
    pub permitted_capacity: Option<rust_decimal::Decimal>,
    pub capacity_unit: Option<String>,
    pub permitted_pollutants: Option<serde_json::Value>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 排污许可证查询参数
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PollutionPermitQuery {
    pub permit_type: Option<String>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 到期预警级别
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpiryWarningLevel {
    /// 未到期（剩余 > 90 天）
    Normal,
    /// 90 天预警
    Warning90Days,
    /// 60 天预警
    Warning60Days,
    /// 30 天预警（《排污许可管理条例》第24条申请延续最低期限）
    Warning30Days,
    /// 已过期
    Expired,
}

impl ExpiryWarningLevel {
    /// 中文描述
    pub fn desc(&self) -> &'static str {
        match self {
            Self::Normal => "正常",
            Self::Warning90Days => "90天到期预警",
            Self::Warning60Days => "60天到期预警",
            Self::Warning30Days => "30天到期预警",
            Self::Expired => "已过期",
        }
    }

    /// 是否需要预警
    pub fn needs_warning(&self) -> bool {
        !matches!(self, Self::Normal)
    }
}

/// 到期预警结果
#[derive(Debug, Clone)]
pub struct PermitExpiryWarning {
    pub permit: PermitModel,
    pub level: ExpiryWarningLevel,
    pub days_until_expiry: i64,
}

pub struct PollutionPermitService {
    db: Arc<DatabaseConnection>,
}

impl PollutionPermitService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建排污许可证（业务校验：许可证编号唯一；到期日期 > 发证日期；许可证类型合法（wastewater/exhaust/solid_waste））
    pub async fn create(&self, req: CreatePollutionPermitRequest) -> Result<PermitModel, AppError> {
        Self::validate_permit_type(&req.permit_type)?;
        if req.expiry_date <= req.issue_date {
            return Err(AppError::bad_request("到期日期必须晚于发证日期"));
        }

        // 校验许可证编号唯一性
        if PermitEntity::find()
            .filter(pollution_permit::Column::PermitNo.eq(&req.permit_no))
            .one(&*self.db)
            .await?
            .is_some()
        {
            return Err(AppError::business(format!(
                "排污许可证编号 {} 已存在",
                req.permit_no
            )));
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        let active = PermitActiveModel {
            permit_no: Set(req.permit_no),
            permit_type: Set(req.permit_type),
            permit_category: Set(req.permit_category),
            issue_date: Set(req.issue_date),
            expiry_date: Set(req.expiry_date),
            issuing_authority: Set(req.issuing_authority),
            permitted_capacity: Set(req.permitted_capacity),
            capacity_unit: Set(req.capacity_unit),
            permitted_pollutants: Set(req.permitted_pollutants),
            status: Set("active".to_string()),
            remarks: Set(req.remarks),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("排污许可证创建失败: {}", e)))?;
        Ok(result)
    }

    /// 查询排污许可证列表（分页）
    pub async fn list(
        &self,
        params: PollutionPermitQuery,
    ) -> Result<(Vec<PermitModel>, u64), AppError> {
        let mut query = PermitEntity::find();

        if let Some(permit_type) = &params.permit_type {
            query = query.filter(pollution_permit::Column::PermitType.eq(permit_type));
        }
        if let Some(status) = &params.status {
            query = query.filter(pollution_permit::Column::Status.eq(status));
        }

        let total = query.clone().count(&*self.db).await?;

        let page = params.page.unwrap_or(1).max(1);
        let page_size = params.page_size.unwrap_or(20).clamp(1, 200);

        let list = query
            .order_by_desc(pollution_permit::Column::ExpiryDate)
            .offset((page - 1) * page_size)
            .limit(page_size)
            .all(&*self.db)
            .await?;

        Ok((list, total))
    }

    /// 获取排污许可证详情
    pub async fn get_by_id(&self, id: i32) -> Result<PermitModel, AppError> {
        PermitEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("排污许可证 {} 不存在", id)))
    }

    /// 吊销排污许可证
    pub async fn revoke(&self, id: i32, _operator_id: i32) -> Result<PermitModel, AppError> {
        let permit = self.get_by_id(id).await?;
        if permit.status == "revoked" {
            return Err(AppError::business(format!(
                "许可证 {} 已吊销，不可重复操作",
                permit.permit_no
            )));
        }

        let mut active: PermitActiveModel = permit.into();
        active.status = Set("revoked".to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 扫描即将到期/已过期的许可证并生成预警（业务规则（《排污许可管理条例》第24条）：到期前 90/60/30 天三级预警；已过期的许可证状态自动更新为 expired）
    pub async fn scan_expiry_warnings(&self) -> Result<Vec<PermitExpiryWarning>, AppError> {
        let today = chrono::Local::now().date_naive();
        let all_permits = PermitEntity::find()
            .filter(pollution_permit::Column::Status.is_in(["active".to_string()]))
            .all(&*self.db)
            .await?;

        let mut warnings = Vec::new();
        for permit in all_permits {
            let days_until_expiry = (permit.expiry_date - today).num_days();
            let level = Self::classify_expiry_level(days_until_expiry);

            if level.needs_warning() {
                // 已过期的许可证自动更新状态
                if level == ExpiryWarningLevel::Expired {
                    let mut active: PermitActiveModel = permit.clone().into();
                    active.status = Set("expired".to_string());
                    active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
                    let _ = active.update(&*self.db).await;
                }

                warnings.push(PermitExpiryWarning {
                    permit,
                    level,
                    days_until_expiry,
                });
            }
        }

        // 按到期日期升序排列（最紧急的在前）
        warnings.sort_by_key(|w| w.days_until_expiry);
        Ok(warnings)
    }

    /// 根据剩余天数判定预警级别（纯函数）
    pub fn classify_expiry_level(days_until_expiry: i64) -> ExpiryWarningLevel {
        if days_until_expiry < 0 {
            ExpiryWarningLevel::Expired
        } else if days_until_expiry <= 30 {
            ExpiryWarningLevel::Warning30Days
        } else if days_until_expiry <= 60 {
            ExpiryWarningLevel::Warning60Days
        } else if days_until_expiry <= 90 {
            ExpiryWarningLevel::Warning90Days
        } else {
            ExpiryWarningLevel::Normal
        }
    }

    /// 校验许可证类型
    fn validate_permit_type(permit_type: &str) -> Result<(), AppError> {
        match permit_type {
            "wastewater" | "exhaust" | "solid_waste" => Ok(()),
            _ => Err(AppError::bad_request(format!(
                "无效的许可证类型: {}（应为 wastewater/exhaust/solid_waste）",
                permit_type
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_expiry_level_normal() {
        assert_eq!(
            PollutionPermitService::classify_expiry_level(120),
            ExpiryWarningLevel::Normal
        );
        assert_eq!(
            PollutionPermitService::classify_expiry_level(91),
            ExpiryWarningLevel::Normal
        );
    }

    #[test]
    fn test_classify_expiry_level_90_days() {
        assert_eq!(
            PollutionPermitService::classify_expiry_level(90),
            ExpiryWarningLevel::Warning90Days
        );
        assert_eq!(
            PollutionPermitService::classify_expiry_level(61),
            ExpiryWarningLevel::Warning90Days
        );
    }

    #[test]
    fn test_classify_expiry_level_60_days() {
        assert_eq!(
            PollutionPermitService::classify_expiry_level(60),
            ExpiryWarningLevel::Warning60Days
        );
        assert_eq!(
            PollutionPermitService::classify_expiry_level(31),
            ExpiryWarningLevel::Warning60Days
        );
    }

    #[test]
    fn test_classify_expiry_level_30_days() {
        assert_eq!(
            PollutionPermitService::classify_expiry_level(30),
            ExpiryWarningLevel::Warning30Days
        );
        assert_eq!(
            PollutionPermitService::classify_expiry_level(1),
            ExpiryWarningLevel::Warning30Days
        );
        assert_eq!(
            PollutionPermitService::classify_expiry_level(0),
            ExpiryWarningLevel::Warning30Days
        );
    }

    #[test]
    fn test_classify_expiry_level_expired() {
        assert_eq!(
            PollutionPermitService::classify_expiry_level(-1),
            ExpiryWarningLevel::Expired
        );
        assert_eq!(
            PollutionPermitService::classify_expiry_level(-30),
            ExpiryWarningLevel::Expired
        );
    }

    #[test]
    fn test_validate_permit_type_valid() {
        assert!(PollutionPermitService::validate_permit_type("wastewater").is_ok());
        assert!(PollutionPermitService::validate_permit_type("exhaust").is_ok());
        assert!(PollutionPermitService::validate_permit_type("solid_waste").is_ok());
    }

    #[test]
    fn test_validate_permit_type_invalid() {
        assert!(PollutionPermitService::validate_permit_type("invalid").is_err());
    }
}
