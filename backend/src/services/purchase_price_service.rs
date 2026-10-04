use crate::models::purchase_price;
use crate::models::status::price_approval;
use crate::models::{product, supplier};
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult, JoinType,
    ModelTrait, Order, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, RelationTrait, Set,
    TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use validator::Validate;

/// 采购价格读模型：实体列 + LEFT JOIN 关联出的产品名 / 产品编码 / 供应商名（实体仅有外键 ID）。
///
/// JOIN 名列均 `Option<String>`；实体自身列按 `purchase_price` 约束保持原类型。
#[derive(Debug, Clone, Serialize, FromQueryResult)]
pub struct PurchasePriceView {
    pub id: i32,
    pub product_id: i32,
    pub supplier_id: i32,
    pub price: Decimal,
    pub currency: String,
    pub unit: String,
    pub min_order_qty: Decimal,
    pub price_type: String,
    pub effective_date: chrono::NaiveDate,
    pub expiry_date: Option<chrono::NaiveDate>,
    pub status: String,
    pub approved_by: Option<i32>,
    pub approved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_by: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub product_name: Option<String>,
    pub product_code: Option<String>,
    pub supplier_name: Option<String>,
}

/// 采购价格查询参数
#[derive(Debug, Clone, Default)]
pub struct PurchasePriceQueryParams {
    pub product_id: Option<i32>,
    pub supplier_id: Option<i32>,
    pub status: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

/// 创建采购价格请求
///
/// `unit`（计量单位）与 `price_type`（价格类型）为创建必填：价格必依附计量单位与价格类型，
/// 对应列 `purchase_prices.unit` / `purchase_prices.price_type` 为 NOT NULL 且无数据库默认值。
/// 缺失/空字符串由 validator 返回 4xx VALIDATION_ERROR（单一 AppError 信封），
/// 不再依赖 DB NOT NULL 约束裸抛 500 DATABASE_ERROR。
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreatePurchasePriceInput {
    pub product_id: i32,
    pub supplier_id: i32,
    pub price: rust_decimal::Decimal,
    pub currency: Option<String>,
    #[validate(length(min = 1, message = "计量单位不能为空"))]
    pub unit: String,
    #[validate(length(min = 1, message = "价格类型不能为空"))]
    pub price_type: String,
    pub min_order_qty: Option<rust_decimal::Decimal>,
    pub effective_date: Option<String>,
    pub expiry_date: Option<String>,
}

pub struct PurchasePriceService {
    db: Arc<DatabaseConnection>,
}

impl PurchasePriceService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 产品引用存在性预检。
    ///
    /// 范式照 `sku_mapping_service::validate_refs` 与 `inventory_reservation_service::create_reservation`：
    /// 坏引用绝不交给 DB 兜底，理由有二——
    /// 1. `purchase_prices` 当前**没有**指向 products/suppliers 的外键（FK 迁移尚未上线），
    ///    不给应用层预检则不存在的 product_id/supplier_id 静默落库成孤儿价目行
    ///    （列表侧 `PurchasePriceView` 的 JOIN 名列如实 NULL 即其下游形态）；
    /// 2. 即便 FK 上线，23503 违约经 `AppError::From<DbErr>` 的 Exec 分支只落裸
    ///    500 `DATABASE_ERROR`（同 `inventory_reservation_service::create_reservation` 头注
    ///    所述缺陷族），用户拿不到可外显的拒绝原因；引用不存在是「用户自己提交的字段非法」，
    ///    按 `utils/error.rs` 模块文档的铁律走 `AppError::validation_displayable`
    ///    （HTTP 400 + code=VALIDATION_ERROR，出参携带真实原因；文案只含请求字段自身的 ID，
    ///    满足其安全边界）。
    ///
    /// 存在性判定只取主键列（同 `sku_mapping_service::validate_refs` 对 suppliers 的
    /// select_only 口径——`models/supplier.rs` 把 supplier_type/credit_code/legal_representative
    /// 等经 ALTER 以可空列加入的扩展列声明为非 Option，整行解码会在历史/手工行上误抛 500，
    /// 该实证注释就写在 `sku_mapping_service::validate_refs` 内）；products 同口径统一。
    async fn assert_product_exists(&self, product_id: i32) -> Result<(), AppError> {
        let exists = product::Entity::find()
            .filter(product::Column::Id.eq(product_id))
            .select_only()
            .column(product::Column::Id)
            .into_tuple::<i32>()
            .one(&*self.db)
            .await?
            .is_some();
        if !exists {
            return Err(AppError::validation_displayable(format!(
                "产品 ID {product_id} 不存在"
            )));
        }
        Ok(())
    }

    /// 供应商引用存在性预检（同 `Self::assert_product_exists` 口径）
    async fn assert_supplier_exists(&self, supplier_id: i32) -> Result<(), AppError> {
        let exists = supplier::Entity::find()
            .filter(supplier::Column::Id.eq(supplier_id))
            .select_only()
            .column(supplier::Column::Id)
            .into_tuple::<i32>()
            .one(&*self.db)
            .await?
            .is_some();
        if !exists {
            return Err(AppError::validation_displayable(format!(
                "供应商 ID {supplier_id} 不存在"
            )));
        }
        Ok(())
    }

    /// 获取采购价格列表
    pub async fn get_prices_list(
        &self,
        params: PurchasePriceQueryParams,
    ) -> Result<(Vec<PurchasePriceView>, u64), AppError> {
        let mut query = purchase_price::Entity::find();

        if let Some(product_id) = params.product_id {
            query = query.filter(purchase_price::Column::ProductId.eq(product_id));
        }

        if let Some(supplier_id) = params.supplier_id {
            query = query.filter(purchase_price::Column::SupplierId.eq(supplier_id));
        }

        if let Some(status) = &params.status {
            query = query.filter(purchase_price::Column::Status.eq(status));
        }

        // 总数在无 JOIN 的基础查询上统计：所有 JOIN 均为多对一（不倍增行），单次查询无 N+1。
        let total = query.clone().count(&*self.db).await?;

        let prices = query
            .column_as(product::Column::Name, "product_name")
            .column_as(product::Column::Code, "product_code")
            .column_as(supplier::Column::SupplierName, "supplier_name")
            .join(JoinType::LeftJoin, purchase_price::Relation::Product.def())
            .join(JoinType::LeftJoin, purchase_price::Relation::Supplier.def())
            .order_by(purchase_price::Column::Id, Order::Desc)
            // 批次 98 P2-A 修复（v5 复审）：page clamp 防 DoS
            .offset((params.page.clamp(1, 1000).saturating_sub(1) * params.page_size) as u64)
            .limit(params.page_size as u64)
            .into_model::<PurchasePriceView>()
            .all(&*self.db)
            .await?;

        Ok((prices, total))
    }

    /// 创建采购价格
    pub async fn create_price(
        &self,
        req: CreatePurchasePriceInput,
        user_id: i32,
    ) -> Result<purchase_price::Model, AppError> {
        info!(
            "用户 {} 正在创建采购价格，产品 ID: {}, 供应商 ID: {}",
            user_id, req.product_id, req.supplier_id
        );

        // 引用存在性预检先于任何写库动作（口径见 `Self::assert_product_exists`
        // 文档注）：product_id/supplier_id 均为 NOT NULL 必检；拒绝路径零副作用（不落行）。
        // update_price 不收 product_id/supplier_id 键（无引用改道），故其路径无需预检。
        self.assert_product_exists(req.product_id).await?;
        self.assert_supplier_exists(req.supplier_id).await?;

        let active_price = purchase_price::ActiveModel {
            product_id: Set(req.product_id),
            supplier_id: Set(req.supplier_id),
            price: Set(req.price),
            currency: Set(req
                .currency
                .unwrap_or_else(|| crate::constants::DEFAULT_CURRENCY.to_string())),
            unit: Set(req.unit),
            price_type: Set(req.price_type),
            min_order_qty: Set(req.min_order_qty.unwrap_or_default()),
            effective_date: Set(req
                .effective_date
                .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string())
                .parse()
                .map_err(|e| AppError::validation_displayable(format!("日期格式错误：{}", e)))?),
            // 到期日与生效日同口径严格解析（map + map_err + transpose，写法与销售侧
            // `services/sales_price_service::create_price` 的 expiry_date 分支同形）：
            // 非法日期串一律 fail-visible 拒绝，绝不落 NULL——NULL 是「长期有效」的既有
            // 业务语义，不得让非法输入冒名顶替成成功行。错误走
            // `AppError::validation_displayable`：HTTP 400 + 机器码 VALIDATION_ERROR，
            // 出参携带真实拒绝原因；文案只描述用户自己提交的字段格式，满足
            // `utils/error.rs` 模块文档的安全边界（先例：
            // `handlers/budget_management_handler.rs` / `handlers/inventory_count_handler.rs`
            // 的日期解析拒绝点）。禁止改用 `AppError::business`（出参被脱敏成固定文案，
            // 用户看不到原因），也禁止 `AppError::internal`（把校验失败拍平成 500）。
            expiry_date: Set(req
                .expiry_date
                .map(|d| {
                    d.parse().map_err(|e| {
                        AppError::validation_displayable(format!("日期格式错误：{}", e))
                    })
                })
                .transpose()?),
            status: Set(price_approval::PENDING.to_string()),
            created_by: Set(Some(user_id)),
            ..Default::default()
        };

        let price = active_price.insert(&*self.db).await?;
        info!("采购价格创建成功，ID: {}", price.id);
        Ok(price)
    }

    /// 获取采购价格
    pub async fn get_price(&self, id: i32) -> Result<purchase_price::Model, AppError> {
        info!("查询采购价格，ID: {}", id);

        let price = purchase_price::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购价格 {} 未找到", id)))?;

        Ok(price)
    }

    /// 批准采购价格
    pub async fn approve_price(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        info!("用户 {} 正在批准采购价格，ID: {}", user_id, id);

        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        let txn = (*self.db).begin().await?;

        let price_model = purchase_price::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购价格 {} 未找到", id)))?;

        // 状态门：价格流转前置与写入值逐字符同源（price_approval::PENDING）。非待审批
        // （approved/inactive）直接批准属状态机前置未满足，归业务族；文案含当前状态
        // token，保持脱敏（与销售侧 approve_price 同一口径，出参 code=BUSINESS_ERROR）。
        if price_model.status != price_approval::PENDING {
            return Err(AppError::business(format!(
                "只有待审批状态的采购价格可以批准，当前状态：{}",
                price_model.status
            )));
        }

        let mut price: purchase_price::ActiveModel = price_model.into();
        price.status = Set(price_approval::APPROVED.to_string());
        price.approved_by = Set(Some(user_id));
        price.approved_at = Set(Some(chrono::Utc::now()));

        // 使用 update_with_audit 在事务内同步写入审计日志
        // P2-3 修复（批次 84 v1 复审）：有意忽略返回的 ActiveModel（字段已通过 Set 表达更新意图），仅传播错误
        // 批次 94 P2-11：审计日志为关键路径，错误已通过 ? 传播；去掉 let _ = 直接丢弃 ActiveModel 返回值
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            price,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        info!("采购价格批准成功，ID: {}", id);
        Ok(())
    }

    /// 获取价格历史
    pub async fn get_price_history(
        &self,
        material_id: i32,
    ) -> Result<Vec<purchase_price::Model>, AppError> {
        info!("查询物料 {} 的价格历史", material_id);

        let history = purchase_price::Entity::find()
            .filter(purchase_price::Column::ProductId.eq(material_id))
            .order_by(purchase_price::Column::EffectiveDate, Order::Desc)
            .all(&*self.db)
            .await?;

        Ok(history)
    }

    pub async fn update_price(
        &self,
        id: i32,
        price: Decimal,
        expiry_date: Option<String>,
        status: Option<String>,
    ) -> Result<(), AppError> {
        info!("更新采购价格，ID: {}", id);

        let mut price_model: purchase_price::ActiveModel = purchase_price::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购价格 {} 未找到", id)))?
            .into();

        price_model.price = Set(price);
        if let Some(ed) = expiry_date {
            price_model.expiry_date =
                Set(Some(ed.parse().map_err(|e| {
                    AppError::validation_displayable(format!("日期格式错误：{}", e))
                })?));
        }
        if let Some(s) = status {
            // 入参取值域校验先于 DB CHECK：非法值归字段校验族（400/VALIDATION_ERROR），
            // 避免撞 chk_purchase_price_status 退化为裸 500 DATABASE_ERROR。
            if !price_approval::ALL.contains(&s.as_str()) {
                return Err(AppError::validation_displayable(format!(
                    "价格状态 {} 不是合法取值，允许值：{}",
                    s,
                    price_approval::ALL.join("/")
                )));
            }
            price_model.status = Set(s);
        }

        price_model.save(&*self.db).await?;
        info!("采购价格更新成功，ID: {}", id);
        Ok(())
    }

    pub async fn delete_price(&self, id: i32) -> Result<(), AppError> {
        info!("删除采购价格，ID: {}", id);

        let price = purchase_price::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购价格 {} 未找到", id)))?;

        price.delete(&*self.db).await?;
        info!("采购价格删除成功，ID: {}", id);
        Ok(())
    }
}
