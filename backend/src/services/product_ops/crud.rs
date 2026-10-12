//! 产品 Service CRUD 子模块（product_ops/crud）
//!
//! 批次 D10 拆分：从原 `product_service.rs` 迁移。
//! 包含 `ProductService` 的产品 CRUD 方法：
//! - `generate_product_code`：生成产品编码（DocumentNumberGenerator）
//! - `build_product_keyword_condition`：关键词检索条件（名称/编码/条码，列清单见 `PRODUCT_KEYWORD_COLUMNS`）
//! - `list_products`：分页 + 过滤查询
//! - `get_product`：详情查询（Redis 读穿透 + 写失效）
//! - `create_product`：创建（含面料行业字段，事务提交后同步 ES）
//! - `delete_product`：硬删除（价目引用预检 + 审计日志 + 失效缓存 + 删除 ES 文档）
//! - `update_product`：更新（审计日志 + 失效缓存 + 同步 ES）
//!
//! 跨模块调用：
//! - create/update/delete 调用 `sync::sync_product_to_es`（`pub(crate)`）
//! - `import_export` 子模块跨模块调用 `list_products` / `create_product`（保持 `pub`）

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, NotSet, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};

use crate::models::product::{self, Entity as ProductEntity};
use crate::models::product_category;
use crate::services::product_service::{CreateProductArgs, ProductService, UpdateProductArgs};
use crate::utils::error::AppError;
use crate::utils::messages::err_msg;
use crate::utils::number_generator::DocumentNumberGenerator;
// P0-D03（Batch 488）：Redis 分布式缓存接入（get_product 读穿透 + 写失效）
// V15 P2 B07-P2-6：使用差异化 TTL（PRODUCT_CACHE_TTL_SECS=600s，产品目录低波动率）
use crate::utils::redis_cache::{
    PRODUCT_CACHE_TTL_SECS, cache_key, redis_cache_del, redis_cache_get_json, redis_cache_set_json,
};
use crate::utils::sql_escape::safe_like_pattern;

impl ProductService {
    /// 生成产品编码
    pub async fn generate_product_code(&self) -> Result<String, crate::utils::error::AppError> {
        DocumentNumberGenerator::generate_no(
            &*self.db,
            "PRD",
            product::Entity,
            product::Column::Code,
        )
        .await
    }

    /// 产品关键词检索覆盖的列：名称/编码/条码
    /// 面料行业按条码扫码取数是常规入口，故条码与名称、编码同层参与模糊匹配
    pub const PRODUCT_KEYWORD_COLUMNS: [product::Column; 3] = [
        product::Column::Name,
        product::Column::Code,
        product::Column::Barcode,
    ];

    /// 构建产品关键词检索条件：`PRODUCT_KEYWORD_COLUMNS` 任一列 LIKE 命中即匹配
    /// `pattern` 必须是已按 LIKE 规则转义的模式串（`safe_like_pattern`）
    pub fn build_product_keyword_condition(pattern: &str) -> sea_orm::Condition {
        let mut cond = sea_orm::Condition::any();
        for column in Self::PRODUCT_KEYWORD_COLUMNS {
            cond = cond.add(column.like(pattern));
        }
        cond
    }

    /// 获取产品列表（支持分页和过滤）
    pub async fn list_products(
        &self,
        page: u64,
        page_size: u64,
        category_id: Option<i32>,
        status: Option<String>,
        search: Option<String>,
    ) -> Result<(Vec<product::Model>, u64), AppError> {
        // 分页参数规范化：page 从 1 开始，page_size 上限 10000（前端 API 外层已 clamp(1,100)，
        // 导出场景需更大上限以支持全量拉取）
        let page = std::cmp::Ord::max(page, 1);
        let page_size = page_size.clamp(1, 10000);

        let mut query = ProductEntity::find();

        // 应用过滤条件
        if let Some(cat_id) = category_id {
            query = query.filter(product::Column::CategoryId.eq(cat_id));
        }

        if let Some(s) = status {
            query = query.filter(product::Column::Status.eq(s));
        }

        if let Some(keyword) = search {
            let pattern = safe_like_pattern(&keyword);
            query = query.filter(Self::build_product_keyword_condition(&pattern));
        }

        // 获取总数
        let total = match query.clone().count(&*self.db).await {
            Ok(count) => {
                tracing::info!("产品查询总数: {}", count);
                count
            }
            Err(e) => {
                tracing::error!("查询产品总数失败: {:?}", e);
                return Err(AppError::from(e));
            }
        };

        // 应用分页和排序
        let offset = (page - 1) * page_size;
        let products = match query
            .order_by(product::Column::Code, Order::Asc)
            .offset(offset)
            .limit(page_size)
            .into_model::<product::Model>()
            .all(&*self.db)
            .await
            .map_err(AppError::from)
        {
            Ok(products) => {
                tracing::info!(
                    "查询到 {} 个产品（page={}, page_size={}, total={})",
                    products.len(),
                    page,
                    page_size,
                    total
                );
                products
            }
            Err(e) => {
                tracing::error!("查询产品列表失败: {:?}", e);
                return Err(e);
            }
        };

        Ok((products, total))
    }

    /// 获取产品详情
    pub async fn get_product(&self, id: i32) -> Result<product::Model, AppError> {
        // P0-D03：先查 Redis 缓存
        let cache_key_str = cache_key("product", id);
        if let Some(cached) = redis_cache_get_json::<product::Model>(&cache_key_str).await {
            return Ok(cached);
        }

        // 缓存未命中 → 查询 DB
        let product = ProductEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("产品 ID {} 不存在", id)))?;

        // 回填 Redis 缓存（V15 P2 B07-P2-6：产品目录 10 分钟 TTL，低波动率）
        redis_cache_set_json(&cache_key_str, &product, PRODUCT_CACHE_TTL_SECS).await;

        Ok(product)
    }

    /// 创建产品（面料行业版）（批次 339 v10 复审 P3 修复：签名从 19 参数改为单一参数对象 `CreateProductArgs`，；消除 `clippy::too_many_arguments` 警告。）
    pub async fn create_product(
        &self,
        args: CreateProductArgs,
    ) -> Result<product::Model, AppError> {
        let CreateProductArgs {
            name,
            code,
            barcode,
            category_id,
            specification,
            unit,
            standard_price,
            cost_price,
            description,
            status,
            product_type,
            fabric_composition,
            yarn_count,
            density,
            width,
            gram_weight,
            structure,
            finish,
            min_order_quantity,
            lead_time,
            execution_standard,
            factory_name,
            factory_address,
            product_grade,
            meters_per_piece,
            meters_per_roll,
        } = args;
        // 外键预校验：products.category_id REFERENCES product_categories(id)，
        // 无 ON DELETE 动作。若未预校验，非法 category_id 会在 insert 时抛出
        // "violates foreign key constraint fk_products_category" 并被映射成 500 DATABASE_ERROR，
        // 调用方无法区分数据错误与系统故障。
        if let Some(cat_id) = category_id {
            product_category::Entity::find_by_id(cat_id)
                .one(&*self.db)
                .await?
                .ok_or_else(|| AppError::business(format!("产品分类 {} 不存在", cat_id)))?;
        }

        let active_model = product::ActiveModel {
            id: NotSet,
            name: Set(name),
            code: Set(code),
            barcode: Set(barcode),
            category_id: Set(category_id),
            specification: Set(specification),
            unit: Set(unit),
            standard_price: Set(
                standard_price.map(|p| Decimal::from_f64_retain(p).unwrap_or(Decimal::ZERO))
            ),
            cost_price: Set(
                cost_price.map(|p| Decimal::from_f64_retain(p).unwrap_or(Decimal::ZERO))
            ),
            description: Set(description),
            status: Set(status),
            is_deleted: Set(false),
            // 面料行业字段
            product_type: Set(product_type),
            fabric_composition: Set(fabric_composition),
            yarn_count: Set(yarn_count),
            density: Set(density),
            width: Set(width.map(|w| Decimal::from_f64_retain(w).unwrap_or(Decimal::ZERO))),
            gram_weight: Set(
                gram_weight.map(|g| Decimal::from_f64_retain(g).unwrap_or(Decimal::ZERO))
            ),
            structure: Set(structure),
            finish: Set(finish),
            min_order_quantity: Set(
                min_order_quantity.map(|q| Decimal::from_f64_retain(q).unwrap_or(Decimal::ZERO))
            ),
            lead_time: Set(lead_time),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            supplier_product_code: sea_orm::ActiveValue::NotSet,
            supplier_id: sea_orm::ActiveValue::NotSet,
            is_batch_managed: sea_orm::ActiveValue::NotSet,
            batch_level: sea_orm::ActiveValue::NotSet,
            // V15 P1 合规字段（《产品质量法》第 27 条）
            execution_standard: Set(execution_standard),
            factory_name: Set(factory_name),
            factory_address: Set(factory_address),
            product_grade: Set(product_grade),
            // 匹/卷换算元数据：透传录入值（码↔匹↔卷换算单一真源入参，供 dual_unit_converter 读取）；
            // 未录入为 NULL，不影响既有；库存落库真相仍为米/公斤双列，本列不参与库存计量
            meters_per_piece: Set(meters_per_piece),
            meters_per_roll: Set(meters_per_roll),
        };

        let result = active_model.insert(&*self.db).await?;

        // 批次 125 v8 复审 P1 修复：PG 事务提交后同步到 ES（最终一致性）
        self.sync_product_to_es(&result, "create").await;

        Ok(result)
    }

    /// 删除产品
    pub async fn delete_product(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        // 引用预检与删除在同一事务、且在父行 lock_exclusive 之后执行
        //（通道与措辞形状照 warehouse_service::delete 先例）：迁移
        // migration::domain::price_fk 已给 sales_prices.product_id /
        // purchase_prices.product_id 施加指向 products.id 的 NO ACTION 外键，
        // 若无预检，删除被价目行引用的产品会撞 SQLSTATE 23503，经
        // utils::error::From<DbErr> 归类成 DatabaseError(DB_RELATION) 裸落 500
        //（出参脱敏为固定文案，用户不知道被谁引用）。并发窗口封堵原理：
        // 子表插入引用行会对父行取 FOR KEY SHARE，与本轮 FOR UPDATE 互斥，
        // 并发"插入价目行"要么先提交被本轮 COUNT 看到、要么阻塞到产品已删后
        // 其 FK 校验自然失败，"预检→删除"之间的 TOCTOU 窗口就此关闭。
        let txn = (*self.db).begin().await?;

        // 存在性校验 + 父行排他锁（锁持有至 txn 提交）；缺失 → 404，
        // 与原 delete_with_audit 内部缺失记录时的 not_found 语义一致
        product::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("产品 ID {} 不存在", id)))?;

        // 引用存在性预检：只统计与拒绝，绝不级联删除/软删价目行（FK 删除行为
        // 属迁移既定口径，见 migration::domain::price_fk 文件头"删除行为说明"）。
        // 文案只给"数量 + 业务名"，不输出表名/约束名/行 ID 列表（防出参泄露面）。
        if let Some((label, count)) = Self::find_product_price_references(id, &txn).await? {
            return Err(AppError::business_displayable(format!(
                "该产品已被 {count} 条{label}引用，无法删除，请先处理关联价目数据"
            )));
        }

        // P0 8-3 修复：delete 操作补审计日志
        // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
        let delete_result =
            crate::services::audit_log_service::AuditLogService::delete_with_audit::<
                ProductEntity,
                _,
            >(&txn, "product", id, Some(user_id))
            .await;
        // 竞态兜底映射（非吞异常、非返回成功）：引用表未纳入前置枚举或
        // deferred 约束在 DELETE 阶段真正命中 FK(23503) 时，降级为与预检同族
        // 的可外显业务拒绝；其余 DB 错误原样传播（真实故障保留 500）。
        if let Err(e) = delete_result {
            return Err(Self::map_product_fk_error(e));
        }

        txn.commit()
            .await
            .map_err(|e| Self::map_product_fk_error(AppError::from(e)))?;

        // P0-D03：失效产品缓存（产品已删除）
        redis_cache_del(&cache_key("product", id)).await;

        // 批次 125 v8 复审 P1 修复：PG 事务提交后删除 ES 文档（最终一致性）
        // 产品是硬删除，ES 文档也需删除
        if let Err(e) = self.search_syncer.delete_product(id).await {
            tracing::warn!(
                error = %e,
                product_id = id,
                operation = "delete",
                "ES 产品删除失败（PG 已提交，最终一致性靠补偿任务修复）"
            );
        }

        Ok(())
    }

    /// 功能：枚举对 products(id) 建外键的价目表，返回首个存在引用位的
    /// (业务可读名称, 引用行数)。入参：id=待删产品主键；db=与删除同事务的连接
    /// （由 delete_product 传入，预检必须先于 DELETE 且共享行锁窗口）。
    /// 清单与迁移 migration::domain::price_fk 的 `REFERENCES "products" ("id")`
    /// 逐一对齐：sales_prices.product_id / purchase_prices.product_id。
    /// FK 不区分价目状态（pending/approved/inactive）与 customer/supplier 归属，
    /// 计数口径与 FK 对齐：只要存在任意引用行删除即会被数据库拒绝，一律计入。
    async fn find_product_price_references<C: ConnectionTrait>(
        id: i32,
        db: &C,
    ) -> Result<Option<(&'static str, u64)>, AppError> {
        use crate::models::{purchase_price, sales_price};

        let sales = sales_price::Entity::find()
            .filter(sales_price::Column::ProductId.eq(id))
            .count(db)
            .await?;
        if sales > 0 {
            return Ok(Some(("销售价目记录", sales)));
        }

        let purchase = purchase_price::Entity::find()
            .filter(purchase_price::Column::ProductId.eq(id))
            .count(db)
            .await?;
        if purchase > 0 {
            return Ok(Some(("采购价目记录", purchase)));
        }

        Ok(None)
    }

    /// 功能：把删除阶段命中的外键/关联类数据库错误（`AppError::DatabaseError(DB_RELATION)`，
    /// 由 utils::error::From<DbErr> 对 SQLSTATE 23503 foreign key 违规归类）映射为
    /// 可外显业务拒绝（HTTP 400 / BUSINESS_ERROR）；非 FK 类错误原样返回
    /// （属真实内部故障，须保留 500，不吞异常、不拍平）。
    /// 调用方：delete_product 的删除与事务提交两处兜底；形状照
    /// supplier_service::map_supplier_fk_error / warehouse_service::map_warehouse_fk_error 先例。
    /// pub：竞态窗口（预检与 DELETE 之间新建价目行）在单连接测试中不可控，
    /// 契约测试需以真实 23503 DbErr 输入直测本映射判据。
    /// 入参：已归类后的 AppError；出参落点：调用方直接作为 Err 返回给 handler 出参。
    pub fn map_product_fk_error(err: AppError) -> AppError {
        match &err {
            AppError::DatabaseError(m) if m == err_msg::DB_RELATION => {
                AppError::business_displayable("该产品仍被业务数据引用，无法删除，请先处理关联数据")
            }
            _ => err,
        }
    }

    /// 更新产品（面料行业版，审计日志 + 失效缓存 + 同步 ES）
    pub async fn update_product(
        &self,
        mut args: UpdateProductArgs,
    ) -> Result<product::Model, AppError> {
        let id = args.id;
        let user_id = args.user_id;

        let mut product = self.fetch_product_active(id).await?;
        Self::apply_product_basic_fields(&mut product, &mut args);
        Self::apply_product_fabric_fields(&mut product, &mut args);
        product.updated_at = Set(Utc::now());

        let result = self.save_product_with_audit(product, id, user_id).await?;
        self.sync_product_to_es(&result, "update").await;
        Ok(result)
    }

    /// 按 ID 获取产品并转为 ActiveModel（不存在返回 not_found）
    async fn fetch_product_active(&self, id: i32) -> Result<product::ActiveModel, AppError> {
        Ok(ProductEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("产品 ID {} 不存在", id)))?
            .into())
    }

    /// 应用产品基础字段更新（名称/条码/规格/单位/价格/描述/状态）
    fn apply_product_basic_fields(
        product: &mut product::ActiveModel,
        args: &mut UpdateProductArgs,
    ) {
        if let Some(n) = args.name.take() {
            product.name = Set(n);
        }
        if let Some(b) = args.barcode.take() {
            product.barcode = Set(Some(b));
        }
        if let Some(spec) = args.specification.take() {
            product.specification = Set(Some(spec));
        }
        if let Some(u) = args.unit.take() {
            product.unit = Set(u);
        }
        if let Some(sp) = args.standard_price.take() {
            product.standard_price =
                Set(Some(Decimal::from_f64_retain(sp).unwrap_or(Decimal::ZERO)));
        }
        if let Some(cp) = args.cost_price.take() {
            product.cost_price = Set(Some(Decimal::from_f64_retain(cp).unwrap_or(Decimal::ZERO)));
        }
        if let Some(d) = args.description.take() {
            product.description = Set(Some(d));
        }
        if let Some(s) = args.status.take() {
            product.status = Set(s);
        }
    }

    /// 应用面料行业字段更新（类型/成分/纱支/密度/门幅/克重/组织/整理/MOQ/提前期）
    fn apply_product_fabric_fields(
        product: &mut product::ActiveModel,
        args: &mut UpdateProductArgs,
    ) {
        if let Some(pt) = args.product_type.take() {
            product.product_type = Set(pt);
        }
        if let Some(fc) = args.fabric_composition.take() {
            product.fabric_composition = Set(Some(fc));
        }
        if let Some(yc) = args.yarn_count.take() {
            product.yarn_count = Set(Some(yc));
        }
        if let Some(den) = args.density.take() {
            product.density = Set(Some(den));
        }
        if let Some(w) = args.width.take() {
            product.width = Set(Some(Decimal::from_f64_retain(w).unwrap_or(Decimal::ZERO)));
        }
        if let Some(gw) = args.gram_weight.take() {
            product.gram_weight = Set(Some(Decimal::from_f64_retain(gw).unwrap_or(Decimal::ZERO)));
        }
        if let Some(st) = args.structure.take() {
            product.structure = Set(Some(st));
        }
        if let Some(fi) = args.finish.take() {
            product.finish = Set(Some(fi));
        }
        if let Some(moq) = args.min_order_quantity.take() {
            product.min_order_quantity =
                Set(Some(Decimal::from_f64_retain(moq).unwrap_or(Decimal::ZERO)));
        }
        if let Some(lt) = args.lead_time.take() {
            product.lead_time = Set(Some(lt));
        }
        // V15 P1 合规字段（《产品质量法》第 27 条）
        if let Some(es) = args.execution_standard.take() {
            product.execution_standard = Set(Some(es));
        }
        if let Some(fn_) = args.factory_name.take() {
            product.factory_name = Set(Some(fn_));
        }
        if let Some(fa) = args.factory_address.take() {
            product.factory_address = Set(Some(fa));
        }
        if let Some(pg) = args.product_grade.take() {
            product.product_grade = Set(Some(pg));
        }
        // 匹/卷换算元数据：仅当传入 Some 才覆盖，None 保持原值（遵循可选数值字段条件 Set 惯例）
        if let Some(mpp) = args.meters_per_piece.take() {
            product.meters_per_piece = Set(Some(mpp));
        }
        if let Some(mpr) = args.meters_per_roll.take() {
            product.meters_per_roll = Set(Some(mpr));
        }
    }

    /// 审计更新产品并失效缓存（audit_log + redis_cache_del）
    async fn save_product_with_audit(
        &self,
        product: product::ActiveModel,
        id: i32,
        user_id: i32,
    ) -> Result<product::Model, AppError> {
        let result = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            product,
            Some(user_id),
        )
        .await?;
        redis_cache_del(&cache_key("product", id)).await;
        Ok(result)
    }
}
