use crate::models::fixed_asset;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    Set, TransactionTrait, QuerySelect, PaginatorTrait, Order,
};
use std::sync::Arc;
use rust_decimal::Decimal;
use chrono::NaiveDate;
use crate::utils::error::AppError;
use tracing::{info, error};

/// 固定资产查询参数
#[derive(Debug, Clone, Default)]
pub struct AssetQueryParams {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub asset_category: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

/// 创建资产请求
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CreateAssetRequest {
    pub asset_no: String,
    pub asset_name: String,
    pub asset_category: Option<String>,
    pub specification: Option<String>,
    pub location: Option<String>,
    pub original_value: Decimal,
    pub useful_life: i32,
    pub depreciation_method: Option<String>,
    pub purchase_date: NaiveDate,
    pub put_in_date: NaiveDate,
    pub supplier_id: Option<i32>,
    pub remark: Option<String>,
}

/// 资产处置请求
#[derive(Debug, Clone)]
pub struct DisposalRequest {
    pub disposal_type: String,
    pub disposal_value: Decimal,
    pub disposal_date: NaiveDate,
    pub reason: String,
    pub buyer_info: Option<String>,
}

pub struct FixedAssetService {
    db: Arc<DatabaseConnection>,
}

impl FixedAssetService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建固定资产
    pub async fn create(
        &self,
        req: CreateAssetRequest,
        user_id: i32,
    ) -> Result<fixed_asset::Model, AppError> {
        info!("用户 {} 正在创建固定资产：{}", user_id, req.asset_no);

        let active_asset = fixed_asset::ActiveModel {
            asset_no: Set(req.asset_no),
            asset_name: Set(req.asset_name),
            asset_category: Set(req.asset_category),
            specification: Set(req.specification),
            use_location: Set(req.location),
            original_value: Set(req.original_value),
            net_value: Set(Some(req.original_value)),
            useful_life: Set(Some(req.useful_life)),
            depreciation_method: Set(req.depreciation_method),
            purchase_date: Set(Some(req.purchase_date)),
            in_service_date: Set(Some(req.put_in_date)),
            supplier_id: Set(req.supplier_id),
            status: Set("active".to_string()),
            created_by: Set(user_id),
            ..Default::default()
        };

        let asset = active_asset.insert(&*self.db).await?;
        info!("固定资产创建成功：{}", asset.asset_no);
        Ok(asset)
    }

    /// 获取资产列表（分页）
    pub async fn get_list(
        &self,
        params: AssetQueryParams,
    ) -> Result<(Vec<fixed_asset::Model>, u64), AppError> {
        let mut query = fixed_asset::Entity::find();

        // 关键词筛选
        if let Some(keyword) = &params.keyword {
            let keyword_pattern = format!("%{}%", keyword);
            query = query.filter(
                fixed_asset::Column::AssetNo.like(&keyword_pattern)
                    .or(fixed_asset::Column::AssetName.like(&keyword_pattern)),
            );
        }

        // 状态筛选
        if let Some(status) = &params.status {
            query = query.filter(fixed_asset::Column::Status.eq(status));
        }

        // 资产类别筛选
        if let Some(category) = &params.asset_category {
            query = query.filter(fixed_asset::Column::AssetCategory.eq(category));
        }

        // 获取总数
        let total = query.clone().count(&*self.db).await?;

        // 分页和排序
        let assets = query
            .order_by(fixed_asset::Column::Id, Order::Desc)
            .offset((params.page * params.page_size) as u64)
            .limit(params.page_size as u64)
            .all(&*self.db)
            .await?;

        Ok((assets, total))
    }

    /// 获取资产详情
    pub async fn get_by_id(&self, id: i32) -> Result<fixed_asset::Model, AppError> {
        let asset = fixed_asset::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("固定资产不存在：{}", id)))?;
        Ok(asset)
    }

    /// 计算月折旧额
    pub async fn calculate_monthly_depreciation(
        &self,
        asset_id: i32,
    ) -> Result<Decimal, AppError> {
        let asset = self.get_by_id(asset_id).await?;

        let monthly_depreciation = match asset.depreciation_method.as_deref() {
            Some("straight_line") | None => {
                // 平均年限法
                let useful_life_months = asset.useful_life.unwrap_or(0) as u32 * 12;
                if useful_life_months > 0 {
                    asset.original_value / Decimal::from(useful_life_months)
                } else {
                    Decimal::ZERO
                }
            }
            Some(method) => {
                error!("不支持的折旧方法：{}", method);
                return Err(AppError::ValidationError(format!(
                    "不支持的折旧方法：{}",
                    method
                )));
            }
        };

        Ok(monthly_depreciation)
    }

    /// 计提折旧
    pub async fn depreciate(&self, asset_id: i32, period: &str, user_id: i32) -> Result<(), AppError> {
        info!("用户 {} 正在计提资产 {} 的 {} 折旧", user_id, asset_id, period);

        // 获取资产
        let asset = self.get_by_id(asset_id).await?;

        // 检查资产状态
        if asset.status != "active" {
            return Err(AppError::ValidationError("只有活跃状态的资产才能计提折旧".to_string()));
        }

        // 计算月折旧额
        let monthly_depreciation = self.calculate_monthly_depreciation(asset_id).await?;

        if monthly_depreciation <= Decimal::ZERO {
            return Err(AppError::ValidationError("月折旧额不能为零".to_string()));
        }

        // 开启事务
        let txn = (&*self.db).begin().await?;

        // 保留需要使用的字段值，避免 moved value 错误
        let accumulated_depreciation = asset.accumulated_depreciation;
        let original_value = asset.original_value;

        // 更新资产累计折旧
        let mut asset_active: crate::models::fixed_asset::ActiveModel = asset.into();
        asset_active.accumulated_depreciation = Set(accumulated_depreciation + monthly_depreciation);
        asset_active.net_value = Set(Some(original_value - accumulated_depreciation - monthly_depreciation));
        asset_active.save(&txn).await?;

        // 提交事务
        txn.commit().await?;

        info!("资产 {} 折旧计提成功，月折旧额：{}", asset_id, monthly_depreciation);
        Ok(())
    }

    /// 资产处置
    pub async fn dispose(
        &self,
        asset_id: i32,
        req: DisposalRequest,
        user_id: i32,
    ) -> Result<(), AppError> {
        info!("用户 {} 正在处置资产 {}", user_id, asset_id);

        let asset = self.get_by_id(asset_id).await?;

        // 检查资产状态
        if asset.status != "active" {
            return Err(AppError::ValidationError("只有活跃状态的资产才能处置".to_string()));
        }

        // 开启事务
        let txn = (&*self.db).begin().await?;

        // 生成处置单号
        let disposal_no = format!(
            "D{}{}",
            chrono::Local::now().format("%Y%m%d"),
            asset_id
        );

        // 计算处置损益
        let net_book_value = asset.net_value.unwrap_or(Decimal::ZERO);
        let _disposal_gain_loss = req.disposal_value - net_book_value;

        // 创建处置记录
        let disposal = crate::models::fixed_asset_disposal::ActiveModel {
            id: Set(0),
            disposal_no: Set(disposal_no),
            asset_id: Set(asset_id),
            disposal_type: Set(req.disposal_type),
            disposal_date: Set(req.disposal_date),
            disposal_amount: Set(req.disposal_value), // 使用 disposal_amount
            disposal_reason: Set(req.reason), // 使用 disposal_reason
            quantity: Set(1), // 处置数量默认为1
            status: Set("COMPLETED".to_string()),
            remarks: Set(req.buyer_info), // 使用 remarks 存储买家信息
            created_by: Set(user_id),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        };

        disposal.insert(&txn).await?;

        // 更新资产状态
        let mut asset_active: crate::models::fixed_asset::ActiveModel = asset.into();
        asset_active.status = Set("disposed".to_string());
        asset_active.save(&txn).await?;

        // 提交事务
        txn.commit().await?;

        info!("资产 {} 处置成功，处置价值：{}", asset_id, req.disposal_value);
        Ok(())
    }

    /// 删除资产（仅支持未使用状态）
    pub async fn delete(&self, asset_id: i32, user_id: i32) -> Result<(), AppError> {
        info!("用户 {} 正在删除资产 {}", user_id, asset_id);

        let asset = self.get_by_id(asset_id).await?;

        if asset.status != "inactive" {
            return Err(AppError::ValidationError("只能删除未使用状态的资产".to_string()));
        }

        fixed_asset::Entity::delete_many()
            .filter(fixed_asset::Column::Id.eq(asset_id))
            .exec(&*self.db)
            .await?;

        info!("资产 {} 删除成功", asset_id);
        Ok(())
    }
}
