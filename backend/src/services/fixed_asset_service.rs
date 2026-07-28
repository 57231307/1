use crate::models::fixed_asset;
// V15 P1 17.8-D4：资产盘点模型
use crate::models::{fixed_asset_count, fixed_asset_count_item};
// 批次 208 P2-5 修复（v12 复审）：硬编码 "active"/"inactive" 替换为 master_data 常量
use crate::models::status::master_data;
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;
use crate::utils::sql_escape::safe_like_pattern;
use chrono::NaiveDate;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info};

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
pub struct CreateAssetRequest {
    pub asset_no: Option<String>,
    pub asset_name: Option<String>,
    pub asset_category: Option<String>,
    pub specification: Option<String>,
    pub location: Option<String>,
    pub original_value: Option<Decimal>,
    pub useful_life: Option<i32>,
    pub depreciation_method: Option<String>,
    pub purchase_date: Option<NaiveDate>,
    pub put_in_date: Option<NaiveDate>,
    pub supplier_id: Option<i32>,
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

/// 折旧记录插入参数（减少 helper 函数参数数量）
struct DepreciationRecordParams {
    asset_id: i32,
    period: String,
    actual_depreciation: Decimal,
    accumulated_depreciation: Decimal,
    new_accumulated: Decimal,
    net_value_before: Decimal,
    new_net_value: Decimal,
    depreciation_method: String,
    user_id: i32,
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
        // 自动生成资产编号
        let asset_no = req.asset_no.unwrap_or_else(|| {
            let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
            let random = crate::utils::random::random_4_digit();
            format!("FA-{}-{:04}", timestamp, random)
        });

        info!("用户 {} 正在创建固定资产：{}", user_id, asset_no);

        let original_value = req.original_value.unwrap_or_default();

        let active_asset = fixed_asset::ActiveModel {
            asset_no: Set(asset_no),
            asset_name: Set(req
                .asset_name
                .unwrap_or_else(|| format!("资产_{}", chrono::Utc::now().timestamp()))),
            asset_category: Set(req.asset_category),
            specification: Set(req.specification),
            use_location: Set(req.location),
            original_value: Set(original_value),
            net_value: Set(Some(original_value)),
            useful_life: Set(Some(req.useful_life.unwrap_or(5))),
            depreciation_method: Set(req.depreciation_method),
            purchase_date: Set(Some(
                req.purchase_date
                    .unwrap_or_else(|| chrono::Utc::now().date_naive()),
            )),
            in_service_date: Set(Some(
                req.put_in_date
                    .unwrap_or_else(|| chrono::Utc::now().date_naive()),
            )),
            supplier_id: Set(req.supplier_id),
            status: Set(master_data::ACTIVE.to_string()),
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
            let keyword_pattern = safe_like_pattern(keyword);
            query = query.filter(
                fixed_asset::Column::AssetNo
                    .like(&keyword_pattern)
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

        // 批次 266：接入 paginate_with_total，消除手写 count + offset/limit 重复
        // 补 page_size.clamp(1, 100) 防 DoS（原实现仅 clamp page，page_size 无上限保护）
        let paginator = query
            .order_by(fixed_asset::Column::Id, Order::Desc)
            .paginate(&*self.db, params.page_size.clamp(1, 100) as u64);
        let (assets, total) =
            paginate_with_total(paginator, params.page.clamp(1, 1000) as u64).await?;

        Ok((assets, total))
    }

    /// 获取资产详情
    pub async fn get_by_id(&self, id: i32) -> Result<fixed_asset::Model, AppError> {
        let asset = fixed_asset::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("固定资产不存在：{}", id)))?;
        Ok(asset)
    }

    /// 计算月折旧额（纯计算函数，无 IO）
    /// 批次 92 P3-10 修复：从 `calculate_monthly_depreciation` 拆出纯计算部分，；供 `depreciate` 事务内复用已 lock_exclusive 读出的 asset，；消除事务外重复 `self.get_by_id(asset_id)` 读取（原实现存在 TOCTOU 风险：事务外读到的 asset 可能已被并发 depreciate/dispose 修改）。；批次 118 P2-8 修复：删除从未接入业务的 `calculate_monthly_depreciation` 异步包装；（depreciate 已直接调用此纯计算函数，预留的折旧预览 API 端点从未实现）。；V15 P1 17.8-D1 扩展：新增 3 种折旧方法；`straight_line`：平均年限法（原有，(原值 - 残值) / (使用年限 × 12)）；`units_of_production`：工作量法（基于月工作量，使用 monthly_depreciation 字段预存）；`sum_of_years_digits`：年数总和法（(原值 - 残值) × 剩余年数 / 年数总和 / 12）；`double_declining_balance`：双倍余额递减法（净值 × 2 / 使用年限 / 12）
    fn calc_monthly_depreciation_for(asset: &fixed_asset::Model) -> Result<Decimal, AppError> {
        let residual_value = asset.salvage_value.unwrap_or(Decimal::ZERO);
        let useful_life_years = asset.useful_life.unwrap_or_default();

        let monthly_depreciation = match asset.depreciation_method.as_deref() {
            Some("straight_line") | None => {
                // 平均年限法：(原值 - 残值) / (使用年限 * 12)
                let useful_life_months = useful_life_years as u32 * 12;
                if useful_life_months > 0 {
                    ((asset.original_value - residual_value) / Decimal::from(useful_life_months))
                        .round_dp(2)
                } else {
                    Decimal::ZERO
                }
            }
            Some("units_of_production") => {
                // V15 P1 17.8-D1：工作量法
                // 月折旧额基于实际工作量计算，由前端按月录入到 asset.monthly_depreciation
                // 字段（视为本月实际工作量对应的折旧额）
                // 若未设置则按 0 处理（待月末录入工作量后重算）
                asset.monthly_depreciation.unwrap_or(Decimal::ZERO)
            }
            Some("sum_of_years_digits") => {
                // V15 P1 17.8-D1：年数总和法
                // 年折旧率 = 剩余使用年数 / 年数总和
                // 月折旧额 = (原值 - 残值) × 年折旧率 / 12
                if useful_life_years <= 0 {
                    Decimal::ZERO
                } else {
                    // 估算剩余年数：使用年限 - 已使用年数（基于累计折旧占比近似）
                    let depreciable_amount = asset.original_value - residual_value;
                    if depreciable_amount <= Decimal::ZERO {
                        Decimal::ZERO
                    } else {
                        // 已折旧比例
                        let depreciated_ratio = if depreciable_amount.is_zero() {
                            Decimal::ZERO
                        } else {
                            asset.accumulated_depreciation / depreciable_amount
                        };
                        // 已使用年数（近似）
                        let used_years = (depreciated_ratio * Decimal::from(useful_life_years))
                            .to_usize()
                            .unwrap_or(0);
                        let remaining_years =
                            useful_life_years.saturating_sub(used_years as i32).max(1);
                        // 年数总和 = n + (n-1) + ... + 1 = n * (n+1) / 2
                        let sum_of_years = useful_life_years * (useful_life_years + 1) / 2;
                        if sum_of_years <= 0 {
                            Decimal::ZERO
                        } else {
                            ((depreciable_amount * Decimal::from(remaining_years)
                                / Decimal::from(sum_of_years))
                                / Decimal::from(12))
                            .round_dp(2)
                        }
                    }
                }
            }
            Some("double_declining_balance") => {
                // V15 P1 17.8-D1：双倍余额递减法
                // 年折旧率 = 2 / 使用年限
                // 月折旧额 = 净值 × 年折旧率 / 12
                // 最后两年改为直线法（此处简化为：净值接近残值时返回 0）
                if useful_life_years <= 0 {
                    Decimal::ZERO
                } else {
                    let net_value = asset.net_value.unwrap_or(asset.original_value);
                    // 净值接近残值时停止折旧
                    if net_value <= residual_value {
                        Decimal::ZERO
                    } else {
                        let annual_rate = Decimal::from(2) / Decimal::from(useful_life_years);
                        ((net_value * annual_rate) / Decimal::from(12)).round_dp(2)
                    }
                }
            }
            Some(method) => {
                error!("不支持的折旧方法：{}", method);
                return Err(AppError::validation(format!(
                    "不支持的折旧方法：{}",
                    method
                )));
            }
        };

        Ok(monthly_depreciation)
    }

    /// 查询资产并校验状态（加 lock_exclusive 串行化并发）
    async fn validate_asset_for_depreciation(
        txn: &sea_orm::DatabaseTransaction,
        asset_id: i32,
    ) -> Result<fixed_asset::Model, AppError> {
        let asset = fixed_asset::Entity::find_by_id(asset_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("固定资产不存在：{}", asset_id)))?;

        if asset.status != master_data::ACTIVE {
            return Err(AppError::validation(
                "只有活跃状态的资产才能计提折旧".to_string(),
            ));
        }

        Ok(asset)
    }

    /// 计算折旧值（封顶到可折旧上限），返回 (actual_depreciation, new_accumulated, new_net_value, depreciable_cap)
    fn compute_depreciation_values(
        asset: &fixed_asset::Model,
        monthly_depreciation: Decimal,
    ) -> (Decimal, Decimal, Decimal, Decimal) {
        let accumulated_depreciation = asset.accumulated_depreciation;
        let original_value = asset.original_value;
        let residual_value = asset.salvage_value.unwrap_or(Decimal::ZERO);

        // 可折旧上限 = original_value - residual_value
        let depreciable_cap = original_value - residual_value;

        // 封顶到 depreciable_cap，防止最后一期溢出残值
        let raw_new_accumulated = accumulated_depreciation + monthly_depreciation;
        let new_accumulated = raw_new_accumulated.min(depreciable_cap);
        // 实际计提额（封顶后可能小于月折旧额）
        let actual_depreciation = new_accumulated - accumulated_depreciation;
        // 净值 = 原值 - 累计折旧，不能低于残值
        let new_net_value = (original_value - new_accumulated).max(residual_value);

        (
            actual_depreciation,
            new_accumulated,
            new_net_value,
            depreciable_cap,
        )
    }

    /// 插入折旧记录，唯一约束冲突转为业务校验错误
    async fn insert_depreciation_record(
        txn: &sea_orm::DatabaseTransaction,
        params: &DepreciationRecordParams,
    ) -> Result<(), AppError> {
        let depreciation_record = crate::models::fixed_asset_depreciation_record::ActiveModel {
            id: Default::default(),
            asset_id: Set(params.asset_id),
            period: Set(params.period.clone()),
            depreciation_amount: Set(params.actual_depreciation),
            accumulated_before: Set(params.accumulated_depreciation),
            accumulated_after: Set(params.new_accumulated),
            net_value_before: Set(Some(params.net_value_before)),
            net_value_after: Set(Some(params.new_net_value)),
            depreciation_method: Set(Some(params.depreciation_method.clone())),
            created_by: Set(params.user_id),
            created_at: Set(chrono::Utc::now()),
        };
        use sea_orm::ActiveModelTrait;
        if let Err(err) = depreciation_record.insert(txn).await {
            let err_str = err.to_string();
            if err_str.contains("uk_fa_depreciation_records_asset_period") {
                tracing::warn!(
                    "资产 {} 期间 {} 重复计提折旧",
                    params.asset_id,
                    params.period
                );
                return Err(AppError::validation("该资产此期间已计提折旧"));
            }
            return Err(err.into());
        }
        Ok(())
    }

    /// 判断是否跳过折旧：返回 Some(原因) 时跳过
    fn should_skip_depreciation(
        asset: &fixed_asset::Model,
        monthly_depreciation: Decimal,
    ) -> Option<&'static str> {
        // 月折旧额为 0（使用寿命为 0 或已封顶）
        if monthly_depreciation <= Decimal::ZERO {
            return Some("月折旧额为 0，跳过本次计提");
        }
        // 已足额折旧（累计 >= 可折旧上限）
        let residual_value = asset.salvage_value.unwrap_or(Decimal::ZERO);
        let depreciable_cap = asset.original_value - residual_value;
        if asset.accumulated_depreciation >= depreciable_cap {
            return Some("已足额折旧，跳过本次计提");
        }
        None
    }

    /// 更新资产累计折旧和净值
    async fn apply_depreciation_update(
        txn: &sea_orm::DatabaseTransaction,
        asset: fixed_asset::Model,
        new_accumulated: Decimal,
        new_net_value: Decimal,
    ) -> Result<(), AppError> {
        let mut asset_active: crate::models::fixed_asset::ActiveModel = asset.into();
        asset_active.accumulated_depreciation = Set(new_accumulated);
        asset_active.net_value = Set(Some(new_net_value));
        asset_active.save(txn).await?;
        Ok(())
    }

    /// 构建折旧记录参数
    fn build_depreciation_record_params(
        asset_id: i32,
        period: &str,
        actual_depreciation: Decimal,
        accumulated_depreciation: Decimal,
        new_accumulated: Decimal,
        net_value_before: Decimal,
        new_net_value: Decimal,
        depreciation_method: String,
        user_id: i32,
    ) -> DepreciationRecordParams {
        DepreciationRecordParams {
            asset_id,
            period: period.to_string(),
            actual_depreciation,
            accumulated_depreciation,
            new_accumulated,
            net_value_before,
            new_net_value,
            depreciation_method,
            user_id,
        }
    }

    /// 计提折旧
    /// 批次 85 v2 复审 P1-4 修复：状态门移入 txn + lock_exclusive 串行化；原实现状态门在 self.db 查询（get_by_id），txn 在状态门后才开始，存在 TOCTOU；（并发 dispose/depreciate 会基于过期状态通过检查后重复写入）
    pub async fn depreciate(
        &self,
        asset_id: i32,
        period: &str,
        user_id: i32,
    ) -> Result<(), AppError> {
        info!(
            "用户 {} 正在计提资产 {} 的 {} 折旧",
            user_id, asset_id, period
        );

        // 开启事务，状态门 + update 在同一事务内
        let txn = (*self.db).begin().await?;
        let asset = Self::validate_asset_for_depreciation(&txn, asset_id).await?;
        let monthly_depreciation = Self::calc_monthly_depreciation_for(&asset)?;

        // 零值或已足额折旧则跳过（rollback + Ok 返回）
        if let Some(reason) = Self::should_skip_depreciation(&asset, monthly_depreciation) {
            info!("资产 {} {}", asset_id, reason);
            txn.rollback().await?;
            return Ok(());
        }

        // 保留记录所需字段（asset 即将被 apply_depreciation_update 消费）
        let accumulated_depreciation = asset.accumulated_depreciation;
        let net_value_before = asset.net_value.unwrap_or(Decimal::ZERO);
        let depreciation_method = asset.depreciation_method.clone().unwrap_or_default();
        let (actual_depreciation, new_accumulated, new_net_value, _) =
            Self::compute_depreciation_values(&asset, monthly_depreciation);

        // 更新资产累计折旧和净值
        Self::apply_depreciation_update(&txn, asset, new_accumulated, new_net_value).await?;

        // 构建并插入折旧记录（失败时显式回滚）
        let record_params = Self::build_depreciation_record_params(
            asset_id,
            period,
            actual_depreciation,
            accumulated_depreciation,
            new_accumulated,
            net_value_before,
            new_net_value,
            depreciation_method,
            user_id,
        );
        if let Err(e) = Self::insert_depreciation_record(&txn, &record_params).await {
            if let Err(rb_err) = txn.rollback().await {
                tracing::error!(error = %rb_err, "事务回滚失败，可能存在连接异常");
            }
            return Err(e);
        }

        // 提交事务
        txn.commit().await?;
        info!(
            "资产 {} 折旧计提成功，实际计提额：{}（月折旧额：{}，累计：{} -> {}）",
            asset_id,
            actual_depreciation,
            monthly_depreciation,
            accumulated_depreciation,
            new_accumulated
        );
        Ok(())
    }

    /// 资产处置
    /// 批次 85 v2 复审 P1-5 修复：状态门移入 txn + lock_exclusive 串行化；原实现状态门在 self.db 查询（get_by_id），txn 在状态门后才开始，存在 TOCTOU；（并发 dispose/depreciate 会基于过期状态通过检查后重复写入）；V15 P1 17.8-D3 扩展：处置时生成处置损益凭证；借：固定资产清理（资产净值）/ 累计折旧（已计提折旧）；贷：固定资产（原值）；借/贷：银行存款（处置收入）/ 营业外收入（处置收益）或 营业外支出（处置损失）
    pub async fn dispose(
        &self,
        asset_id: i32,
        req: DisposalRequest,
        user_id: i32,
    ) -> Result<(), AppError> {
        info!("用户 {} 正在处置资产 {}", user_id, asset_id);

        // 开启事务，状态门 + update 在同一事务内
        let txn = (*self.db).begin().await?;

        // 加 lock_exclusive 串行化并发状态变更
        let asset = fixed_asset::Entity::find_by_id(asset_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("固定资产不存在：{}", asset_id)))?;

        // 检查资产状态
        if asset.status != master_data::ACTIVE {
            return Err(AppError::validation(
                "只有活跃状态的资产才能处置".to_string(),
            ));
        }

        // 生成处置单号
        let disposal_no = format!("D{}{}", chrono::Local::now().format("%Y%m%d"), asset_id);

        // 计算处置损益
        let net_book_value = asset.net_value.unwrap_or(Decimal::ZERO);
        let accumulated_depreciation = asset.accumulated_depreciation;
        let original_value = asset.original_value;
        // 批次 88 PH-3 占位符实现：计算结果持久化到 fixed_asset_disposals.gain_loss 列
        let disposal_gain_loss = req.disposal_value - net_book_value;

        // 创建处置记录
        // v3 P1-1 修复：id: Set(0) 会覆盖 SERIAL 默认值导致第二次插入主键冲突，改为 Default::default()
        let disposal = crate::models::fixed_asset_disposal::ActiveModel {
            id: Default::default(),
            disposal_no: Set(disposal_no.clone()),
            asset_id: Set(asset_id),
            disposal_type: Set(req.disposal_type.clone()),
            disposal_date: Set(req.disposal_date),
            disposal_amount: Set(req.disposal_value), // 使用 disposal_amount
            gain_loss: Set(Some(disposal_gain_loss)), // 批次 88 PH-3：持久化处置损益
            disposal_reason: Set(req.reason.clone()), // 使用 disposal_reason
            quantity: Set(1),                         // 处置数量默认为1
            status: Set("COMPLETED".to_string()),
            remarks: Set(req.buyer_info.clone()), // 使用 remarks 存储买家信息
            created_by: Set(user_id),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        };

        let inserted_disposal = disposal.insert(&txn).await?;

        // V15 P1 17.8-D3：生成处置损益凭证
        Self::generate_disposal_voucher_txn(
            &txn,
            &inserted_disposal,
            &asset,
            original_value,
            accumulated_depreciation,
            net_book_value,
            req.disposal_value,
            disposal_gain_loss,
            user_id,
        )
        .await?;

        // 更新资产状态
        let mut asset_active: crate::models::fixed_asset::ActiveModel = asset.into();
        asset_active.status = Set("disposed".to_string());
        asset_active.disposal_date = Set(Some(req.disposal_date));
        asset_active.save(&txn).await?;

        // 提交事务
        txn.commit().await?;

        info!(
            "资产 {} 处置成功，处置价值：{}，损益：{}",
            asset_id, req.disposal_value, disposal_gain_loss
        );
        Ok(())
    }

    /// V15 P1 17.8-D3：生成固定资产处置损益凭证
    /// 凭证分录（以"固定资产清理"为中间科目）：1. 结转固定资产原值：借：固定资产清理 1606 = 资产净值；借：累计折旧 1602 = 已计提累计折旧；贷：固定资产 1601 = 原值；2. 收到处置款项：借：银行存款 1002 = 处置收入；贷：固定资产清理 1606 = 处置收入；3. 结转处置损益：若收益（gain > 0）：借 固定资产清理 1606 / 贷 营业外收入 6301；若损失（gain < 0）：借 营业外支出 6711 / 贷 固定资产清理 1606
    async fn generate_disposal_voucher_txn(
        txn: &sea_orm::DatabaseTransaction,
        disposal: &crate::models::fixed_asset_disposal::Model,
        asset: &fixed_asset::Model,
        original_value: Decimal,
        accumulated_depreciation: Decimal,
        net_book_value: Decimal,
        disposal_value: Decimal,
        gain_loss: Decimal,
        user_id: i32,
    ) -> Result<(), AppError> {
        use crate::models::{voucher, voucher_item};

        let voucher_no = format!("FAD-{}", disposal.disposal_no);
        let summary = format!("固定资产处置-{}-{}", asset.asset_no, disposal.disposal_type);

        // 创建凭证主表
        let voucher_active = voucher::ActiveModel {
            voucher_no: Set(voucher_no.clone()),
            voucher_type: Set("transfer".to_string()),
            voucher_date: Set(disposal.disposal_date),
            source_type: Set(Some("fixed_asset_disposal".to_string())),
            source_module: Set(Some("fixed_asset".to_string())),
            source_bill_id: Set(Some(disposal.id)),
            source_bill_no: Set(Some(disposal.disposal_no.clone())),
            status: Set("DRAFT".to_string()),
            attachment_count: Set(0),
            created_by: Set(user_id),
            ..Default::default()
        };
        let voucher_model = voucher_active.insert(txn).await?;

        let mut line_no: i32 = 1;
        let mut entries: Vec<(String, String, Decimal, Decimal)> = Vec::new();

        // 1. 结转固定资产原值（借：固定资产清理 + 累计折旧，贷：固定资产）
        entries.push((
            "1606".to_string(),
            "固定资产清理".to_string(),
            net_book_value,
            Decimal::ZERO,
        ));
        entries.push((
            "1602".to_string(),
            "累计折旧".to_string(),
            accumulated_depreciation,
            Decimal::ZERO,
        ));
        entries.push((
            "1601".to_string(),
            "固定资产".to_string(),
            Decimal::ZERO,
            original_value,
        ));

        // 2. 收到处置款项（借：银行存款，贷：固定资产清理）
        if disposal_value > Decimal::ZERO {
            entries.push((
                "1002".to_string(),
                "银行存款".to_string(),
                disposal_value,
                Decimal::ZERO,
            ));
            entries.push((
                "1606".to_string(),
                "固定资产清理".to_string(),
                Decimal::ZERO,
                disposal_value,
            ));
        }

        // 3. 结转处置损益
        if gain_loss > Decimal::ZERO {
            // 收益：借 固定资产清理 / 贷 营业外收入
            entries.push((
                "1606".to_string(),
                "固定资产清理".to_string(),
                gain_loss,
                Decimal::ZERO,
            ));
            entries.push((
                "6301".to_string(),
                "营业外收入".to_string(),
                Decimal::ZERO,
                gain_loss,
            ));
        } else if gain_loss < Decimal::ZERO {
            // 损失：借 营业外支出 / 贷 固定资产清理
            let loss = -gain_loss;
            entries.push((
                "6711".to_string(),
                "营业外支出".to_string(),
                loss,
                Decimal::ZERO,
            ));
            entries.push((
                "1606".to_string(),
                "固定资产清理".to_string(),
                Decimal::ZERO,
                loss,
            ));
        }

        // 插入所有分录
        for (subject_code, subject_name, debit, credit) in entries {
            let item_active = voucher_item::ActiveModel {
                voucher_id: Set(voucher_model.id),
                line_no: Set(line_no),
                subject_code: Set(subject_code),
                subject_name: Set(subject_name),
                debit: Set(debit),
                credit: Set(credit),
                summary: Set(Some(summary.clone())),
                ..Default::default()
            };
            item_active.insert(txn).await?;
            line_no += 1;
        }

        info!(
            "固定资产处置凭证已生成：voucher_no={}, voucher_id={}, 分录数={}",
            voucher_no,
            voucher_model.id,
            line_no - 1
        );
        Ok(())
    }

    /// 查询指定资产的折旧历史记录（v3 复审 P1-3：折旧记录查询 API；按 created_at 倒序返回，补 .limit(10_000) 兜底（与批次 87 LIMIT 模式一致））
    pub async fn list_depreciation_records(
        &self,
        asset_id: i32,
    ) -> Result<Vec<crate::models::fixed_asset_depreciation_record::Model>, AppError> {
        let records = crate::models::fixed_asset_depreciation_record::Entity::find()
            .filter(crate::models::fixed_asset_depreciation_record::Column::AssetId.eq(asset_id))
            .order_by_desc(crate::models::fixed_asset_depreciation_record::Column::CreatedAt)
            .limit(10_000)
            .all(&*self.db)
            .await?;
        Ok(records)
    }

    /// 查询资产处置记录列表（v3 复审 P1-8：处置记录查询 API；按 created_at 倒序返回，补 .limit(10_000) 兜底（与批次 87 LIMIT 模式一致））
    pub async fn list_disposals(
        &self,
    ) -> Result<Vec<crate::models::fixed_asset_disposal::Model>, AppError> {
        let disposals = crate::models::fixed_asset_disposal::Entity::find()
            .order_by_desc(crate::models::fixed_asset_disposal::Column::CreatedAt)
            .limit(10_000)
            .all(&*self.db)
            .await?;
        Ok(disposals)
    }

    /// 删除资产（仅支持未使用状态）
    /// 批次 86 v2 复审 P2-9 修复：find + 状态门 + delete 移入单一事务 + lock_exclusive 串行化；原实现 find（get_by_id 内部 self.db）+ delete 在 self.db 上分别执行，无 txn 无 lock，；存在 TOCTOU（并发 depreciate/dispose 会基于过期状态通过检查后被误删）
    pub async fn delete(&self, asset_id: i32, user_id: i32) -> Result<(), AppError> {
        info!("用户 {} 正在删除资产 {}", user_id, asset_id);

        let txn = (*self.db).begin().await?;

        // 加 lock_exclusive 串行化并发状态变更
        let asset = fixed_asset::Entity::find_by_id(asset_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("固定资产不存在：{}", asset_id)))?;

        if asset.status != master_data::INACTIVE {
            return Err(AppError::validation("只能删除未使用状态的资产".to_string()));
        }

        fixed_asset::Entity::delete_many()
            .filter(fixed_asset::Column::Id.eq(asset_id))
            .exec(&txn)
            .await?;
        txn.commit().await?;

        info!("资产 {} 删除成功", asset_id);
        Ok(())
    }

    /// 批量计算折旧（仅预览，不持久化）
    /// v3 复审 P2-3：本方法为纯只读计算，不修改 fixed_asset 表的累计折旧/净值，；也不插入 fixed_asset_depreciation_records 记录。；如需持久化计提，请逐条调用 `depreciate(asset_id, period, user_id)`。；前端批量入口应先调本方法预览，用户确认后逐条调 depreciate 完成计提。；批次 92 P3-11 修复：入口加 asset_ids 长度校验（>10_000 拒绝），防止 IN 列表过长拖垮 DB；查询改用 `.paginate(&*self.db, 1000)` 流式拉取，避免一次性 `.all()` 内存峰值
    pub async fn batch_calculate_depreciation(
        &self,
        asset_ids: Vec<i32>,
        calculation_date: String,
        _user_id: i32,
    ) -> Result<Vec<DepreciationResult>, AppError> {
        use chrono::NaiveDate;

        // 批次 92 P3-11：长度校验，防止超大 IN 列表
        if asset_ids.is_empty() {
            return Ok(Vec::new());
        }
        if asset_ids.len() > 10_000 {
            return Err(AppError::validation(format!(
                "批量计算折旧的资产数量超限（{} > 10000），请分批调用",
                asset_ids.len()
            )));
        }

        let calc_date = calculation_date
            .parse::<NaiveDate>()
            .map_err(|_| AppError::validation("日期格式错误"))?;

        // 批次 92 P3-11：流式分页查询，避免一次性加载全部资产到内存
        // page_size=1000 在 IN(asset_ids) 过滤下，每页 IO 成本可控
        let paginator = fixed_asset::Entity::find()
            .filter(fixed_asset::Column::Id.is_in(asset_ids.clone()))
            .paginate(&*self.db, 1000);

        let mut asset_map: HashMap<i32, fixed_asset::Model> = HashMap::new();
        let num_pages = paginator.num_pages().await?;
        for page_idx in 0..num_pages {
            let page_items = paginator.fetch_page(page_idx).await?;
            for a in page_items {
                asset_map.insert(a.id, a);
            }
        }

        let mut results = Vec::new();

        for asset_id in asset_ids {
            let asset = asset_map
                .get(&asset_id)
                .ok_or_else(|| AppError::not_found("固定资产"))?;

            // 计算折旧
            let depreciation = self.calculate_asset_depreciation(asset, calc_date)?;

            results.push(DepreciationResult {
                asset_id: asset.id,
                asset_no: asset.asset_no.clone(),
                original_value: asset.original_value,
                accumulated_depreciation: asset.accumulated_depreciation + depreciation,
                current_depreciation: depreciation,
                net_value: asset.original_value - asset.accumulated_depreciation - depreciation,
                depreciation_method: asset.depreciation_method.clone().unwrap_or_default(),
            });
        }

        Ok(results)
    }

    /// 计算单项资产折旧
    fn calculate_asset_depreciation(
        &self,
        asset: &fixed_asset::Model,
        calc_date: NaiveDate,
    ) -> Result<rust_decimal::Decimal, AppError> {
        use chrono::Datelike;

        // P3 维度 3 修复（批次 87）：消除嵌套 expect，常量日期必然合法
        let purchase_date = asset
            .purchase_date
            .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap_or_default());
        // useful_life 缺失时按 0 年处理 → 不折旧（守卫见下）
        let useful_life_years = asset.useful_life.unwrap_or_default();
        let original_value = asset.original_value;
        let residual_value = asset.salvage_value.unwrap_or(Decimal::ZERO);

        if useful_life_years <= 0 {
            return Ok(rust_decimal::Decimal::ZERO);
        }

        // 计算已使用月数
        let months_used = (calc_date.year() - purchase_date.year()) * 12
            + (calc_date.month() as i32 - purchase_date.month() as i32);

        if months_used <= 0 {
            return Ok(rust_decimal::Decimal::ZERO);
        }

        // 直线法折旧：(原值 - 残值) / (使用年限 * 12)
        // P2-2 修复：补 round_dp(2)，与 calc_monthly_depreciation_for(line 167)
        // 批次 87 P3 维度 4 修复保持一致，防止 36 月等不能整除时累加误差
        let useful_life_months = useful_life_years * 12;
        let depreciable_amount = original_value - residual_value;
        let monthly_depreciation =
            (depreciable_amount / rust_decimal::Decimal::from(useful_life_months)).round_dp(2);

        // 总应计折旧 = 月折旧额 * min(已用月数, 总月数)
        let applicable_months = Ord::min(months_used, useful_life_months);
        let total_depreciation =
            monthly_depreciation * rust_decimal::Decimal::from(applicable_months);

        // 本次应计提 = 总应计折旧 - 已计提折旧
        let current_depreciation = total_depreciation - asset.accumulated_depreciation;
        Ok(current_depreciation.max(rust_decimal::Decimal::ZERO))
    }

    /// V15 P1 17.8-D2：月末自动计提折旧
    /// 供 cron/scheduler 月末调用，遍历所有 active 状态资产按指定期间计提折旧。；单资产失败不中断整体流程，记录到 failures 列表返回，保证批处理韧性。；幂等性由 `uk_fa_depreciation_records_asset_period` 唯一约束保证（重复计提会被跳过）。
    pub async fn auto_monthly_depreciation(
        &self,
        period: &str,
        user_id: i32,
    ) -> Result<AutoDepreciationSummary, AppError> {
        info!("自动计提折旧开始：期间={}, 触发人={}", period, user_id);

        // 分页拉取所有 active 资产，避免一次性加载
        let paginator = fixed_asset::Entity::find()
            .filter(fixed_asset::Column::Status.eq(master_data::ACTIVE))
            .paginate(&*self.db, 500);

        let num_pages = paginator.num_pages().await?;
        let mut total_scanned = 0u64;
        let mut success_count = 0u64;
        let mut skipped_count = 0u64;
        let mut failures: Vec<AutoDepreciationFailure> = Vec::new();

        for page_idx in 0..num_pages {
            let page_items = paginator.fetch_page(page_idx).await?;
            for asset in page_items {
                total_scanned += 1;
                let asset_id = asset.id;
                let asset_no = asset.asset_no.clone();
                match self.depreciate(asset_id, period, user_id).await {
                    Ok(()) => success_count += 1,
                    Err(e) => {
                        // 区分"跳过"（零折旧/已足额）与真实失败
                        let err_str = e.to_string();
                        if err_str.contains("重复计提") || err_str.contains("已足额") {
                            skipped_count += 1;
                        } else {
                            tracing::warn!(
                                asset_id,
                                asset_no = %asset_no,
                                error = %err_str,
                                "自动计提折旧单资产失败"
                            );
                            failures.push(AutoDepreciationFailure {
                                asset_id,
                                asset_no,
                                error: err_str,
                            });
                        }
                    }
                }
            }
        }

        let summary = AutoDepreciationSummary {
            period: period.to_string(),
            total_scanned,
            success_count,
            skipped_count,
            failure_count: failures.len() as u64,
            failures,
        };
        info!(
            "自动计提折旧完成：期间={}, 扫描={}, 成功={}, 跳过={}, 失败={}",
            period,
            summary.total_scanned,
            summary.success_count,
            summary.skipped_count,
            summary.failure_count
        );
        Ok(summary)
    }

    /// V15 P1 17.8-D4：创建资产盘点计划（按资产类别/存放地点筛选资产生成盘点计划，状态 DRAFT→COUNTING→COMPLETED。）
    pub async fn create_count_plan(
        &self,
        req: CreateCountPlanRequest,
        user_id: i32,
    ) -> Result<fixed_asset_count::Model, AppError> {
        info!("用户 {} 正在创建资产盘点计划：{}", user_id, req.plan_name);

        let txn = (*self.db).begin().await?;

        let count_no = format!(
            "FAC{}{:06}",
            chrono::Utc::now().format("%Y%m%d"),
            crate::utils::random::random_4_digit()
        );

        let count_date = req
            .count_date
            .unwrap_or_else(|| chrono::Utc::now().date_naive());

        let plan = fixed_asset_count::ActiveModel {
            count_no: Set(count_no.clone()),
            plan_name: Set(req.plan_name.clone()),
            count_date: Set(count_date),
            asset_category: Set(req.asset_category.clone()),
            use_location: Set(req.use_location.clone()),
            status: Set("DRAFT".to_string()),
            total_items: Set(0),
            counted_items: Set(0),
            surplus_items: Set(0),
            shortage_items: Set(0),
            notes: Set(req.notes.clone()),
            created_by: Set(user_id),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        // 按筛选条件拉取资产并生成盘点明细
        let mut query =
            fixed_asset::Entity::find().filter(fixed_asset::Column::Status.eq(master_data::ACTIVE));
        if let Some(category) = &req.asset_category {
            query = query.filter(fixed_asset::Column::AssetCategory.eq(category));
        }
        if let Some(location) = &req.use_location {
            query = query.filter(fixed_asset::Column::UseLocation.eq(location));
        }

        let assets = query.all(&txn).await?;
        let mut total_items = 0i32;
        for asset in assets {
            fixed_asset_count_item::ActiveModel {
                count_id: Set(plan.id),
                asset_id: Set(asset.id),
                asset_no: Set(asset.asset_no.clone()),
                asset_name: Set(asset.asset_name.clone()),
                book_original_value: Set(asset.original_value),
                book_net_value: Set(asset.net_value),
                book_use_location: Set(asset.use_location.clone()),
                actual_original_value: Set(None),
                actual_net_value: Set(None),
                actual_use_location: Set(None),
                count_result: Set(None),
                variance_type: Set(None),
                variance_amount: Set(None),
                remarks: Set(None),
                ..Default::default()
            }
            .insert(&txn)
            .await?;
            total_items += 1;
        }

        // 回填 total_items
        let mut plan_update: fixed_asset_count::ActiveModel = plan.clone().into();
        plan_update.total_items = Set(total_items);
        let plan_final = plan_update.update(&txn).await?;

        txn.commit().await?;
        info!(
            "资产盘点计划创建成功：{}，明细 {} 项",
            count_no, total_items
        );
        Ok(plan_final)
    }

    /// V15 P1 17.8-D4：录入盘点结果（单条）（count_result: "consistent"=一致, "surplus"=盘盈, "shortage"=盘亏, "damaged"=毁损）
    pub async fn record_count_item(
        &self,
        count_id: i32,
        asset_id: i32,
        actual_original_value: Option<Decimal>,
        actual_net_value: Option<Decimal>,
        actual_use_location: Option<String>,
        count_result: String,
        remarks: Option<String>,
        user_id: i32,
    ) -> Result<fixed_asset_count_item::Model, AppError> {
        info!(
            "用户 {} 录入盘点结果：盘点单={}, 资产={}, 结果={}",
            user_id, count_id, asset_id, count_result
        );

        let txn = (*self.db).begin().await?;

        // 校验盘点单状态
        let plan = fixed_asset_count::Entity::find_by_id(count_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("盘点计划不存在：{}", count_id)))?;

        if plan.status == "COMPLETED" {
            return Err(AppError::validation("盘点计划已完成，不可修改".to_string()));
        }
        // 自动将 DRAFT 切换为 COUNTING
        if plan.status == "DRAFT" {
            let mut plan_active: fixed_asset_count::ActiveModel = plan.into();
            plan_active.status = Set("COUNTING".to_string());
            plan_active.update(&txn).await?;
        }

        let item = fixed_asset_count_item::Entity::find()
            .filter(fixed_asset_count_item::Column::CountId.eq(count_id))
            .filter(fixed_asset_count_item::Column::AssetId.eq(asset_id))
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!(
                    "盘点明细不存在：count_id={}, asset_id={}",
                    count_id, asset_id
                ))
            })?;

        let book_original = item.book_original_value;
        let book_net = item.book_net_value.unwrap_or(Decimal::ZERO);
        let actual_orig = actual_original_value.unwrap_or(book_original);
        let actual_net = actual_net_value.unwrap_or(book_net);

        // 计算差异类型与金额
        let (variance_type, variance_amount) = match count_result.as_str() {
            "consistent" => (None, None),
            "surplus" => {
                let v = (actual_orig - book_original).max(Decimal::ZERO);
                (Some("surplus".to_string()), Some(v))
            }
            "shortage" => {
                let v = (book_original - actual_orig).max(Decimal::ZERO);
                (Some("shortage".to_string()), Some(v))
            }
            "damaged" => {
                // 毁损：净值差异
                let v = (book_net - actual_net).max(Decimal::ZERO);
                (Some("damaged".to_string()), Some(v))
            }
            other => {
                return Err(AppError::validation(format!(
                    "不支持的盘点结果类型：{}",
                    other
                )));
            }
        };

        let mut item_active: fixed_asset_count_item::ActiveModel = item.into();
        item_active.actual_original_value = Set(Some(actual_orig));
        item_active.actual_net_value = Set(Some(actual_net));
        item_active.actual_use_location = Set(actual_use_location);
        item_active.count_result = Set(Some(count_result));
        item_active.variance_type = Set(variance_type);
        item_active.variance_amount = Set(variance_amount);
        item_active.remarks = Set(remarks);
        item_active.counted_by = Set(Some(user_id));
        item_active.counted_at = Set(Some(chrono::Utc::now()));
        let updated = item_active.update(&txn).await?;

        // 更新盘点单 counted_items
        let counted = fixed_asset_count_item::Entity::find()
            .filter(fixed_asset_count_item::Column::CountId.eq(count_id))
            .filter(fixed_asset_count_item::Column::CountResult.is_not_null())
            .count(&txn)
            .await?;
        let mut plan_active = fixed_asset_count::ActiveModel {
            id: sea_orm::ActiveValue::Unchanged(count_id),
            ..Default::default()
        };
        plan_active.counted_items = Set(counted as i32);
        plan_active.update(&txn).await?;

        txn.commit().await?;
        Ok(updated)
    }

    /// V15 P1 17.8-D4：完成盘点并生成盘盈盘亏处理（统计盘盈/盘亏数量，将盘点单置为 COMPLETED。；盘亏资产标记为 INACTIVE（待处置），盘盈资产需手工建档。）
    pub async fn complete_count_plan(
        &self,
        count_id: i32,
        user_id: i32,
    ) -> Result<CountCompletionSummary, AppError> {
        info!("用户 {} 正在完成资产盘点计划：{}", user_id, count_id);

        let txn = (*self.db).begin().await?;

        let plan = fixed_asset_count::Entity::find_by_id(count_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("盘点计划不存在：{}", count_id)))?;

        if plan.status == "COMPLETED" {
            return Err(AppError::validation("盘点计划已完成".to_string()));
        }

        let items = fixed_asset_count_item::Entity::find()
            .filter(fixed_asset_count_item::Column::CountId.eq(count_id))
            .all(&txn)
            .await?;

        let mut surplus_count = 0i32;
        let mut shortage_count = 0i32;
        let mut damaged_count = 0i32;
        let mut surplus_value = Decimal::ZERO;
        let mut shortage_value = Decimal::ZERO;
        let mut damaged_value = Decimal::ZERO;

        for item in &items {
            match item.variance_type.as_deref() {
                Some("surplus") => {
                    surplus_count += 1;
                    surplus_value += item.variance_amount.unwrap_or(Decimal::ZERO);
                }
                Some("shortage") => {
                    shortage_count += 1;
                    shortage_value += item.variance_amount.unwrap_or(Decimal::ZERO);
                    // 盘亏资产标记为 INACTIVE
                    let asset = fixed_asset::Entity::find_by_id(item.asset_id)
                        .one(&txn)
                        .await?;
                    if let Some(a) = asset {
                        let mut a_active: fixed_asset::ActiveModel = a.into();
                        a_active.status = Set(master_data::INACTIVE.to_string());
                        a_active.update(&txn).await?;
                    }
                }
                Some("damaged") => {
                    damaged_count += 1;
                    damaged_value += item.variance_amount.unwrap_or(Decimal::ZERO);
                }
                _ => {}
            }
        }

        // 更新盘点单状态
        let mut plan_active: fixed_asset_count::ActiveModel = plan.into();
        plan_active.status = Set("COMPLETED".to_string());
        plan_active.surplus_items = Set(surplus_count);
        plan_active.shortage_items = Set(shortage_count + damaged_count);
        plan_active.completed_at = Set(Some(chrono::Utc::now()));
        plan_active.approved_by = Set(Some(user_id));
        plan_active.update(&txn).await?;

        txn.commit().await?;

        let summary = CountCompletionSummary {
            count_id,
            total_items: items.len() as i32,
            surplus_count,
            shortage_count,
            damaged_count,
            surplus_value,
            shortage_value,
            damaged_value,
        };
        info!(
            "资产盘点完成：count_id={}, 盘盈={}, 盘亏={}, 毁损={}",
            count_id, surplus_count, shortage_count, damaged_count
        );
        Ok(summary)
    }

    /// V15 P1 17.8-D4：查询盘点计划列表
    pub async fn list_count_plans(
        &self,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<fixed_asset_count::Model>, u64), AppError> {
        let paginator = fixed_asset_count::Entity::find()
            .order_by(fixed_asset_count::Column::Id, Order::Desc)
            .paginate(&*self.db, page_size.clamp(1, 100) as u64);
        let (plans, total) = paginate_with_total(paginator, page.clamp(1, 1000) as u64).await?;
        Ok((plans, total))
    }

    /// V15 P1 17.8-D4：查询盘点明细
    pub async fn list_count_items(
        &self,
        count_id: i32,
    ) -> Result<Vec<fixed_asset_count_item::Model>, AppError> {
        let items = fixed_asset_count_item::Entity::find()
            .filter(fixed_asset_count_item::Column::CountId.eq(count_id))
            .order_by(fixed_asset_count_item::Column::AssetId, Order::Asc)
            .all(&*self.db)
            .await?;
        Ok(items)
    }
}

/// 折旧结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct DepreciationResult {
    pub asset_id: i32,
    pub asset_no: String,
    pub original_value: rust_decimal::Decimal,
    pub accumulated_depreciation: rust_decimal::Decimal,
    pub current_depreciation: rust_decimal::Decimal,
    pub net_value: rust_decimal::Decimal,
    pub depreciation_method: String,
}

/// V15 P1 17.8-D2：自动计提折旧摘要
#[derive(Debug, Clone, serde::Serialize)]
pub struct AutoDepreciationSummary {
    pub period: String,
    pub total_scanned: u64,
    pub success_count: u64,
    pub skipped_count: u64,
    pub failure_count: u64,
    pub failures: Vec<AutoDepreciationFailure>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AutoDepreciationFailure {
    pub asset_id: i32,
    pub asset_no: String,
    pub error: String,
}

/// V15 P1 17.8-D4：创建盘点计划请求
#[derive(Debug, Clone)]
pub struct CreateCountPlanRequest {
    pub plan_name: String,
    pub asset_category: Option<String>,
    pub use_location: Option<String>,
    pub count_date: Option<NaiveDate>,
    pub notes: Option<String>,
}

/// V15 P1 17.8-D4：盘点完成摘要
#[derive(Debug, Clone, serde::Serialize)]
pub struct CountCompletionSummary {
    pub count_id: i32,
    pub total_items: i32,
    pub surplus_count: i32,
    pub shortage_count: i32,
    pub damaged_count: i32,
    pub surplus_value: Decimal,
    pub shortage_value: Decimal,
    pub damaged_value: Decimal,
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    /// 测试折旧计算逻辑（直接调用内部方法）
    /// 由于 calculate_asset_depreciation 是私有方法，我们通过测试计算逻辑来验证
    #[test]
    fn test_depreciation_calculation_logic() {
        // 原值 100000，残值 10000，使用寿命 120 个月
        let original_value = Decimal::from(100000);
        let salvage_value = Decimal::from(10000);
        let useful_life = 120i32;

        // 可折旧金额
        let depreciable_amount = original_value - salvage_value;
        assert_eq!(depreciable_amount, Decimal::from(90000));

        // 月折旧额
        let monthly_depreciation = depreciable_amount / Decimal::from(useful_life);
        assert_eq!(monthly_depreciation, Decimal::from(750));

        // 36 个月折旧
        let months_used = 36;
        let total_depreciation = monthly_depreciation * Decimal::from(months_used);
        assert_eq!(total_depreciation, Decimal::from(27000));
    }

    #[test]
    fn test_depreciation_with_accumulated() {
        let total_depreciation = Decimal::from(27000);
        let accumulated_depreciation = Decimal::from(10000);

        // 当期折旧 = 总折旧 - 已累计折旧
        let current_depreciation = total_depreciation - accumulated_depreciation;
        assert_eq!(current_depreciation, Decimal::from(17000));
    }

    #[test]
    fn test_depreciation_fully_depreciated() {
        let original_value = Decimal::from(100000);
        let salvage_value = Decimal::from(10000);
        let useful_life = 120i32;
        let months_used = 150; // 超过使用寿命

        let depreciable_amount = original_value - salvage_value;
        let monthly_depreciation = depreciable_amount / Decimal::from(useful_life);

        // 折旧不能超过可折旧金额
        let max_depreciation = depreciable_amount;
        let calculated =
            monthly_depreciation * Decimal::from(std::cmp::Ord::min(months_used, useful_life));

        assert_eq!(calculated, max_depreciation);
    }

    #[test]
    fn test_net_value_calculation() {
        let original_value = Decimal::from(100000);
        let accumulated_depreciation = Decimal::from(27000);

        let net_value = original_value - accumulated_depreciation;
        assert_eq!(net_value, Decimal::from(73000));
    }

    #[test]
    fn test_depreciation_before_purchase() {
        // 购买日期晚于计算日期，应返回 0
        let purchase_year = 2025;
        let calc_year = 2024;

        let months_used = (calc_year - purchase_year) * 12;
        assert!(months_used < 0, "购买前不应计算折旧");
    }

    #[test]
    fn test_various_depreciation_scenarios() {
        let test_cases = vec![
            // (原值, 残值, 使用寿命月, 已用月数, 期望折旧)
            (100000, 10000, 120, 12, 9000),   // 1 年
            (100000, 10000, 120, 36, 27000),  // 3 年
            (100000, 10000, 120, 60, 45000),  // 5 年
            (100000, 10000, 120, 120, 90000), // 满寿命
            (50000, 5000, 60, 24, 18000),     // 另一设备
        ];

        for (original, salvage, life, months, expected) in test_cases {
            let original_value = Decimal::from(original);
            let salvage_value = Decimal::from(salvage);
            let depreciable = original_value - salvage_value;
            let monthly = depreciable / Decimal::from(life);
            let total = monthly * Decimal::from(std::cmp::Ord::min(months, life));

            assert_eq!(
                total,
                Decimal::from(expected),
                "原值={}, 残值={}, 寿命={}, 月数={} 的折旧计算错误",
                original,
                salvage,
                life,
                months
            );
        }
    }

    /// 测试处置损益计算：处置价值 > 账面净值 → 收益为正
    /// 对应 dispose 方法 line 331-333 的计算逻辑：`net_book_value = asset.net_value.unwrap_or(Decimal::ZERO)`；`disposal_gain_loss = req.disposal_value - net_book_value`；gain_loss 计算公式验证，完整 dispose 事务流程需集成测试
    #[test]
    fn test_disposal_gain_loss_positive() {
        // 资产：原值 10000，累计折旧 2000，账面净值 8000
        // net_book_value 对应 dispose 方法中 asset.net_value.unwrap_or(Decimal::ZERO)
        let net_book_value = Decimal::from(8000);
        let disposal_value = Decimal::from(9000);

        // 模拟 dispose 方法 line 333 的损益计算公式
        let gain_loss = disposal_value - net_book_value;

        assert_eq!(gain_loss, Decimal::from(1000));
        assert!(
            gain_loss > Decimal::ZERO,
            "处置价值 > 账面净值应为收益（正数）"
        );
    }

    /// 测试处置损益计算：处置价值 < 账面净值 → 损失为负（gain_loss 计算公式验证，完整 dispose 事务流程需集成测试）
    #[test]
    fn test_disposal_gain_loss_negative() {
        // 同一资产，账面净值 8000，处置价值仅 7000
        let net_book_value = Decimal::from(8000);
        let disposal_value = Decimal::from(7000);

        let gain_loss = disposal_value - net_book_value;

        assert_eq!(gain_loss, Decimal::from(-1000));
        assert!(
            gain_loss < Decimal::ZERO,
            "处置价值 < 账面净值应为损失（负数）"
        );
    }

    /// 测试处置损益计算：处置价值 = 账面净值 → 损益为 0（gain_loss 计算公式验证，完整 dispose 事务流程需集成测试）
    #[test]
    fn test_disposal_gain_loss_zero() {
        let net_book_value = Decimal::from(8000);
        let disposal_value = Decimal::from(8000);

        let gain_loss = disposal_value - net_book_value;

        assert_eq!(gain_loss, Decimal::ZERO);
    }

    /// 测试 calculate_asset_depreciation 的 round_dp(2) 精度行为
    /// 构造资产：original_value=10000, salvage_value=Some(0), useful_life=Some(3)（36 个月）；月折旧 = (10000 - 0) / 36 = 277.7777...，round_dp(2) 四舍五入为 277.78；calculate_asset_depreciation 是私有方法且需 &self（FixedAssetService 含 DatabaseConnection），；此处验证其内部 round_dp(2) 精度逻辑，完整方法调用需集成测试
    #[test]
    fn test_calculate_asset_depreciation_round_dp() {
        let original_value = Decimal::from(10000);
        let salvage_value = Decimal::from(0);
        let useful_life_years = 3i32;

        // 复刻 calculate_asset_depreciation line 516-520 的月折旧计算
        let useful_life_months = useful_life_years * 12;
        let depreciable_amount = original_value - salvage_value;
        let monthly_depreciation =
            (depreciable_amount / Decimal::from(useful_life_months)).round_dp(2);

        // 10000/36 = 277.7777...，round_dp(2) 采用 MidpointAwayFromZero 四舍五入，
        // 第 3 位小数 7 >= 5 进位，结果为 277.78（Decimal::new(27778, 2) = 277.78）
        assert_eq!(monthly_depreciation, Decimal::new(27778, 2));

        // 验证 round 确实发生：未 round 的无限循环小数与 round 后值不同
        let unrounded = depreciable_amount / Decimal::from(useful_life_months);
        assert_ne!(
            monthly_depreciation, unrounded,
            "round_dp(2) 必须截断无限循环小数"
        );

        // 验证精度为 2 位小数：再次 round_dp(2) 值不变
        assert_eq!(monthly_depreciation.round_dp(2), monthly_depreciation);
    }
}
