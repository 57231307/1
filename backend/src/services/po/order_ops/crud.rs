//! 采购订单-CRUD 子模块（order_ops/crud）
//!
//! 拆分自原 `po/order.rs` 的 `impl PurchaseOrderService` 块。
//! 包含创建 / 更新 / 删除 / 列表 / 详情方法及其私有 helper：
//! - `generate_order_no_with_txn`（宏生成，`pub`）——「调用方自行插入」路径的取号入口
//!   （price 子模块缺料转采购跨 impl 块调用）；`create_order_header` 走生成器的
//!   `insert_with_no_retry`（事务内取号 + 23505 保存点重试）。两者前缀/位数一致
//! - `create_order` / `validate_order_request` / `create_order_header` /
//!   `validate_products_exist_txn` / `calculate_item_amounts` /
//!   `build_order_item_active_model` / `create_order_items`（创建链路 + 私有 helper）
//! - `update_order` / `validate_order_modification` / `apply_order_field_updates`（更新链路）
//! - `delete_order`（删除）
//! - `list_orders` / `get_order`（列表 / 详情查询，含行级数据权限）
//!
//! 业务规则：
//! - 创建订单含明细金额计算（round_dp(2) 精度归一化）与预算检查占用
//! - 更新 / 删除仅限草稿（DRAFT）状态，更新另允许 REJECTED；仅创建人可操作
//! - 列表 / 详情接入 V15 P0-S01 行级数据权限过滤与 IDOR 校验

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::sea_query::{Expr, Func, Query, SelectStatement, SubQueryStatement};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, ExprTrait, JoinType, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, RelationTrait, Set, TransactionTrait,
};

use crate::models::{
    department, product, product_color, purchase_order, purchase_order_item, purchase_receipt,
    status, supplier, user, warehouse,
};
use crate::services::po::order::{PurchaseOrderDto, PurchaseOrderService};
use crate::services::po::{CreatePurchaseOrderRequest, UpdatePurchaseOrderRequest};
use crate::services::sku_mapping_service::SkuMappingService;
use crate::services::supplier_blacklist_service::SupplierBlacklistService;
use crate::services::supplier_qualification_gate::SupplierQualificationGate;
// V15 P0-S01：行级数据权限工具
use crate::utils::data_scope::{
    DataScopeContext, apply_data_scope, check_resource_owner_by_member_scope,
};
use crate::utils::error::AppError;
// 单号取号+插入的唯一入口（事务内取号 + 23505 保存点重试），见 `create_order_header`
use crate::utils::number_generator::DocumentNumberGenerator;
// 批次 260 修复：接入 paginate_with_total 统一分页逻辑
use crate::utils::pagination::paginate_with_total;
use crate::utils::sql_escape::safe_like_pattern;

/// 单行明细金额计算结果（`calculate_item_amounts` 权威口径的数据载体；
/// 创建与明细更新两条链路共用，自动补货两条链路按同一公式复用本结构承载单行金额，
/// 集成测试直调本函数并按本结构逐值断言税额口径，故结构与字段公开）
#[derive(Debug)]
pub struct ItemAmounts {
    pub quantity_ordered: Decimal,
    pub unit_price: Decimal,
    pub quantity_alt_ordered: Decimal,
    pub tax_percent: Decimal,
    pub discount_percent: Decimal,
    pub amount: Decimal,
    pub tax_amount: Decimal,
    pub discount_amount: Decimal,
}

impl PurchaseOrderService {
    // 采购订单号取号入口：`PO` + `YYYYMMDD` + 3 位当日流水（PO20260315001）。
    //
    // 必须是「写入事务内」变体（宏体走 `DocumentNumberGenerator::generate_no_with_width_txn`，
    // 见 `utils/crud_macro.rs:37`）：`pg_advisory_xact_lock(prefix+date)` 持锁到调用方事务提交，
    // 并在同一持锁事务内探测候选号是否已被占用，从而把并发的「取号 → 插入」整体串行化。
    // 若改用 `generate_no`（`utils/number_generator.rs::generate_no_with_width` 自开计数事务），
    // 锁随该内层事务提交即释放，返回号只保证「提交那一刻未被占用」，调用方 INSERT 前存在并发窗口，
    // 会撞 `purchase_orders.order_no` UNIQUE
    // （`migration/src/domain/system/m0001_initial_schema.rs:453`）表现为 500。
    // 域内所有建单路径统一走生成器：本模块 `create_order_header` 走
    // `DocumentNumberGenerator::insert_with_no_retry`（内部即调用本事务内取号，
    // 并对 INSERT 撞 23505 做保存点重试），`po/price.rs:101`/`:332` 缺料转采购走本函数取号；
    // 不得再各写一套前缀/位数。
    crate::impl_generate_no!(
        generate_order_no_with_txn,
        "PO",
        purchase_order::Entity,
        purchase_order::Column::OrderNo,
        txn
    );

    /// 创建采购订单（含明细）
    pub async fn create_order(
        &self,
        req: CreatePurchaseOrderRequest,
        user_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 验证请求参数
        let (warehouse_id, department_id) =
            self.validate_order_request(&req, user_id, &txn).await?;

        // 2. 创建订单主表
        let order = self
            .create_order_header(&req, warehouse_id, department_id, user_id, &txn)
            .await?;

        // 3. 创建订单明细并计算总金额
        let (total_amount, total_quantity, total_quantity_alt) =
            self.create_order_items(&req, order.id, &txn).await?;

        // 4. 更新订单总金额和数量
        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.total_amount = Set(total_amount);
        order_active.total_quantity = Set(total_quantity);
        order_active.total_quantity_alt = Set(total_quantity_alt);
        order_active.updated_at = Set(chrono::Utc::now());
        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            order_active,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        // 5. 预算检查与占用（V15 P0-B06 强制拦截：预算不足或无方案时阻断订单创建）
        // 注意：此处使用 ? 传播错误，事务会自动回滚（Drop 时 rollback）
        // 避免订单已创建但未关联预算的不一致状态
        self.check_and_occupy_budget(&order, department_id, total_amount, user_id)
            .await?;

        // 6. 提交事务
        txn.commit().await?;

        Ok(order)
    }

    /// 验证采购订单请求参数
    async fn validate_order_request(
        &self,
        req: &CreatePurchaseOrderRequest,
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(i32, i32), AppError> {
        // 检查供应商是否存在
        let supplier_exists = supplier::Entity::find_by_id(req.supplier_id)
            .one(txn)
            .await?;
        if supplier_exists.is_none() {
            tracing::error!("供应商 ID {} 不存在", req.supplier_id);
            return Err(AppError::bad_request(format!(
                "供应商 ID {} 不存在",
                req.supplier_id
            )));
        }

        // 采购门控：校验供应商是否在有效黑名单中（事务内执行，消除 TOCTOU）
        SupplierBlacklistService::new(self.db.clone())
            .check_supplier_not_blacklisted(txn, req.supplier_id)
            .await?;

        // 采购门控：供应商资质过期分级门控（黑名单门控同一落点、同一事务内读资质行，消除 TOCTOU）。
        // 法定许可类过期→阻断新建单；一般类过期→存量不阻断仅出预警、新供应商首单阻断；
        // 例外放行需 supplier_qualification:waive 权限并携带原因，同事务落审计。
        // 拒绝文案 business_displayable 外显（business 会被出参脱敏为用户看不到的「业务处理失败」）。
        SupplierQualificationGate::new(self.db.clone())
            .check_purchase_order_gate(
                txn,
                req.supplier_id,
                user_id,
                req.qualification_waiver_reason.as_deref(),
            )
            .await?;

        // 检查仓库是否存在
        let warehouse_id = req
            .warehouse_id
            .ok_or_else(|| AppError::bad_request("仓库 ID 不能为空"))?;
        let warehouse_exists = warehouse::Entity::find_by_id(warehouse_id).one(txn).await?;
        if warehouse_exists.is_none() {
            tracing::error!("仓库 ID {} 不存在", warehouse_id);
            return Err(AppError::bad_request(format!(
                "仓库 ID {} 不存在",
                warehouse_id
            )));
        }

        // 检查部门是否存在
        let department_id = req
            .department_id
            .ok_or_else(|| AppError::bad_request("部门 ID 不能为空"))?;
        let department_exists = department::Entity::find_by_id(department_id)
            .one(txn)
            .await?;
        if department_exists.is_none() {
            tracing::error!("Transaction rolled back: 部门 ID {} 不存在", department_id);
            return Err(AppError::bad_request(format!(
                "部门 ID {} 不存在",
                department_id
            )));
        }

        // 日期合理性检查
        if let Some(expected_date) = req.expected_delivery_date {
            if expected_date < req.order_date {
                tracing::error!("预计交货日期不能早于订单日期");
                return Err(AppError::bad_request(
                    "预计交货日期不能早于订单日期".to_string(),
                ));
            }
        }

        Ok((warehouse_id, department_id))
    }

    /// 创建采购订单主表：单号统一由生成器产生（`PO` + `YYYYMMDD` + 3 位当日流水）。
    ///
    /// 取号在本事务内完成（`pg_advisory_xact_lock(prefix+date)` 持锁到 create_order
    /// 提交，并在同一持锁事务内探测候选号空闲），并进一步用
    /// `DocumentNumberGenerator::insert_with_no_retry` 把 INSERT 包进保存点：
    /// 撞 `purchase_orders.order_no` UNIQUE
    /// （`migration/src/domain/system/m0001_initial_schema.rs:453`，来源是旁路写入或
    /// `handlers/purchase_order_handler.rs:641` 事务外预生成号）时自动回滚本次尝试、
    /// 重新取号重试，而不是裸 500；其余 SQL 错误显式上抛，不吞、不改写。
    /// 禁止时间戳/随机数拼号（绕过 order_no UNIQUE 语义、制造脏数据）。
    async fn create_order_header(
        &self,
        req: &CreatePurchaseOrderRequest,
        warehouse_id: i32,
        department_id: i32,
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<purchase_order::Model, AppError> {
        DocumentNumberGenerator::insert_with_no_retry(
            txn,
            "PO",
            purchase_order::Entity,
            purchase_order::Column::OrderNo,
            |order_no| {
                Self::build_order_header_active_model(
                    req,
                    warehouse_id,
                    department_id,
                    user_id,
                    order_no,
                )
            },
        )
        .await
    }

    /// 构建采购订单主表 ActiveModel（单号由生成器提供）
    fn build_order_header_active_model(
        req: &CreatePurchaseOrderRequest,
        warehouse_id: i32,
        department_id: i32,
        user_id: i32,
        order_no: String,
    ) -> purchase_order::ActiveModel {
        purchase_order::ActiveModel {
            order_no: Set(order_no),
            supplier_id: Set(req.supplier_id),
            order_date: Set(req.order_date),
            expected_delivery_date: Set(req.expected_delivery_date),
            warehouse_id: Set(warehouse_id),
            department_id: Set(department_id),
            purchaser_id: Set(user_id),
            currency: Set(req
                .currency
                .clone()
                .unwrap_or_else(|| crate::constants::DEFAULT_CURRENCY.to_string())),
            exchange_rate: Set(req.exchange_rate.unwrap_or(Decimal::new(1, 0))),
            order_status: Set(status::purchase_order::DRAFT.to_string()),
            payment_terms: Set(req.payment_terms.clone()),
            shipping_terms: Set(req.shipping_terms.clone()),
            notes: Set(req.notes.clone()),
            attachment_urls: Set(req.attachment_urls.clone()),
            // 转采来源真正落库：此前该值只存在于请求体、仅用于触发 SKU 对照翻译，
            // 库里查不到「这张采购单由哪张销售订单转出」。None = 普通采购单，
            // 如实落 NULL（不代填、不猜测来源）。
            source_sales_order_id: Set(req.source_sales_order_id),
            created_by: Set(user_id),
            // 显式初始化金额/数量字段：迁移 ADD COLUMN 未声明 DEFAULT，
            // 依赖 ..Default::default() 的 NotSet 会导致 SeaORM insert 报
            // "Missing value for column 'total_amount_foreign'"，故显式赋 0
            total_amount: Set(Decimal::ZERO),
            total_amount_foreign: Set(Decimal::ZERO),
            total_quantity: Set(Decimal::ZERO),
            total_quantity_alt: Set(Decimal::ZERO),
            ..Default::default()
        }
    }

    /// 批量校验订单明细引用的产品是否存在
    async fn validate_products_exist_txn(
        txn: &sea_orm::DatabaseTransaction,
        items: &[crate::services::po::CreateOrderItemRequest],
    ) -> Result<(), AppError> {
        let mut product_ids = std::collections::HashSet::new();
        for item in items {
            if let Some(material_id) = item.material_id {
                if material_id != 0 {
                    product_ids.insert(material_id);
                }
            }
        }
        if product_ids.is_empty() {
            return Ok(());
        }
        let existing_products = product::Entity::find()
            .filter(product::Column::Id.is_in(product_ids.iter().cloned().collect::<Vec<_>>()))
            .all(txn)
            .await?;
        let existing_product_ids: std::collections::HashSet<i32> =
            existing_products.into_iter().map(|p| p.id).collect();
        for material_id in &product_ids {
            if !existing_product_ids.contains(material_id) {
                tracing::error!("Transaction rolled back: 物料 ID {} 不存在", material_id);
                return Err(AppError::bad_request(format!(
                    "物料 ID {} 不存在",
                    material_id
                )));
            }
        }
        Ok(())
    }

    /// 计算单行明细金额（round_dp(2) 精度归一化）——purchase_order_item 金额口径的
    /// **唯一权威实现**：创建路径（`create_order_items`）与更新明细路径
    /// （`receipt::update_order_item` 重算 subtotal/discount_amount/tax_amount/
    /// total_amount 派生列）共用，禁止另写第二套公式。
    ///
    /// 入参 item 是采购订单行请求，税率取 item 的 tax_rate，口径为百分数（13 即 13%），
    /// 与落库列 tax_percent 同单位，税额经 utils tax 内核换算，产出 ItemAmounts 供
    /// build_order_item_active_model 与 receipt 更新路径写入 purchase_order_item 的
    /// subtotal、tax_percent、tax_amount、discount_amount、total_amount 列。
    ///
    /// 税率口径是**分层**的，两条链路各有一条且只有一条合法路径，系统不设任何默认档位。
    /// - 人工建单与人工改行（本函数的调用方）必须提交 tax_rate，缺值一律点名拒绝。
    ///   0 是业务声明值（免税、出口零税率、无进项票），不能当"没录"的缺省，
    ///   否则漏录与免税在进项台账里无法区分。
    /// - 自动补货链路（services/po/price.rs 的缺料预警与库存预警两条建单入口）无人工录入
    ///   环节、也无税率来源，由该侧显式取 utils tax 的无进项税口径常量落账并留 info 日志，
    ///   不经过本函数，也不得在本函数里补一条缺省分支。
    ///
    /// 提交了 tax_rate 则必须落在落库列域 0 到 999.9999 内（上界取自 purchase_order_item
    /// 的 tax_percent 列 DECIMAL(7,4) 口径，与 utils tax 的 TAX_PERCENT_MAX 同源），
    /// 越界在算金额之前点名拒绝，避免把非法入参留到 insert 阶段炸成列溢出。
    pub fn calculate_item_amounts(
        item: &crate::services::po::CreateOrderItemRequest,
    ) -> Result<ItemAmounts, AppError> {
        let quantity_ordered = item.quantity_ordered.unwrap_or(Decimal::ZERO);
        let unit_price = item.unit_price.unwrap_or(Decimal::ZERO);
        let amount = (quantity_ordered * unit_price).round_dp(2);
        let tax_percent = match item.tax_rate {
            Some(percent) => {
                if percent < Decimal::ZERO || percent > crate::utils::tax::TAX_PERCENT_MAX {
                    return Err(AppError::business_displayable(format!(
                        "采购订单行税率 {} 非法：须为百分数口径且落在 0 到 {} 之间（13 表示 13%，0 表示无进项税），系统不设默认税率档位",
                        percent,
                        crate::utils::tax::TAX_PERCENT_MAX
                    )));
                }
                percent
            }
            None => {
                return Err(AppError::business_displayable(
                    "采购订单行缺少税率 tax_rate，须按百分数提交（13 表示 13%，0 表示无进项税），系统不设默认税率档位，请录入税率后重试",
                ));
            }
        };
        let tax_amount = crate::utils::tax::tax_amount(amount, tax_percent);
        let discount_percent = item.discount_percent.unwrap_or(Decimal::ZERO);
        let discount_amount = (amount * discount_percent / Decimal::new(100, 0)).round_dp(2);
        let quantity_alt_ordered = item.quantity_alt_ordered.unwrap_or(Decimal::ZERO);
        Ok(ItemAmounts {
            quantity_ordered,
            unit_price,
            quantity_alt_ordered,
            tax_percent,
            discount_percent,
            amount,
            tax_amount,
            discount_amount,
        })
    }

    /// 构造订单明细 ActiveModel（material_id 缺失拒绝创建）
    fn build_order_item_active_model(
        item: &crate::services::po::CreateOrderItemRequest,
        order_id: i32,
        index: usize,
        amounts: &ItemAmounts,
    ) -> Result<purchase_order_item::ActiveModel, AppError> {
        let material_id = item.material_id.ok_or_else(|| {
            AppError::validation_displayable(format!("订单行 {} 缺少物料ID", index + 1))
        })?;
        Ok(purchase_order_item::ActiveModel {
            id: Default::default(),
            order_id: Set(order_id),
            line_no: Set(item.line_no.unwrap_or((index + 1) as i32)),
            product_id: Set(material_id),
            quantity: Set(amounts.quantity_ordered),
            quantity_alt: Set(amounts.quantity_alt_ordered),
            unit_price: Set(amounts.unit_price),
            unit_price_foreign: Set(amounts.unit_price),
            discount_percent: Set(amounts.discount_percent),
            tax_percent: Set(amounts.tax_percent),
            subtotal: Set(amounts.amount),
            tax_amount: Set(amounts.tax_amount),
            discount_amount: Set(amounts.discount_amount),
            total_amount: Set(amounts.amount + amounts.tax_amount - amounts.discount_amount),
            received_quantity: Set(Decimal::ZERO),
            received_quantity_alt: Set(Decimal::ZERO),
            quantity_tolerance_pct: Set(item.quantity_tolerance_pct),
            notes: Set(item.notes.clone()),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            // v14 批次 417：面料行业追溯字段，使用 NotSet 让 DB 默认值处理
            color_code: Set(item.color_no.clone()),
            lot_no: sea_orm::ActiveValue::NotSet,
            batch_no: sea_orm::ActiveValue::NotSet,
            // 转采购快照列：由 create_order_items 在 resolve 后设置
            supplier_product_code: sea_orm::ActiveValue::NotSet,
            supplier_color_no: sea_orm::ActiveValue::NotSet,
        })
    }

    /// 色号反查 + 供应商 SKU 对照（保密映射）解析——创建路径（转采购）与明细更新路径
    /// 共用的**唯一权威实现**，禁止在任何一侧复制粘贴出第二套规则：
    /// - `svc = None`：创建的非转采购场景，不反查、不解析对照，返回 `None`（既有口径）。
    /// - `svc = Some`：`color_no` 非空时先按 (product_id, color_no) 反查 `product_colors.id`
    ///   （查不到 → `business` 拒绝，文案含内部 product_id，保持脱敏）；再经
    ///   `SkuMappingService::resolve_supplier_sku` 解析供应商侧快照，解析不到 →
    ///   `business_displayable`「第 N 行无该色号」——措辞中性可外显：只回显用户自己
    ///   提交的行号，不泄露该产品是调货还是自制，也不外显对照内容。
    ///
    /// 返回值即 `supplier_product_code`/`supplier_color_no` 保密快照列的唯一来源，
    /// 任何写侧请求 DTO 都不得直接携带这两列。
    pub(crate) async fn resolve_supplier_sku_snapshot(
        txn: &sea_orm::DatabaseTransaction,
        svc: Option<&SkuMappingService>,
        product_id: i32,
        color_no: &Option<String>,
        supplier_id: i32,
        line_no: usize,
    ) -> Result<Option<crate::services::sku_mapping_service::ResolvedSku>, AppError> {
        let Some(svc) = svc else {
            return Ok(None);
        };
        // 按 color_no 反查 product_colors.id
        let product_color_id = match color_no {
            Some(cn) if !cn.is_empty() => {
                let pc = product_color::Entity::find()
                    .filter(product_color::Column::ProductId.eq(product_id))
                    .filter(product_color::Column::ColorNo.eq(cn))
                    .one(txn)
                    .await?
                    .ok_or_else(|| {
                        AppError::business(format!(
                            "第 {} 行色号「{}」在产品 {} 下不存在，无法转采购",
                            line_no, cn, product_id
                        ))
                    })?;
                Some(pc.id)
            }
            _ => None,
        };

        let resolved = svc
            .resolve_supplier_sku(product_id, product_color_id, supplier_id)
            .await?
            .ok_or_else(|| {
                // 措辞须中性且可外显：不得泄露该产品是调货还是自制，
                // 但要把「无该色号」如实回给用户（用 displayable，非脱敏 business）。
                AppError::business_displayable(format!("第 {} 行无该色号，无法转采购", line_no))
            })?;

        Ok(Some(resolved))
    }

    /// 创建采购订单明细
    async fn create_order_items(
        &self,
        req: &CreatePurchaseOrderRequest,
        order_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(Decimal, Decimal, Decimal), AppError> {
        let mut total_amount = Decimal::new(0, 0);
        let mut total_quantity = Decimal::new(0, 0);
        let mut total_quantity_alt = Decimal::new(0, 0);

        let items = req.items.clone().unwrap_or_default();
        Self::validate_products_exist_txn(txn, &items).await?;

        // 转采购翻译：仅当请求携带来源销售订单标识时触发
        let sku_service = if req.source_sales_order_id.is_some() {
            Some(SkuMappingService::new(self.db.clone()))
        } else {
            None
        };

        for (index, item) in items.iter().enumerate() {
            let mut amounts = Self::calculate_item_amounts(item)?;

            let resolved = Self::resolve_supplier_sku_snapshot(
                txn,
                sku_service.as_ref(),
                item.material_id.unwrap_or(0),
                &item.color_no,
                req.supplier_id,
                index + 1,
            )
            .await?;

            // 若请求未显式指定单价，用 supplier_price 作为默认
            if let Some(resolved) = &resolved {
                if item.unit_price.is_none() || item.unit_price == Some(Decimal::ZERO) {
                    if let Some(price) = resolved.supplier_price {
                        amounts.unit_price = price;
                        amounts.amount = (amounts.quantity_ordered * price).round_dp(2);
                        // 税额走 utils tax 内核，与本函数开头的行税额同一公式，
                        // 禁止在此再写一套除 100
                        amounts.tax_amount =
                            crate::utils::tax::tax_amount(amounts.amount, amounts.tax_percent);
                        amounts.discount_amount = (amounts.amount * amounts.discount_percent
                            / Decimal::new(100, 0))
                        .round_dp(2);
                    }
                }
            }

            let mut order_item =
                Self::build_order_item_active_model(item, order_id, index, &amounts)?;

            // 设置转采购快照列（唯一来源：resolve_supplier_sku_snapshot 权威解析）
            if let Some(resolved) = &resolved {
                let resolved_product_code = Some(resolved.supplier_product_code.clone());
                let resolved_color_no = resolved.supplier_color_no.clone();
                order_item.supplier_product_code = Set(resolved_product_code);
                order_item.supplier_color_no = Set(resolved_color_no);
            }

            order_item.insert(txn).await?;

            total_amount += amounts.amount + amounts.tax_amount - amounts.discount_amount;
            total_quantity += amounts.quantity_ordered;
            total_quantity_alt += amounts.quantity_alt_ordered;
        }

        Ok((total_amount, total_quantity, total_quantity_alt))
    }

    /// 更新采购订单（仅草稿状态）
    pub async fn update_order(
        &self,
        order_id: i32,
        req: UpdatePurchaseOrderRequest,
        user_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        let txn = (*self.db).begin().await?;
        let order = purchase_order::Entity::find_by_id(order_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", order_id)))?;
        Self::validate_order_modification(&order, user_id)?;
        let mut order_active: purchase_order::ActiveModel = order.into();
        Self::apply_order_field_updates(&mut order_active, &req, user_id);
        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            order_active,
            Some(user_id),
        )
        .await?;
        txn.commit().await?;
        Ok(order)
    }

    /// 校验订单是否可修改（状态+权限）
    fn validate_order_modification(
        order: &purchase_order::Model,
        user_id: i32,
    ) -> Result<(), AppError> {
        if order.order_status != status::purchase_order::DRAFT
            && order.order_status != status::purchase_order::REJECTED
        {
            return Err(AppError::business(format!(
                "订单状态不允许修改，当前状态：{}",
                order.order_status
            )));
        }
        if order.created_by != user_id {
            return Err(AppError::permission_denied(
                "只能修改自己创建的订单".to_string(),
            ));
        }
        Ok(())
    }

    /// 将请求字段应用到 ActiveModel
    fn apply_order_field_updates(
        order_active: &mut purchase_order::ActiveModel,
        req: &UpdatePurchaseOrderRequest,
        user_id: i32,
    ) {
        if let Some(supplier_id) = req.supplier_id {
            order_active.supplier_id = Set(supplier_id);
        }
        if let Some(order_date) = req.order_date {
            order_active.order_date = Set(order_date);
        }
        if let Some(expected_delivery_date) = req.expected_delivery_date {
            order_active.expected_delivery_date = Set(Some(expected_delivery_date));
        }
        if let Some(warehouse_id) = req.warehouse_id {
            order_active.warehouse_id = Set(warehouse_id);
        }
        if let Some(department_id) = req.department_id {
            order_active.department_id = Set(department_id);
        }
        if let Some(currency) = &req.currency {
            order_active.currency = Set(currency.clone());
        }
        if let Some(exchange_rate) = req.exchange_rate {
            order_active.exchange_rate = Set(exchange_rate);
        }
        if let Some(payment_terms) = &req.payment_terms {
            order_active.payment_terms = Set(Some(payment_terms.clone()));
        }
        if let Some(shipping_terms) = &req.shipping_terms {
            order_active.shipping_terms = Set(Some(shipping_terms.clone()));
        }
        if let Some(notes) = &req.notes {
            order_active.notes = Set(Some(notes.clone()));
        }
        if let Some(attachment_urls) = &req.attachment_urls {
            order_active.attachment_urls = Set(Some(attachment_urls.clone()));
        }
        order_active.updated_by = Set(Some(user_id));
        order_active.updated_at = Set(Utc::now());
    }

    /// 删除采购订单（仅草稿状态）
    pub async fn delete_order(&self, order_id: i32, user_id: i32) -> Result<(), AppError> {
        // 1. 查询订单
        let order = purchase_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态
        if order.order_status != status::purchase_order::DRAFT {
            return Err(AppError::business(format!(
                "订单状态不允许删除，当前状态：{}",
                order.order_status
            )));
        }

        // 3. 检查权限
        if order.created_by != user_id {
            return Err(AppError::permission_denied(
                "只能删除自己创建的订单".to_string(),
            ));
        }

        // 4. 删除订单（级联删除明细）（P0 8-3 修复：补审计日志）
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            purchase_order::Entity,
            _,
        >(&*self.db, "purchase_order", order_id, Some(user_id))
        .await
    }

    /// 已入库金额关联标量子查询：`SUM(purchase_receipt.total_amount) WHERE order_id = 当前订单.id`。
    ///
    /// 采用标量子查询而非 LEFT JOIN + GROUP BY，避免一对多 JOIN 造成的行倍增，
    /// 从而保持列表 `paginate` 的行数与 `total` 不变，仍是单次查询。
    fn received_amount_subquery() -> SelectStatement {
        Query::select()
            .expr(Func::sum(Expr::col((
                purchase_receipt::Entity,
                purchase_receipt::Column::TotalAmount,
            ))))
            .from(purchase_receipt::Entity)
            .and_where(
                Expr::col((purchase_receipt::Entity, purchase_receipt::Column::OrderId))
                    .equals((purchase_order::Entity, purchase_order::Column::Id)),
            )
            .to_owned()
    }

    /// 列表与导出共用的唯一筛选条件构造器（行级数据权限、状态、供应商与关键字）。
    ///
    /// 两条取数腿必须走本函数，各自再写一遍条件时，任一侧漏一个筛选项就会出现
    /// 界面与导出取的是不同行集而无人报错。返回未排序的查询，排序与取数形态由调用方
    /// 按自身语义施加，列表加分页与排序，导出取全量。
    /// 行级数据权限在此施加，与列表同源，导出腿当前传 None，该面单独登记。
    pub(crate) fn build_orders_query(
        status: Option<String>,
        supplier_id: Option<i32>,
        keyword: Option<String>,
        data_scope: Option<&DataScopeContext>,
    ) -> sea_orm::Select<purchase_order::Entity> {
        let mut query = purchase_order::Entity::find()
            .column_as(supplier::Column::SupplierName, "supplier_name")
            .column_as(warehouse::Column::Name, "warehouse_name")
            .column_as(department::Column::Name, "department_name")
            .column_as(user::Column::RealName, "creator_name")
            .column_as(
                Expr::from(SubQueryStatement::from(Self::received_amount_subquery())),
                "received_amount",
            )
            .join(JoinType::LeftJoin, purchase_order::Relation::Supplier.def())
            .join(
                JoinType::LeftJoin,
                purchase_order::Relation::Warehouse.def(),
            )
            .join(
                JoinType::LeftJoin,
                purchase_order::Relation::Department.def(),
            )
            .join(JoinType::LeftJoin, purchase_order::Relation::Creator.def());

        // 行级数据权限过滤（purchase_order 表有 created_by + department_id，支持完整 Dept）
        if let Some(ctx) = data_scope {
            query = apply_data_scope(
                query,
                ctx,
                purchase_order::Column::CreatedBy,
                purchase_order::Column::DepartmentId,
            );
        }

        // 添加筛选条件
        if let Some(status) = status {
            query = query.filter(purchase_order::Column::OrderStatus.eq(status));
        }
        if let Some(supplier_id) = supplier_id {
            query = query.filter(purchase_order::Column::SupplierId.eq(supplier_id));
        }
        // 关键字：列表页搜索框承诺匹配「订单号/供应商名」，两侧都要真正参与过滤
        if let Some(kw) = keyword.as_deref().filter(|s| !s.is_empty()) {
            let pattern = safe_like_pattern(kw);
            query = query.filter(
                purchase_order::Column::OrderNo
                    .like(&pattern)
                    .or(supplier::Column::SupplierName.like(&pattern)),
            );
        }

        query
    }

    /// 获取订单列表（分页）
    pub async fn list_orders(
        &self,
        page: u64,
        page_size: u64,
        status: Option<String>,
        supplier_id: Option<i32>,
        keyword: Option<String>,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<(Vec<PurchaseOrderDto>, u64), AppError> {
        let paginator = Self::build_orders_query(status, supplier_id, keyword, data_scope)
            .order_by(purchase_order::Column::CreatedAt, sea_orm::Order::Desc)
            .into_model::<PurchaseOrderDto>()
            .paginate(&*self.db, page_size);

        // paginate_with_total 内部已处理 saturating_sub(1) 偏移
        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;
        Ok((items, total))
    }

    /// 获取订单详情
    pub async fn get_order(
        &self,
        order_id: i32,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<PurchaseOrderDto, AppError> {
        let order = purchase_order::Entity::find_by_id(order_id)
            .column_as(supplier::Column::SupplierName, "supplier_name")
            .column_as(warehouse::Column::Name, "warehouse_name")
            .column_as(department::Column::Name, "department_name")
            .column_as(user::Column::RealName, "creator_name")
            .column_as(
                Expr::from(SubQueryStatement::from(Self::received_amount_subquery())),
                "received_amount",
            )
            .join(JoinType::LeftJoin, purchase_order::Relation::Supplier.def())
            .join(
                JoinType::LeftJoin,
                purchase_order::Relation::Warehouse.def(),
            )
            .join(
                JoinType::LeftJoin,
                purchase_order::Relation::Department.def(),
            )
            .join(JoinType::LeftJoin, purchase_order::Relation::Creator.def())
            .into_model::<PurchaseOrderDto>()
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", order_id)))?;

        // 行级归属门（读族）：本表 department_id 由建单请求所选采购部门直填、不由 m_rls_dept_domain
        // 触发器维护，不能当权威部门；列表侧按「归属人∈可见成员集合」下推，故单行同用成员集合
        // 口径，保证列表可见 ⇔ 详情可读。拒绝出参为固定脱敏常量（不含单据 ID）。
        if let Some(ctx) = data_scope {
            if !check_resource_owner_by_member_scope(ctx, Some(order.created_by)) {
                return Err(AppError::permission_denied(
                    "无权访问该采购订单（数据范围限制）",
                ));
            }
        }

        Ok(order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::po::CreateOrderItemRequest;
    use std::str::FromStr;

    /// 构造只关心数量与税率的采购订单行请求，其余字段一律 None（表示前端未提交该字段）。
    /// 入参 tax_rate 原样写入请求的 tax_rate，供 calculate_item_amounts 的提交分支与
    /// 缺值分支各自取用，数量固定 20、单价固定 15.00，因此不含税金额恒为 300.00，
    /// 税额可按手算核对。
    fn item_with_tax_rate(tax_rate: Option<Decimal>) -> CreateOrderItemRequest {
        CreateOrderItemRequest {
            line_no: Some(1),
            material_id: Some(7),
            unit_price: Some(Decimal::from_str("15.00").unwrap()),
            quantity_ordered: Some(Decimal::from(20)),
            quantity_alt_ordered: None,
            tax_rate,
            discount_percent: None,
            quantity_tolerance_pct: None,
            color_no: None,
            notes: None,
        }
    }

    /// 人工建单缺 tax_rate 一律点名拒绝，不按 0 落账，0 是业务声明值（免税、无进项票），
    /// 代填会让漏录与免税在进项台账里无法区分。拒绝必须走 business_displayable
    /// （错误码 BUSINESS_ERROR、出参携带真实文案），且文案不得内插行号/物料 ID
    /// 这类记录标识（本仓出参脱敏族）。
    #[test]
    fn tax_rate_absent_is_rejected_not_defaulted() {
        let item = item_with_tax_rate(None);
        let error = PurchaseOrderService::calculate_item_amounts(&item)
            .expect_err("人工建单缺 tax_rate 必须拒绝计算，不得按任何值代填税率");
        assert_eq!(error.error_code(), "BUSINESS_ERROR");
        let message = error.public_message();
        assert!(
            message.contains("缺少税率 tax_rate") && message.contains("请录入税率后重试"),
            "拒绝文案必须说明该怎么填，实际: {message}"
        );
        assert!(
            !message.contains("line_no") && !message.contains("物料"),
            "拒绝文案不得内插记录标识，实际: {message}"
        );
    }

    /// 提交的 tax_rate 是百分数口径（13 即 13%），税额必须是 300 乘 13 除 100 得 39.00。
    /// 第二样本取 0.26，内核按字面百分数处理，300 乘 0.26 除 100 得 0.78，
    /// 若调用方把它当小数率 26% 传进来，期望应为 78，两者相差 100 倍，
    /// 据此钉死调用方只能传百分数口径。
    #[test]
    fn tax_rate_is_percent_unit_not_fraction() {
        let item = item_with_tax_rate(Some(Decimal::from(13)));
        let amounts = PurchaseOrderService::calculate_item_amounts(&item).unwrap();
        assert_eq!(amounts.tax_percent, Decimal::from(13));
        assert_eq!(amounts.tax_amount, Decimal::from_str("39.00").unwrap());

        let misread_item = item_with_tax_rate(Some(Decimal::from_str("0.26").unwrap()));
        let misread = PurchaseOrderService::calculate_item_amounts(&misread_item).unwrap();
        assert_eq!(misread.tax_amount, Decimal::from_str("0.78").unwrap());
    }

    /// 显式提交的税率越出落库列域（负数，或超过 tax_percent 列 DECIMAL(7,4) 的 999.9999）
    /// 时在算金额之前点名拒绝，错误码 BUSINESS_ERROR，不允许留到 insert 阶段炸成列溢出。
    /// 列域上界本身仍属合法载荷，单独一样本防止边界被判死。
    #[test]
    fn tax_rate_outside_column_domain_is_rejected() {
        let illegal_values = [
            Decimal::from_str("-1").unwrap(),
            Decimal::from_str("1000").unwrap(),
        ];
        for illegal in illegal_values {
            let item = item_with_tax_rate(Some(illegal));
            let error = PurchaseOrderService::calculate_item_amounts(&item).unwrap_err();
            assert_eq!(error.error_code(), "BUSINESS_ERROR");
        }

        let max_item = item_with_tax_rate(Some(crate::utils::tax::TAX_PERCENT_MAX));
        let amounts = PurchaseOrderService::calculate_item_amounts(&max_item).unwrap();
        assert_eq!(amounts.tax_percent, crate::utils::tax::TAX_PERCENT_MAX);
    }
}
