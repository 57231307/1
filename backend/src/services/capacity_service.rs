//! 产能分析 Service
//!
//! 提供产能负荷计算、产能瓶颈识别、工作中心管理等功能

use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::models::production_order::{
    Column as ProductionOrderColumn, Entity as ProductionOrderEntity,
};
use crate::models::work_center::{
    ActiveModel as WorkCenterActiveModel, Column as WorkCenterColumn, Entity as WorkCenterEntity,
};
use crate::utils::error::AppError;

/// 可用产能查询结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableCapacity {
    pub work_center_id: i32,
    pub work_center_code: String,
    pub work_center_name: String,
    pub daily_capacity: Decimal,
    pub capacity_unit: Option<String>,
    pub date_from: NaiveDate,
    pub date_to: NaiveDate,
    pub total_capacity: Decimal,
    pub used_capacity: Decimal,
    pub available_capacity: Decimal,
    pub load_rate: Decimal,
}

/// 产能负荷预警事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityOverloadAlert {
    pub work_center_id: i32,
    pub work_center_code: String,
    pub work_center_name: String,
    pub daily_capacity: Decimal,
    pub current_demand: Decimal,
    pub load_rate: Decimal,
    pub threshold: Decimal,
    pub alert_level: String,
    pub message: String,
}

/// 工作中心产能信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkCenterCapacity {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub work_center_type: Option<String>,
    pub daily_capacity: Decimal,
    pub capacity_unit: Option<String>,
    pub status: String,
    pub shifts: Vec<ShiftInfo>,
}

/// 班次信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShiftInfo {
    pub shift_name: String,
    pub start_time: String,
    pub end_time: String,
    pub capacity_ratio: Decimal,
}

/// 产能负荷分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityLoadItem {
    pub work_center_id: i32,
    pub work_center_code: String,
    pub work_center_name: String,
    pub daily_capacity: Decimal,
    pub capacity_unit: Option<String>,
    pub planned_quantity: Decimal,
    pub in_progress_quantity: Decimal,
    pub total_demand: Decimal,
    pub load_rate: Decimal,
    pub status: String,
}

/// 产能概览
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityOverview {
    pub total_work_centers: i64,
    pub active_work_centers: i64,
    pub total_daily_capacity: Decimal,
    pub total_planned_demand: Decimal,
    pub overall_load_rate: Decimal,
    pub bottleneck_work_centers: Vec<CapacityLoadItem>,
    pub overloaded_count: i64,
    pub idle_count: i64,
}

/// 产能负荷查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct LoadAnalysisQuery {
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
    #[allow(dead_code)] // TODO(tech-debt): 报表模块接入后移除
    pub work_center_id: Option<i32>,
}

/// 产能分析 Service
pub struct CapacityService {
    db: Arc<DatabaseConnection>,
}

impl CapacityService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取所有工作中心及其产能信息
    pub async fn list_work_centers(&self) -> Result<Vec<WorkCenterCapacity>, AppError> {
        let work_centers = WorkCenterEntity::find()
            .filter(WorkCenterColumn::Status.ne("INACTIVE"))
            .order_by_asc(WorkCenterColumn::Code)
            .all(&*self.db)
            .await?;

        let result = work_centers
            .into_iter()
            .map(|wc| {
                let shifts = Self::default_shifts_for_type(&wc.work_center_type);
                WorkCenterCapacity {
                    id: wc.id,
                    code: wc.code,
                    name: wc.name,
                    work_center_type: wc.work_center_type,
                    daily_capacity: wc.daily_capacity.unwrap_or(Decimal::ZERO),
                    capacity_unit: wc.capacity_unit,
                    status: wc.status,
                    shifts,
                }
            })
            .collect();

        Ok(result)
    }

    /// 获取单个工作中心产能详情
    pub async fn get_work_center(&self, id: i32) -> Result<WorkCenterCapacity, AppError> {
        let wc = WorkCenterEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("工作中心 ID {} 不存在", id)))?;

        let shifts = Self::default_shifts_for_type(&wc.work_center_type);
        Ok(WorkCenterCapacity {
            id: wc.id,
            code: wc.code,
            name: wc.name,
            work_center_type: wc.work_center_type,
            daily_capacity: wc.daily_capacity.unwrap_or(Decimal::ZERO),
            capacity_unit: wc.capacity_unit,
            status: wc.status,
            shifts,
        })
    }

    /// 产能负荷分析
    pub async fn load_analysis(
        &self,
        query: LoadAnalysisQuery,
    ) -> Result<Vec<CapacityLoadItem>, AppError> {
        let work_centers = WorkCenterEntity::find()
            .filter(WorkCenterColumn::Status.eq("ACTIVE"))
            .all(&*self.db)
            .await?;

        // v15 批次 42 修复：批量查询所有工作中心的订单，避免循环内逐个查询（N+1）
        let wc_ids: Vec<i32> = work_centers.iter().map(|wc| wc.id).collect();
        let all_orders = if wc_ids.is_empty() {
            Vec::new()
        } else {
            let mut order_query = ProductionOrderEntity::find()
                .filter(ProductionOrderColumn::WorkCenterId.is_in(wc_ids))
                .filter(ProductionOrderColumn::Status.is_in(vec!["SCHEDULED", "IN_PROGRESS"]));
            if let Some(from) = query.date_from {
                order_query = order_query.filter(ProductionOrderColumn::PlannedEndDate.gte(from));
            }
            if let Some(to) = query.date_to {
                order_query = order_query.filter(ProductionOrderColumn::PlannedStartDate.lte(to));
            }
            order_query.all(&*self.db).await?
        };
        let orders_by_wc: std::collections::HashMap<i32, Vec<crate::models::production_order::Model>> =
            all_orders.into_iter().fold(
                std::collections::HashMap::new(),
                |mut acc, o| {
                    acc.entry(o.work_center_id).or_default().push(o);
                    acc
                },
            );

        let mut results = Vec::new();

        for wc in work_centers {
            let daily_capacity = wc.daily_capacity.unwrap_or(Decimal::ZERO);

            // 从批量查询结果获取该工作中心的订单
            let orders = orders_by_wc.get(&wc.id).cloned().unwrap_or_default();

            let mut planned_quantity = Decimal::ZERO;
            let mut in_progress_quantity = Decimal::ZERO;

            for order in &orders {
                match order.status.as_str() {
                    "SCHEDULED" => planned_quantity += order.planned_quantity,
                    "IN_PROGRESS" => in_progress_quantity += order.planned_quantity,
                    _ => {}
                }
            }

            let total_demand = planned_quantity + in_progress_quantity;
            let load_rate = if daily_capacity > Decimal::ZERO {
                (total_demand / daily_capacity * Decimal::from(100))
                    .round_dp_with_strategy(2, rust_decimal::RoundingStrategy::MidpointAwayFromZero)
            } else {
                Decimal::ZERO
            };

            let status = if load_rate > Decimal::from(100) {
                "OVERLOADED".to_string()
            } else if load_rate > Decimal::from(80) {
                "HIGH".to_string()
            } else if load_rate > Decimal::from(20) {
                "NORMAL".to_string()
            } else {
                "IDLE".to_string()
            };

            results.push(CapacityLoadItem {
                work_center_id: wc.id,
                work_center_code: wc.code,
                work_center_name: wc.name,
                daily_capacity,
                capacity_unit: wc.capacity_unit,
                planned_quantity,
                in_progress_quantity,
                total_demand,
                load_rate,
                status,
            });
        }

        // 按负荷率降序排列
        results.sort_by_key(|b| std::cmp::Reverse(b.load_rate));
        Ok(results)
    }

    /// 产能概览
    pub async fn overview(&self) -> Result<CapacityOverview, AppError> {
        let load_items = self
            .load_analysis(LoadAnalysisQuery {
                date_from: None,
                date_to: None,
                work_center_id: None,
            })
            .await?;

        let total_work_centers = load_items.len() as i64;
        let active_work_centers = load_items.iter().filter(|i| i.status != "IDLE").count() as i64;

        let total_daily_capacity: Decimal = load_items.iter().map(|i| i.daily_capacity).sum();
        let total_planned_demand: Decimal = load_items.iter().map(|i| i.total_demand).sum();

        let overall_load_rate = if total_daily_capacity > Decimal::ZERO {
            (total_planned_demand / total_daily_capacity * Decimal::from(100))
                .round_dp_with_strategy(2, rust_decimal::RoundingStrategy::MidpointAwayFromZero)
        } else {
            Decimal::ZERO
        };

        // 识别瓶颈工作中心（负荷率 > 80%）
        let bottleneck_work_centers: Vec<CapacityLoadItem> = load_items
            .iter()
            .filter(|i| i.load_rate > Decimal::from(80))
            .cloned()
            .collect();

        let overloaded_count = load_items
            .iter()
            .filter(|i| i.status == "OVERLOADED")
            .count() as i64;

        let idle_count = load_items.iter().filter(|i| i.status == "IDLE").count() as i64;

        Ok(CapacityOverview {
            total_work_centers,
            active_work_centers,
            total_daily_capacity,
            total_planned_demand,
            overall_load_rate,
            bottleneck_work_centers,
            overloaded_count,
            idle_count,
        })
    }

    /// 根据工作中心类型返回默认班次配置
    fn default_shifts_for_type(wc_type: &Option<String>) -> Vec<ShiftInfo> {
        match wc_type.as_deref().unwrap_or("STANDARD") {
            "CONTINUOUS" => vec![
                ShiftInfo {
                    shift_name: "早班".to_string(),
                    start_time: "08:00".to_string(),
                    end_time: "20:00".to_string(),
                    capacity_ratio: Decimal::from(50),
                },
                ShiftInfo {
                    shift_name: "晚班".to_string(),
                    start_time: "20:00".to_string(),
                    end_time: "08:00".to_string(),
                    capacity_ratio: Decimal::from(50),
                },
            ],
            _ => vec![ShiftInfo {
                shift_name: "白班".to_string(),
                start_time: "08:00".to_string(),
                end_time: "17:00".to_string(),
                capacity_ratio: Decimal::from(100),
            }],
        }
    }

    /// 创建工作中心
    pub async fn create_work_center(
        &self,
        input: CreateWorkCenterInput,
    ) -> Result<WorkCenterCapacity, AppError> {
        let now = Utc::now();
        // 自动生成代码
        let code = input.code.unwrap_or_else(|| {
            let timestamp = now.format("%Y%m%d%H%M%S");
            let random = crate::utils::random::random_4_digit();
            format!("WC-{}-{:04}", timestamp, random)
        });

        let active_model = WorkCenterActiveModel {
            code: Set(code),
            name: Set(input.name),
            work_center_type: Set(input.work_center_type),
            daily_capacity: Set(Some(
                input
                    .daily_capacity
                    .unwrap_or(rust_decimal::Decimal::new(100, 0)),
            )),
            capacity_unit: Set(input.capacity_unit),
            status: Set(input.status.unwrap_or_else(|| "ACTIVE".to_string())),
            remarks: Set(input.remarks),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };

        let model = active_model.insert(&*self.db).await?;

        self.get_work_center(model.id).await
    }

    /// 更新工作中心
    pub async fn update_work_center(
        &self,
        id: i32,
        input: UpdateWorkCenterInput,
    ) -> Result<WorkCenterCapacity, AppError> {
        let existing = WorkCenterEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("工作中心 ID {} 不存在", id)))?;

        let mut active_model: WorkCenterActiveModel = existing.into();

        if let Some(code) = input.code {
            active_model.code = Set(code);
        }
        if let Some(name) = input.name {
            active_model.name = Set(name);
        }
        if let Some(wc_type) = input.work_center_type {
            active_model.work_center_type = Set(Some(wc_type));
        }
        if let Some(capacity) = input.daily_capacity {
            active_model.daily_capacity = Set(Some(capacity));
        }
        if let Some(unit) = input.capacity_unit {
            active_model.capacity_unit = Set(Some(unit));
        }
        if let Some(status) = input.status {
            active_model.status = Set(status);
        }
        if let Some(remarks) = input.remarks {
            active_model.remarks = Set(Some(remarks));
        }
        active_model.updated_at = Set(Utc::now());

        let model = active_model.update(&*self.db).await?;

        self.get_work_center(model.id).await
    }

    /// 删除工作中心（软删除）
    pub async fn delete_work_center(&self, id: i32) -> Result<(), AppError> {
        let existing = WorkCenterEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("工作中心 ID {} 不存在", id)))?;

        let mut active_model: WorkCenterActiveModel = existing.into();
        active_model.status = Set("INACTIVE".to_string());
        active_model.updated_at = Set(Utc::now());

        active_model.update(&*self.db).await?;

        Ok(())
    }

    /// 产能预测（基于历史数据）
    pub async fn forecast_capacity(
        &self,
        work_center_id: i32,
        days: i32,
    ) -> Result<CapacityForecast, AppError> {
        let wc = WorkCenterEntity::find_by_id(work_center_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("工作中心 ID {} 不存在", work_center_id)))?;

        let daily_capacity = wc.daily_capacity.unwrap_or(Decimal::ZERO);

        // 简单预测：基于当前产能和历史负荷率
        let load_items = self
            .load_analysis(LoadAnalysisQuery {
                date_from: None,
                date_to: None,
                work_center_id: Some(work_center_id),
            })
            .await?;

        let current_load = load_items
            .first()
            .map(|i| i.load_rate)
            .unwrap_or(Decimal::ZERO);

        // 预测未来产能
        let forecasted_capacity = daily_capacity * Decimal::from(days);
        let predicted_demand = forecasted_capacity * (current_load / Decimal::from(100));
        let predicted_available = forecasted_capacity - predicted_demand;

        Ok(CapacityForecast {
            work_center_id,
            work_center_name: wc.name,
            daily_capacity,
            forecast_days: days,
            total_capacity: forecasted_capacity,
            predicted_demand,
            predicted_available,
            predicted_load_rate: current_load,
            confidence: 0.8, // 简化的置信度
        })
    }

    /// 获取指定时间段内的可用产能
    pub async fn get_available_capacity(
        &self,
        work_center_id: i32,
        date_from: NaiveDate,
        date_to: NaiveDate,
    ) -> Result<AvailableCapacity, AppError> {
        let wc = WorkCenterEntity::find_by_id(work_center_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("工作中心 ID {} 不存在", work_center_id)))?;

        let daily_capacity = wc.daily_capacity.unwrap_or(Decimal::ZERO);
        let days = (date_to - date_from).num_days() + 1;
        let total_capacity = daily_capacity * Decimal::from(days);

        // 查询该时间段内已占用的产能
        let orders = ProductionOrderEntity::find()
            .filter(ProductionOrderColumn::WorkCenterId.eq(work_center_id))
            .filter(ProductionOrderColumn::Status.is_in(vec!["SCHEDULED", "IN_PROGRESS"]))
            .filter(ProductionOrderColumn::PlannedEndDate.gte(date_from))
            .filter(ProductionOrderColumn::PlannedStartDate.lte(date_to))
            .all(&*self.db)
            .await?;

        let used_capacity: Decimal = orders.iter().map(|o| o.planned_quantity).sum();
        let available_capacity = total_capacity - used_capacity;
        let load_rate = if total_capacity > Decimal::ZERO {
            (used_capacity / total_capacity * Decimal::from(100))
                .round_dp_with_strategy(2, rust_decimal::RoundingStrategy::MidpointAwayFromZero)
        } else {
            Decimal::ZERO
        };

        Ok(AvailableCapacity {
            work_center_id,
            work_center_code: wc.code,
            work_center_name: wc.name,
            daily_capacity,
            capacity_unit: wc.capacity_unit,
            date_from,
            date_to,
            total_capacity,
            used_capacity,
            available_capacity,
            load_rate,
        })
    }

    /// 检查产能负荷是否超阈值
    pub async fn check_capacity_overload(
        &self,
        threshold: Decimal,
    ) -> Result<Vec<CapacityOverloadAlert>, AppError> {
        let load_items = self
            .load_analysis(LoadAnalysisQuery {
                date_from: None,
                date_to: None,
                work_center_id: None,
            })
            .await?;

        let mut alerts = Vec::new();

        for item in load_items {
            if item.load_rate >= threshold {
                let alert_level = if item.load_rate >= Decimal::from(100) {
                    "CRITICAL".to_string()
                } else if item.load_rate >= Decimal::from(90) {
                    "HIGH".to_string()
                } else {
                    "MEDIUM".to_string()
                };

                alerts.push(CapacityOverloadAlert {
                    work_center_id: item.work_center_id,
                    work_center_code: item.work_center_code,
                    work_center_name: item.work_center_name.clone(),
                    daily_capacity: item.daily_capacity,
                    current_demand: item.total_demand,
                    load_rate: item.load_rate,
                    threshold,
                    alert_level,
                    message: format!(
                        "工作中心 {} 产能负荷已达 {}%，超过阈值 {}%",
                        item.work_center_name, item.load_rate, threshold
                    ),
                });
            }
        }

        Ok(alerts)
    }
}

/// 产能预测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityForecast {
    pub work_center_id: i32,
    pub work_center_name: String,
    pub daily_capacity: Decimal,
    pub forecast_days: i32,
    pub total_capacity: Decimal,
    pub predicted_demand: Decimal,
    pub predicted_available: Decimal,
    pub predicted_load_rate: Decimal,
    pub confidence: f64,
}

/// 创建工作中心输入
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkCenterInput {
    pub code: Option<String>,
    pub name: String,
    pub work_center_type: Option<String>,
    pub daily_capacity: Option<Decimal>,
    pub capacity_unit: Option<String>,
    pub status: Option<String>,
    pub remarks: Option<String>,
}

/// 更新工作中心输入
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWorkCenterInput {
    pub code: Option<String>,
    pub name: Option<String>,
    pub work_center_type: Option<String>,
    pub daily_capacity: Option<Decimal>,
    pub capacity_unit: Option<String>,
    pub status: Option<String>,
    pub remarks: Option<String>,
}
