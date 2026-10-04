use crate::models::sales_price;
use crate::models::status::price_approval;
use crate::models::status::purchase_inventory::inventory_stock_grade;
use crate::models::{customer, product};
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, FromQueryResult,
    IntoActiveModel, JoinType, Order, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
    RelationTrait, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use validator::Validate;

/// 双层 Option 反序列化适配器（本仓权威范式，见 `department_handler.rs:46` /
/// `api_gateway_handler.rs:104`）。serde 对 `Option<Option<T>>` 的默认行为在遇到
/// JSON null 时直接 `visit_none()`，把「显式 null」塌成外层 `None`，与「键缺席」
/// 不可区分，三态清空从 HTTP 根本发不出来；本适配器先按内层 `Option<T>` 反序列化
/// 再包一层：键缺席（配 `#[serde(default)]`）= `None`、显式 null = `Some(None)`、
/// 有值 = `Some(Some(v))`。
fn double_option<'de, T, D>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Deserialize::deserialize(de).map(Some)
}

/// 手工维护销售价目的价格等级白名单（保守集）。
///
/// 词表**唯一来源**是 `inventory_stock_grade`（models/status/purchase_inventory.rs:308-320，
/// 一等品/二等品/等外品）——该列的真实系统写入方与过滤消费方是质检降级联动
/// （quality_inspection_service.rs:53-54 引用同一组词表常量）；抄字面量数组进来正是
/// 本缺陷族（前端 A/B/C/D 伪词表）的成因，禁止。
/// TODO(待用户终裁)：「等外品」(`inventory_stock_grade::OFF_GRADE`) 是否开放手工定价
/// 入口尚未拍板（决策建议书 §5 裁定 2 保守推荐仅一等/二等），批了才扩白名单，本批不扩。
const SALES_PRICE_LEVEL_ALLOWED: &[&str] =
    &[inventory_stock_grade::FIRST, inventory_stock_grade::SECOND];

/// 价格等级入参白名单校验：空串/纯空白/缺键 ⇒ `None`（落 NULL = 标准价语义，见
/// quality_inspection_service.rs:576 `PriceLevel.is_null()` 认标准价，不得把该语义
/// 改成空串）；合法值 trim 后原样返回；越界值 ⇒ 400 `VALIDATION_ERROR` 并回显允许值
/// （范式照 handlers/inventory_stock_handler.rs:62-74 与销售侧 status 筛选校验）。
fn validate_price_level_input(raw: Option<String>) -> Result<Option<String>, AppError> {
    let Some(value) = raw.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    if SALES_PRICE_LEVEL_ALLOWED.contains(&value.as_str()) {
        return Ok(Some(value));
    }
    Err(AppError::validation_displayable(format!(
        "价格等级 {value} 不是合法取值，允许值：{}",
        SALES_PRICE_LEVEL_ALLOWED.join("/")
    )))
}

/// 销售价目列表读模型：实体全列 + LEFT JOIN 关联出的产品名/产品编码/客户名/客户编码
/// （实体仅有外键 ID，列表与导出的消费方需要人类可读标识）。
///
/// JOIN 名列均 `Option<String>`（两表无 FK，孤儿行名称如实为 NULL，禁止造名兜底）；
/// 实体自身列按 `sales_price` 约束保持原类型。范式照 `purchase_price_service.rs:19-40`。
#[derive(Debug, Clone, Serialize, FromQueryResult)]
pub struct SalesPriceView {
    pub id: i32,
    pub product_id: i32,
    pub customer_id: Option<i32>,
    pub customer_type: Option<String>,
    pub price: Decimal,
    pub currency: String,
    pub unit: String,
    pub min_order_qty: Decimal,
    pub price_type: String,
    pub price_level: Option<String>,
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
    pub customer_name: Option<String>,
    pub customer_code: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SalesPriceQueryParams {
    pub product_id: Option<i32>,
    /// 客户 ID 等值筛选（sales_prices.customer_id 真实列，前端筛选栏一直在传，此前后端不收）
    pub customer_id: Option<i32>,
    /// 关键词筛选：语义 =「产品名称/客户名称」模糊匹配（前端筛选栏承诺，见 handler SalesPriceQuery 注释）
    pub keyword: Option<String>,
    pub customer_type: Option<String>,
    pub status: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateSalesPriceInput {
    pub product_id: i32,
    pub customer_id: Option<i32>,
    pub customer_type: Option<String>,
    pub price: Decimal,
    pub currency: Option<String>,
    #[validate(length(min = 1, message = "计量单位不能为空"))]
    pub unit: String,
    #[validate(length(min = 1, message = "价格类型不能为空"))]
    pub price_type: String,
    /// 价格等级：取值域 = `SALES_PRICE_LEVEL_ALLOWED`（词表常量单源）；空串/缺键 ⇒ NULL（标准价）
    pub price_level: Option<String>,
    pub min_order_qty: Option<Decimal>,
    pub effective_date: Option<String>,
    pub expiry_date: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateSalesPriceInput {
    pub product_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub customer_type: Option<String>,
    pub price: Option<Decimal>,
    pub currency: Option<String>,
    /// 价格等级：给合法值 = 改值；空串 ⇒ 视为未填（不改）；越界值 ⇒ 400。
    /// 注：本键为透传通道，「显式 null 清空回标准价」未在本批裁定范围（三态统一见任务板 #169），交回列账。
    pub price_level: Option<String>,
    pub min_order_qty: Option<Decimal>,
    /// 三态（effective_date 为 NOT NULL 列）：键缺席=不改、有值=改值、显式 null=拒绝（不能清空必填列）
    #[serde(default, deserialize_with = "double_option")]
    pub effective_date: Option<Option<String>>,
    /// 三态（可空列统一口径，任务板 #169；照 department_service.rs:220-221 先例）：
    /// 键缺席=保持原值、显式 null=清空为 NULL、有值=改值
    #[serde(default, deserialize_with = "double_option")]
    pub expiry_date: Option<Option<String>>,
}

pub struct SalesPriceService {
    db: Arc<DatabaseConnection>,
}

impl SalesPriceService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 销售价目列表（读模型富化版）。
    ///
    /// `total` 在带筛选的基础查询上统计（无富化 JOIN；keyword 谓词命中时含其所需的
    /// 两条 LEFT JOIN，保证 total 与 items 同源）；两条 LEFT JOIN 均指向对端主键、
    /// 多对一不倍增行 ⇒ 有无富化 JOIN 行数/分页/offset 语义完全一致。
    pub async fn get_prices_list(
        &self,
        params: SalesPriceQueryParams,
    ) -> Result<(Vec<SalesPriceView>, u64), AppError> {
        let mut query = sales_price::Entity::find();

        if let Some(product_id) = params.product_id {
            query = query.filter(sales_price::Column::ProductId.eq(product_id));
        }

        if let Some(customer_id) = params.customer_id {
            query = query.filter(sales_price::Column::CustomerId.eq(customer_id));
        }

        if let Some(customer_type) = params.customer_type {
            query = query.filter(sales_price::Column::CustomerType.eq(customer_type));
        }

        if let Some(status) = &params.status {
            query = query.filter(sales_price::Column::Status.eq(status));
        }

        // keyword：前端筛选栏承诺「产品名称/客户名称」模糊匹配（SalesPriceFilter.vue），
        // 谓词依赖 products/customers 两表，命中时在本层挂两条 LeftJoin（多对一，不倍增行）。
        let keyword = params
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        if let Some(kw) = keyword {
            query = query
                .join(JoinType::LeftJoin, sales_price::Relation::Product.def())
                .join(JoinType::LeftJoin, sales_price::Relation::Customer.def())
                .filter(
                    Condition::any()
                        .add(product::Column::Name.contains(kw))
                        .add(customer::Column::CustomerName.contains(kw)),
                );
        }

        let total = query.clone().count(&*self.db).await?;

        // 读模型富化（范式照 purchase_price_service.rs:104-114）：名称/编码列取自既有
        // products/customers 两表，孤儿行（无 FK 约束）名称如实为 NULL。keyword 分支已挂过
        // 同两条 JOIN 时不再重复挂载，全查询恒定一条关系一次 JOIN。
        let mut enriched = query;
        if keyword.is_none() {
            enriched = enriched
                .join(JoinType::LeftJoin, sales_price::Relation::Product.def())
                .join(JoinType::LeftJoin, sales_price::Relation::Customer.def());
        }
        let prices = enriched
            .column_as(product::Column::Name, "product_name")
            .column_as(product::Column::Code, "product_code")
            .column_as(customer::Column::CustomerName, "customer_name")
            .column_as(customer::Column::CustomerCode, "customer_code")
            .order_by(sales_price::Column::Id, Order::Desc)
            // 批次 98 P2-A 修复（v5 复审）：page clamp 防 DoS
            .offset((params.page.clamp(1, 1000).saturating_sub(1) * params.page_size) as u64)
            .limit(params.page_size as u64)
            .into_model::<SalesPriceView>()
            .all(&*self.db)
            .await?;

        Ok((prices, total))
    }

    pub async fn create_price(
        &self,
        req: CreateSalesPriceInput,
        user_id: i32,
    ) -> Result<sales_price::Model, AppError> {
        info!(
            "用户 {} 正在创建销售价格，产品 ID: {}",
            user_id, req.product_id
        );

        let active_price = sales_price::ActiveModel {
            product_id: Set(req.product_id),
            customer_id: Set(req.customer_id),
            customer_type: Set(req.customer_type),
            price: Set(req.price),
            currency: Set(req
                .currency
                .unwrap_or_else(|| crate::constants::DEFAULT_CURRENCY.to_string())),
            unit: Set(req.unit),
            price_type: Set(req.price_type),
            // 白名单校验后的等级：合法值 Some(v) 落库；空串/缺键 None 落 NULL——
            // NULL 是"标准价"的既有语义载体（quality_inspection_service.rs:576 按
            // price_level IS NULL 认标准价），不可写空串顶替。
            price_level: Set(validate_price_level_input(req.price_level)?),
            min_order_qty: Set(req.min_order_qty.unwrap_or_default()),
            effective_date: Set(req
                .effective_date
                .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string())
                .parse()
                .map_err(|e| AppError::validation(format!("日期格式错误：{}", e)))?),
            // 到期日与生效日同口径严格解析：此前 `.and_then(|d| d.parse().ok())` 把
            // 非法日期串静默吞成 NULL（不报错、不落值），属"不静默"红线缺陷，改 fail-visible 400。
            expiry_date: Set(req
                .expiry_date
                .map(|d| {
                    d.parse()
                        .map_err(|e| AppError::validation(format!("日期格式错误：{}", e)))
                })
                .transpose()?),
            status: Set(price_approval::PENDING.to_string()),
            created_by: Set(Some(user_id)),
            ..Default::default()
        };

        let price = active_price.insert(&*self.db).await?;
        info!("销售价格创建成功，ID: {}", price.id);
        Ok(price)
    }

    pub async fn get_price(&self, id: i32) -> Result<sales_price::Model, AppError> {
        info!("查询销售价格，ID: {}", id);

        let price = sales_price::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售价格 {} 未找到", id)))?;

        Ok(price)
    }

    pub async fn approve_price(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        info!("用户 {} 正在批准销售价格，ID: {}", user_id, id);

        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        let txn = (*self.db).begin().await?;

        let price_model = sales_price::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售价格 {} 未找到", id)))?;

        // 检查状态，只有待审批状态可以批准
        if price_model.status != price_approval::PENDING {
            // 状态门：价格当前非待审批，前置状态未满足，归业务族；文案含状态 token 保持脱敏
            return Err(AppError::business(format!(
                "只有待审批状态的价格可以批准，当前状态：{}",
                price_model.status
            )));
        }

        let mut price: sales_price::ActiveModel = price_model.into();
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

        info!("销售价格批准成功，ID: {}", id);
        Ok(())
    }

    pub async fn get_price_history(
        &self,
        product_id: i32,
    ) -> Result<Vec<sales_price::Model>, AppError> {
        info!("查询产品 {} 的价格历史", product_id);

        let history = sales_price::Entity::find()
            .filter(sales_price::Column::ProductId.eq(product_id))
            .order_by(sales_price::Column::EffectiveDate, Order::Desc)
            .all(&*self.db)
            .await?;

        Ok(history)
    }

    /// 更新销售价格
    pub async fn update_price(
        &self,
        id: i32,
        req: UpdateSalesPriceInput,
    ) -> Result<sales_price::Model, AppError> {
        info!("更新销售价格，ID: {}", id);

        // NOT NULL 列清空拒绝门控在任何 DB 访问之前（照 department_service.rs:228-250
        // 先例）：double_option 之后「显式 null」不再塌成「键缺席」，必须 fail-visible
        // 拒绝，而不是静默保持原值。
        if matches!(req.effective_date, Some(None)) {
            return Err(AppError::business_displayable(
                "生效日期不能清空：该字段为必填项",
            ));
        }
        let price_level = validate_price_level_input(req.price_level)?;

        let price_model = sales_price::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售价格 {} 未找到", id)))?;

        let mut active: sales_price::ActiveModel = price_model.into_active_model();

        if let Some(product_id) = req.product_id {
            active.product_id = Set(product_id);
        }
        if let Some(customer_id) = req.customer_id {
            active.customer_id = Set(Some(customer_id));
        }
        if let Some(customer_type) = req.customer_type {
            active.customer_type = Set(Some(customer_type));
        }
        if let Some(price) = req.price {
            active.price = Set(price);
        }
        if let Some(currency) = req.currency {
            active.currency = Set(currency);
        }
        if let Some(min_order_qty) = req.min_order_qty {
            active.min_order_qty = Set(min_order_qty);
        }
        // 等级透传：给合法值才 Set；缺键/空串（校验归一为 None）= 保持原值不动。
        if let Some(level) = price_level {
            active.price_level = Set(Some(level));
        }
        if let Some(effective_date) = req.effective_date.flatten() {
            active.effective_date = Set(effective_date
                .parse()
                .map_err(|e| AppError::validation(format!("日期格式错误：{}", e)))?);
        }
        // expiry_date 三态（可空列统一清空口径，任务板 #169；写法照 department_service.rs:258-261
        // 的三态注释）：None=不 Set（列保持原值）/ Some(None)=Set(None) 清空为 NULL /
        // Some(Some(d))=严格解析后覆盖，非法日期 400，绝不静默。
        match req.expiry_date {
            None => {}
            Some(None) => active.expiry_date = Set(None),
            Some(Some(expiry_date)) => {
                active.expiry_date =
                    Set(Some(expiry_date.parse().map_err(|e| {
                        AppError::validation(format!("日期格式错误：{}", e))
                    })?));
            }
        }

        let updated = active.update(&*self.db).await?;
        info!("销售价格更新成功，ID: {}", updated.id);
        Ok(updated)
    }

    /// 删除销售价格（批次 94 P2-10：补 user_id 参数，将 Some(0) 占位符改为真实操作人 user_id，；保证审计日志能追溯实际删除人。）
    pub async fn delete_price(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        info!("删除销售价格，ID: {}，操作人: {}", id, user_id);
        // P0 8-3 修复：delete 操作补审计日志
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            sales_price::Entity,
            _,
        >(&*self.db, "sales_price", id, Some(user_id))
        .await?;
        info!("销售价格删除成功，ID: {}", id);
        Ok(())
    }
}
