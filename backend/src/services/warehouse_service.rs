use chrono::Utc;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, ExprTrait, NotSet, Order,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, SqlErr, TransactionTrait,
};

use crate::models::warehouse::{self, Entity as WarehouseEntity};
// 批次 211 P2-5 修复（v12 复审）：硬编码 "active" 替换为 master_data 常量
use crate::models::status::master_data;
use crate::utils::error::AppError;
use crate::utils::messages::err_msg;
use crate::utils::number_generator::DocumentNumberGenerator;
use crate::utils::sql_escape::safe_like_pattern;

crate::define_service!(WarehouseService);

/// 仓库编码（warehouses.warehouse_code）自动编码前缀：旧手写格式
/// `WH{13位毫秒时间戳}{4位随机}`（同毫秒并发即重号，直撞 UNIQUE——DDL 证据
/// migration/src/domain/system/m0001_initial_schema.rs:281
/// `"warehouse_code" VARCHAR(50) NOT NULL UNIQUE`），
/// 新格式统一 `{WH}{YYYYMMDD}{3位流水}`（前缀常量集中定义，
/// 对齐 chemical_ops/requisition.rs 风格；人工传入码原样保留，
/// 对齐 fixed_asset_service / quality_standard_service 分支形态）。
pub const WAREHOUSE_CODE_NO_PREFIX: &str = "WH";

impl WarehouseService {
    /// 获取仓库列表（支持分页和过滤）
    pub async fn list(
        &self,
        query: crate::handlers::warehouse_handler::WarehouseListQuery,
    ) -> Result<crate::utils::response::PaginatedResponse<warehouse::Model>, AppError> {
        let mut q = WarehouseEntity::find();

        // 应用过滤条件
        if let Some(s) = query.status {
            q = q.filter(warehouse::Column::IsActive.eq(s == master_data::ACTIVE));
        }

        if let Some(keyword) = query.search {
            let pattern = safe_like_pattern(&keyword);
            q = q.filter(
                warehouse::Column::Name
                    .like(&pattern)
                    .or(warehouse::Column::WarehouseCode.like(&pattern)),
            );
        }

        // 列表页「类型」下拉的取值来自该列的真实码，空串按未筛选处理
        if let Some(warehouse_type) = query.warehouse_type.filter(|s| !s.is_empty()) {
            q = q.filter(warehouse::Column::WarehouseType.eq(warehouse_type));
        }

        // 获取总数
        let total = q.clone().count(&*self.db).await?;

        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(10).clamp(1, 100); // v10 P1-1 修复：page_size clamp(1,100) 防 DoS

        // 应用分页和排序
        let warehouses = q
            .order_by(warehouse::Column::WarehouseCode, Order::Asc)
            .offset(page.saturating_sub(1) * page_size)
            .limit(page_size)
            .into_model::<warehouse::Model>()
            .all(&*self.db)
            .await?;

        Ok(crate::utils::response::PaginatedResponse::new(
            warehouses, total, page, page_size,
        ))
    }

    /// 获取仓库详情
    pub async fn get(&self, id: i32) -> Result<warehouse::Model, AppError> {
        WarehouseEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("仓库 ID {} 不存在", id)))
    }

    /// 创建仓库
    pub async fn create(
        &self,
        req: crate::handlers::warehouse_handler::CreateWarehouseRequest,
        user_id: i32,
    ) -> Result<warehouse::Model, AppError> {
        // 仓库编码：人工传入原样保留；缺省时由生成器取号（格式契约见
        // WAREHOUSE_CODE_NO_PREFIX 注释）。取号与 INSERT 同事务。
        let manual_code = req.code.clone().filter(|c| !c.is_empty());

        // P1 扩展：接入 manager（解析为 manager_id，与 update 方法对齐）；
        // 按引用匹配：req 后续还被取号闭包整体借用，不能部分移出字段
        let manager_id = match &req.manager {
            Some(m) if !m.is_empty() => match m.parse::<i32>() {
                Ok(parsed) => Some(parsed),
                Err(e) => {
                    tracing::warn!("仓库经理ID解析失败: {} ({})", m, e);
                    return Err(AppError::bad_request(format!("仓库经理ID格式错误：{}", m)));
                }
            },
            _ => None,
        };

        let txn = (*self.db).begin().await?;
        let result = match manual_code {
            // 人工指定码：先在同事务内预校验，编码已存在 → 可外显业务拒绝
            //（"改一个码再提交"属可执行公开规则，先例：chemical_ops/category.rs、
            // supplier 名称、 流程编码 = business_displayable 族）；文案只回显
            // 用户自己提交的编码，禁止撞 warehouses.warehouse_code UNIQUE(23505)
            // 经 From<DbErr> 拍平成 500 DATABASE_ERROR 裸抛，也禁止重新取号覆盖用户码。
            Some(code) => {
                if WarehouseEntity::find()
                    .filter(warehouse::Column::WarehouseCode.eq(&code))
                    .one(&txn)
                    .await?
                    .is_some()
                {
                    tracing::warn!(
                        "创建仓库被拒：人工指定编码 {} 已存在（操作人 {}）",
                        code,
                        user_id
                    );
                    return Err(AppError::business_displayable(format!(
                        "仓库编码 {} 已存在，请更换编码后重试",
                        code
                    )));
                }
                // 竞态兜底（照 role_permission/chemical_ops「预校验 + 23505 归类」范式）：
                // 预校验通过与 INSERT 之间并发请求可能已落同码，撞该表真实存在的
                // warehouse_code UNIQUE 时降级为与预校验同口径的业务拒绝——
                // 单语句 INSERT 原子失败、整事务回滚、不留半行。
                // 非唯一类 DbErr 走统一分类路径上抛，不吞不改道。
                let code_for_race = code.clone();
                Self::build_warehouse_active_model(code, &req, manager_id)
                    .insert(&txn)
                    .await
                    .map_err(|e| {
                        if matches!(e.sql_err(), Some(SqlErr::UniqueConstraintViolation(_))) {
                            tracing::error!(
                                "仓库创建撞编码唯一约束（并发同码，warehouse_code={}，操作人 {}）：行未写入、整事务回滚，底层错误={}",
                                code_for_race,
                                user_id,
                                e
                            );
                            AppError::business_displayable(format!(
                                "仓库编码 {} 已存在，请更换编码后重试",
                                code_for_race
                            ))
                        } else {
                            AppError::from(e)
                        }
                    })?
            }
            None => DocumentNumberGenerator::insert_with_no_retry(
                &txn,
                WAREHOUSE_CODE_NO_PREFIX,
                WarehouseEntity,
                warehouse::Column::WarehouseCode,
                |code| Self::build_warehouse_active_model(code, &req, manager_id),
            )
            .await
            .map_err(|e| {
                tracing::error!(
                    error = %e,
                    prefix = WAREHOUSE_CODE_NO_PREFIX,
                    "仓库编码取号/插入失败（warehouse_service.create）"
                );
                AppError::business_displayable("仓库编码生成失败，请稍后重试")
            })?,
        };
        txn.commit().await?;

        // 默认仓库全局唯一：新仓库为默认时，清除其他仓库的默认标志
        if result.is_default {
            Self::clear_other_default(&self.db, result.id).await?;
        }

        Ok(result)
    }

    /// 构建仓库 ActiveModel（人工码与自动取号两条路径共用字段装配，仅编码来源不同；
    /// `insert_with_no_retry` 的重试闭包要求可按候选号重复构建）
    fn build_warehouse_active_model(
        code: String,
        req: &crate::handlers::warehouse_handler::CreateWarehouseRequest,
        manager_id: Option<i32>,
    ) -> warehouse::ActiveModel {
        warehouse::ActiveModel {
            id: NotSet,
            warehouse_code: Set(code),
            name: Set(req
                .name
                .clone()
                .unwrap_or_else(|| format!("仓库_{}", Utc::now().timestamp()))),
            address: Set(req.address.clone()),
            city: Set(None),
            province: Set(None),
            country: Set(None),
            postal_code: Set(None),
            phone: Set(req.phone.clone()),
            // 契约对齐：前端创建表单 contact_person / is_default
            contact_person: Set(req.contact_person.clone()),
            is_default: Set(req.is_default.unwrap_or(false)),
            email: Set(None),
            manager_id: Set(manager_id),
            is_active: Set(true),
            // description 写入 notes 列
            notes: Set(req.description.clone()),
            // 仓库类型（greige=胚布仓/finished=成品仓/NULL 不校验），匹号领域规则使用
            warehouse_type: Set(req.warehouse_type.clone()),
            // 批次 158 v11 真实接入：capacity 字段持久化（原 #[allow(dead_code)] 移除）
            capacity: Set(req.capacity),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        }
    }

    /// 保证默认仓库全局唯一：将除 `keep_id` 外的仓库 is_default 置为 false
    async fn clear_other_default(
        db: &sea_orm::DatabaseConnection,
        keep_id: i32,
    ) -> Result<(), AppError> {
        use sea_orm::TransactionTrait;
        let txn = db.begin().await?;
        warehouse::Entity::update_many()
            .col_expr(warehouse::Column::IsDefault, Expr::value(false))
            .filter(warehouse::Column::Id.ne(keep_id))
            .filter(warehouse::Column::IsDefault.eq(true))
            .exec(&txn)
            .await?;
        txn.commit()
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        Ok(())
    }

    /// 更新仓库（批次 94 P2-10：补 user_id 参数，将 Some(0) 占位符改为真实操作人 user_id，；保证审计日志能追溯实际更新人。）
    ///
    /// 字段三态语义（对齐 RFC 7386 JSON Merge Patch）：
    /// None（键缺席）=保持原值、Some(None)（显式 null）=置 NULL（仅 DB 可空列）、Some(Some(v))=覆盖。
    pub async fn update(
        &self,
        id: i32,
        user_id: i32,
        req: crate::handlers::warehouse_handler::UpdateWarehouseRequest,
    ) -> Result<warehouse::Model, AppError> {
        // NOT NULL 列门控（warehouses.name，m0001 DDL；is_default/is_active（status 映射）
        // 实体 Model 为非 Option bool，置 NULL 后该行按模型不可读）：
        // 显式 null 是调用方错误，不是"保持原值"；在任何 DB 访问之前拒绝，错误外显不脱敏。
        if matches!(req.name, Some(None)) {
            return Err(AppError::business_displayable(
                "仓库名称不能清空：该字段为必填项",
            ));
        }
        if matches!(req.is_default, Some(None)) {
            return Err(AppError::business_displayable(
                "默认仓库标志不能清空：该字段为必填项",
            ));
        }
        if matches!(req.status, Some(None)) {
            return Err(AppError::business_displayable(
                "启用状态不能清空：该字段为必填项",
            ));
        }

        let mut wh: warehouse::ActiveModel = WarehouseEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("仓库 ID {} 不存在", id)))?
            .into();

        // 三态写入规则：None=不 Set（UPDATE 不含该列，原值不动）；
        // Some(None)=Set(None) 置 NULL；Some(Some(v))=Set(v) 覆盖
        // name/is_default/status 为 NOT NULL 列（Some(None) 已在入口拒绝）：仅覆盖/保持
        if let Some(n) = req.name.flatten() {
            wh.name = Set(n);
        }
        // address/warehouse_type/manager_id/phone/contact_person/capacity 为 DB 可空列：
        // Some(inner)=Set(inner)，显式 null 直落 NULL，不得塌成"保持原值"
        if let Some(a) = req.address {
            wh.address = Set(a);
        }
        if let Some(wt) = req.warehouse_type {
            wh.warehouse_type = Set(wt);
        }
        if let Some(m) = req.manager {
            match m {
                // 仓库经理 ID 解析失败时如实报错（记录 warn），不吞不兜底
                Some(m) => match m.parse::<i32>() {
                    Ok(parsed) => wh.manager_id = Set(Some(parsed)),
                    Err(e) => {
                        tracing::warn!("仓库经理ID解析失败: {} ({})", m, e);
                        return Err(AppError::bad_request(format!("仓库经理ID格式错误：{}", m)));
                    }
                },
                None => wh.manager_id = Set(None),
            }
        }
        if let Some(p) = req.phone {
            wh.phone = Set(p);
        }
        // 契约对齐：前端编辑表单 contact_person / is_default
        if let Some(cp) = req.contact_person {
            wh.contact_person = Set(cp);
        }
        if let Some(def) = req.is_default.flatten() {
            wh.is_default = Set(def);
        }
        if let Some(s) = req.status.flatten() {
            wh.is_active = Set(s == master_data::ACTIVE);
        }
        // 批次 158 v11 真实接入：capacity 字段持久化（原 #[allow(dead_code)] 移除）
        if let Some(c) = req.capacity {
            wh.capacity = Set(c);
        }

        wh.updated_at = Set(Utc::now());

        let result = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            wh,
            Some(user_id),
        )
        .await?;

        // 默认仓库全局唯一：更新为默认时，清除其他仓库的默认标志
        if result.is_default {
            Self::clear_other_default(&self.db, result.id).await?;
        }

        Ok(result)
    }

    /// 删除仓库（含真实操作人 user_id 审计追溯）。
    ///
    /// 口径与 supplier_service::delete_supplier / crm lead 同族：先做引用存在性预检，
    /// 命中即返回可外显业务拒绝（HTTP 400 / BUSINESS_ERROR），不再让数据库 FK 约束在
    /// DELETE 阶段裸冒 DATABASE_ERROR(500)（DELETE /warehouses/3 被
    /// fk_greige_fabric_warehouse 拒绝后 500）。禁止 CASCADE 静默删子行，禁止吞错返回成功。
    ///
    /// 与 supplier 先例的差异（更严）：预检查询全部放进与删除**同一事务**、且在父行
    /// lock_exclusive 之后执行——PG 子表插入会对父行取 FOR KEY SHARE，与 FOR UPDATE 互斥，
    /// 并发"插入引用行"要么先提交被本事务的 COUNT 看到、要么阻塞到仓库行已删（其 FK
    /// 校验自然失败），"预检→删除"之间的 TOCTOU 窗口就此关闭。
    pub async fn delete(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        let txn = (*self.db).begin().await?;

        // 存在性校验 + 父行排他锁（锁持有至 txn 提交）；缺失 → 404，与既有语义一致
        warehouse::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("仓库不存在"))?;

        // 引用存在性预检：命中任一引用即拒绝，文案只给"数量 + 业务名"，
        // 不输出表名/约束名/内部 ID 列表（防出参泄露面）。
        if let Some((label, count)) = self.find_warehouse_references(id, &txn).await? {
            return Err(AppError::business_displayable(format!(
                "该仓库已被 {count}{label}占用，无法删除，请先处理关联数据"
            )));
        }

        // P0 8-3 修复：delete 操作补审计日志（resource_type 保持 "warehouse"，审计查询口径不变）
        let delete_result =
            crate::services::audit_log_service::AuditLogService::delete_with_audit::<
                WarehouseEntity,
                _,
            >(&txn, "warehouse", id, Some(user_id))
            .await;
        // 兜底映射（非吞异常、非返回成功）：引用表未纳入前置枚举或 deferred 约束在 DELETE
        // 阶段真正命中 FK 时，降级为可读业务拒绝；其余 DB 错误原样传播（真实故障保留 500）。
        if let Err(e) = delete_result {
            return Err(Self::map_warehouse_fk_error(e));
        }

        txn.commit()
            .await
            .map_err(|e| Self::map_warehouse_fk_error(AppError::from(e)))
    }

    /// 将删除阶段命中数据库 FK/关联错误（`AppError::DatabaseError(DB_RELATION)`，由
    /// `From<DbErr>` 对 foreign key 违规归类）映射为可外显业务拒绝；非 FK 类数据库错误
    /// 原样返回。与 supplier_service::map_supplier_fk_error 同型。
    fn map_warehouse_fk_error(err: AppError) -> AppError {
        match &err {
            AppError::DatabaseError(m) if m == err_msg::DB_RELATION => {
                AppError::business_displayable("该仓库仍被业务数据占用，无法删除，请先处理关联数据")
            }
            _ => err,
        }
    }

    /// 枚举所有对 warehouses(id) 建外键的表，返回首个存在引用的 (业务可读名称, 数量)。
    /// 清单与迁移 grep `REFERENCES "warehouses" ("id")` 逐一对齐：
    /// purchase_receipt / sales_delivery / sales_return / inventory_transfers(from+to 双列) /
    /// inventory_adjustments / greige_fabric / inventory_stocks / inventory_reservations /
    /// inventory_transactions / inventory_piece / inventory_count_items / warehouse_locations /
    /// products。FK 不区分业务状态与软删标记（如 greige_fabric.is_deleted），计数口径与 FK
    /// 对齐：只要存在任意引用行，删除即会被数据库拒绝，故此处一律计入。
    /// products.warehouse_id 为 m0001 DDL 遗留列（实体 models/product.rs 未映射该列），
    /// 按 DDL 用参数化原生 COUNT 核对，不为凑实体字段新造第二套映射。
    async fn find_warehouse_references<C: ConnectionTrait>(
        &self,
        id: i32,
        db: &C,
    ) -> Result<Option<(&'static str, u64)>, AppError> {
        use crate::models::{
            greige_fabric, inventory_adjustment, inventory_count_item, inventory_piece,
            inventory_reservation, inventory_stock, inventory_transaction, inventory_transfer,
            location, purchase_receipt, sales_delivery, sales_return,
        };

        // 单据类在前、台账/库位/档案类在后：用户最先能行动的是"处理单据"
        macro_rules! first_hit {
            ($entity:ident, $col:expr, $label:literal) => {{
                let n = $entity::Entity::find()
                    .filter($col.eq(id))
                    .count(db)
                    .await?;
                if n > 0 {
                    return Ok(Some(($label, n)));
                }
            }};
        }

        first_hit!(
            purchase_receipt,
            purchase_receipt::Column::WarehouseId,
            "张入库单"
        );
        first_hit!(
            sales_delivery,
            sales_delivery::Column::WarehouseId,
            "张销售出库单"
        );
        first_hit!(
            sales_return,
            sales_return::Column::WarehouseId,
            "张销售退货单"
        );
        first_hit!(
            inventory_adjustment,
            inventory_adjustment::Column::WarehouseId,
            "张库存调整单"
        );

        // 调拨单：from/to 双列各建一条 FK（fk_inventory_transfers_from/to_warehouse）
        let n = inventory_transfer::Entity::find()
            .filter(
                inventory_transfer::Column::FromWarehouseId
                    .eq(id)
                    .or(inventory_transfer::Column::ToWarehouseId.eq(id)),
            )
            .count(db)
            .await?;
        if n > 0 {
            return Ok(Some(("张调拨单", n)));
        }

        first_hit!(
            greige_fabric,
            greige_fabric::Column::WarehouseId,
            "条坯布库存记录"
        );
        first_hit!(
            inventory_stock,
            inventory_stock::Column::WarehouseId,
            "条库存台账记录"
        );
        first_hit!(
            inventory_reservation,
            inventory_reservation::Column::WarehouseId,
            "条库存预留记录"
        );
        first_hit!(
            inventory_transaction,
            inventory_transaction::Column::WarehouseId,
            "条出入库流水记录"
        );
        first_hit!(
            inventory_piece,
            inventory_piece::Column::WarehouseId,
            "条库存匹码记录"
        );
        first_hit!(
            inventory_count_item,
            inventory_count_item::Column::WarehouseId,
            "条盘点明细记录"
        );
        first_hit!(location, location::Column::WarehouseId, "个库位");

        let row: Option<sea_orm::QueryResult> = db
            .query_one_raw(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                r#"SELECT COUNT(*) FROM "products" WHERE "warehouse_id" = $1"#,
                [id.into()],
            ))
            .await?;
        let n = row
            .and_then(|r| r.try_get_by_index::<i64>(0).ok())
            .unwrap_or(0);
        if n > 0 {
            return Ok(Some(("个产品档案的默认仓关联", n as u64)));
        }

        Ok(None)
    }
}
