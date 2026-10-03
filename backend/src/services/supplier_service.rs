use crate::models::{supplier, supplier_contact, supplier_qualification};
// V15 P0-S01：行级数据权限工具
use crate::utils::data_scope::{DataScope, DataScopeContext, check_resource_owner};
use crate::utils::error::AppError;
use crate::utils::messages::err_msg;
use crate::utils::number_generator::DocumentNumberGenerator;
use crate::utils::pagination::paginate_with_total;
// P0-D03（Batch 488）：Redis 分布式缓存接入（get_supplier 读穿透 + 写失效）
use crate::utils::redis_cache::{
    DEFAULT_CACHE_TTL_SECS, cache_key, redis_cache_del, redis_cache_get_json, redis_cache_set_json,
};
use crate::utils::response::PaginatedResponse;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, ExprTrait, Order,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use validator::Validate;

// P2 1-8 修复：手机号正则全局编译一次，避免每次 validate_mobile_phone 调用都重新编译（NFA→DFA 开销）
// 批次 404 修复：正则编译失败时优雅降级（返回验证失败而非 panic）。
// 模式为静态字面量，编译失败概率几乎为零；但若 regex crate 版本升级导致行为变化，
// 用 Option<Regex> 可让进程继续运行而非崩溃。
static MOBILE_PHONE_RE: std::sync::LazyLock<Option<regex::Regex>> =
    std::sync::LazyLock::new(|| regex::Regex::new(r"^1[3-9]\d{9}$").ok());

/// 供应商服务
pub struct SupplierService {
    db: Arc<DatabaseConnection>,
}

/// 供应商编码（suppliers.supplier_code）前缀常量：取号格式契约集中定义
/// （对齐 WAREHOUSE_CODE_NO_PREFIX 形态）；DDL 证据
/// migration/src/domain/system/m0001_initial_schema.rs:303
/// `"supplier_code" VARCHAR(50) NOT NULL UNIQUE` —— 取号与 INSERT 之间的并发
/// 撞号由 DocumentNumberGenerator::insert_with_no_retry 保存点重试兜底（见
/// create_supplier），禁止第二处手写取号+直插。
pub const SUPPLIER_CODE_NO_PREFIX: &str = "SUP";

impl SupplierService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建供应商（含联系人和资质，事务保证三表原子写入）
    pub async fn create_supplier(
        &self,
        req: CreateSupplierRequest,
        user_id: i32,
    ) -> Result<supplier::Model, AppError> {
        self.check_supplier_name_unique(&req.supplier_name).await?;

        let txn = (*self.db).begin().await?;
        // 编码取号 + 判重 + INSERT 收敛到 DocumentNumberGenerator::insert_with_no_retry
        // 唯一实现（同 warehouse_service::create 自动码路径）：撞 suppliers.supplier_code
        // UNIQUE(23505) 时保存点重试重新取号，不让 From<DbErr> 把并发撞号拍平成
        // 500 DATABASE_ERROR 裸抛；失败整事务回滚，联系人/资质不落半行。
        let supplier = DocumentNumberGenerator::insert_with_no_retry(
            &txn,
            SUPPLIER_CODE_NO_PREFIX,
            supplier::Entity,
            supplier::Column::SupplierCode,
            |code| Self::build_supplier_active_model(&req, code, user_id),
        )
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                prefix = SUPPLIER_CODE_NO_PREFIX,
                "供应商编码取号/插入失败（supplier_service.create_supplier）"
            );
            AppError::business_displayable("供应商编码生成失败，请稍后重试")
        })?;

        if let Some(contacts) = req.contacts {
            Self::insert_supplier_contacts(&txn, supplier.id, contacts).await?;
        }
        if let Some(qualifications) = req.qualifications {
            Self::insert_supplier_qualifications(&txn, supplier.id, qualifications).await?;
        }

        txn.commit().await?;
        Ok(supplier)
    }

    /// 校验供应商名称唯一性
    async fn check_supplier_name_unique(&self, supplier_name: &str) -> Result<(), AppError> {
        let existing = supplier::Entity::find()
            .filter(supplier::Column::SupplierName.eq(supplier_name))
            .one(&*self.db)
            .await?;
        if existing.is_some() {
            // 唯一性冲突：供应商名称重复，归业务族；回显用户自己提交的名称可外显
            return Err(AppError::business_displayable(format!(
                "供应商名称 '{}' 已存在",
                supplier_name
            )));
        }
        Ok(())
    }

    /// 构建供应商 ActiveModel（含默认值：普通供应商/一般纳税人/今日成立日期）
    fn build_supplier_active_model(
        req: &CreateSupplierRequest,
        supplier_code: String,
        user_id: i32,
    ) -> supplier::ActiveModel {
        supplier::ActiveModel {
            supplier_code: Set(supplier_code),
            supplier_name: Set(req.supplier_name.clone()),
            supplier_short_name: Set(req.supplier_short_name.clone().unwrap_or_default()),
            supplier_type: Set(req
                .supplier_type
                .clone()
                .unwrap_or_else(|| "普通供应商".to_string())),
            credit_code: Set(req.credit_code.clone().unwrap_or_default()),
            registered_address: Set(req.registered_address.clone().unwrap_or_default()),
            business_address: Set(req.business_address.clone()),
            legal_representative: Set(req.legal_representative.clone().unwrap_or_default()),
            registered_capital: Set(req.registered_capital.unwrap_or_default()),
            establishment_date: Set(req
                .establishment_date
                .unwrap_or_else(|| chrono::Utc::now().date_naive())),
            business_term: Set(req.business_term.clone()),
            business_scope: Set(req.business_scope.clone()),
            taxpayer_type: Set(req
                .taxpayer_type
                .clone()
                .unwrap_or_else(|| "一般纳税人".to_string())),
            bank_name: Set(req.bank_name.clone().unwrap_or_default()),
            bank_account: Set(req.bank_account.clone().unwrap_or_default()),
            contact_phone: Set(req.contact_phone.clone().unwrap_or_default()),
            fax: Set(req.fax.clone()),
            website: Set(req.website.clone()),
            contact_email: Set(req.email.clone()),
            main_business: Set(req.main_business.clone()),
            main_market: Set(req.main_market.clone()),
            employee_count: Set(req.employee_count),
            annual_revenue: Set(req.annual_revenue),
            category_id: Set(req.category_id),
            is_processor: Set(req.is_processor.unwrap_or(false)),
            processor_type: Set(req.processor_type.clone()),
            // 备注与更新路径同源写入（更新分支见 `apply_supplier_business_fields`）：
            // 建单时用户填的备注必须落库，入参省略则该可空列保持 NULL（不造默认值）。
            remarks: Set(req.remarks.clone()),
            created_by: Set(Some(user_id)),
            ..Default::default()
        }
    }

    /// 批量插入供应商联系人
    async fn insert_supplier_contacts(
        txn: &sea_orm::DatabaseTransaction,
        supplier_id: i32,
        contacts: Vec<CreateContactRequest>,
    ) -> Result<(), AppError> {
        for contact_req in contacts {
            supplier_contact::ActiveModel {
                supplier_id: Set(supplier_id),
                contact_name: Set(contact_req.contact_name),
                department: Set(contact_req.department),
                position: Set(contact_req.position),
                mobile_phone: Set(contact_req.mobile_phone),
                tel_phone: Set(contact_req.tel_phone),
                email: Set(contact_req.email),
                wechat: Set(contact_req.wechat),
                qq: Set(contact_req.qq),
                is_primary: Set(contact_req.is_primary),
                remarks: Set(contact_req.remarks),
                ..Default::default()
            }
            .insert(txn)
            .await?;
        }
        Ok(())
    }

    /// 批量插入供应商资质
    async fn insert_supplier_qualifications(
        txn: &sea_orm::DatabaseTransaction,
        supplier_id: i32,
        qualifications: Vec<CreateQualificationRequest>,
    ) -> Result<(), AppError> {
        for qual_req in qualifications {
            Self::check_qualification_dates(qual_req.issue_date, qual_req.valid_until)?;
            supplier_qualification::ActiveModel {
                supplier_id: Set(supplier_id),
                qualification_name: Set(qual_req.qualification_name),
                qualification_type: Set(qual_req.qualification_type),
                qualification_no: Set(qual_req.qualification_no),
                issuing_authority: Set(qual_req.issuing_authority),
                issue_date: Set(qual_req.issue_date),
                valid_until: Set(qual_req.valid_until),
                attachment_path: Set(qual_req.attachment_path),
                need_annual_check: Set(qual_req.need_annual_check),
                annual_check_record: Set(qual_req.annual_check_record),
                // is_expired 由 valid_until 与当前日期如实派生（NOT NULL 列，写入即落真实值）
                is_expired: Set(Self::qualification_is_expired(qual_req.valid_until)),
                ..Default::default()
            }
            .insert(txn)
            .await?;
        }
        Ok(())
    }

    /// 查询供应商列表（分页、筛选、排序）
    pub async fn list_suppliers(
        &self,
        params: SupplierQueryParams,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<PaginatedResponse<supplier::Model>, AppError> {
        let mut query = supplier::Entity::find();

        // 行级数据权限过滤：owner=CreatedBy，dept=DepartmentId（m_rls_dept_domain）
        // 供应商为主数据：created_by IS NULL 的系统种子/共享记录对所有已认证用户可见
        if let Some(ctx) = data_scope {
            let scope_cond = match ctx.scope {
                DataScope::All => Condition::all(),
                DataScope::Dept => {
                    let owner_branch = if ctx.dept_ids.is_empty() {
                        Condition::all().add(supplier::Column::CreatedBy.eq(ctx.user_id))
                    } else {
                        Condition::any()
                            .add(supplier::Column::CreatedBy.eq(ctx.user_id))
                            .add(supplier::Column::DepartmentId.is_in(ctx.dept_ids.clone()))
                    };
                    owner_branch.add(supplier::Column::CreatedBy.is_null())
                }
                DataScope::Self_ => Condition::any()
                    .add(supplier::Column::CreatedBy.eq(ctx.user_id))
                    .add(supplier::Column::CreatedBy.is_null()),
            };
            query = query.filter(scope_cond);
        }

        // 应用筛选与排序
        query = Self::apply_supplier_filters(query, &params);
        let sort_by = params.sort_by.unwrap_or_else(|| "created_at".to_string());
        let sort_order = params.sort_order.unwrap_or_else(|| "DESC".to_string());
        query = Self::apply_supplier_sorting(query, &sort_by, &sort_order);

        // 分页
        let page = params.page.unwrap_or(1);
        let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

        let paginator = query.paginate(&*self.db, page_size);
        // 批次 255 修复：接入 paginate_with_total 统一分页逻辑（内部已处理 saturating_sub(1) 偏移）
        // 保留 page.clamp(1, 1000) 防 DoS（批次 98 P2-A 修复）
        let (data, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;

        Ok(PaginatedResponse::new(data, total, page, page_size))
    }

    /// 应用供应商筛选条件（类型/等级/状态/关键字/启用状态）
    fn apply_supplier_filters(
        mut query: sea_orm::Select<supplier::Entity>,
        params: &SupplierQueryParams,
    ) -> sea_orm::Select<supplier::Entity> {
        if let Some(v) = &params.supplier_type {
            query = query.filter(supplier::Column::SupplierType.eq(v.as_str()));
        }
        if let Some(v) = &params.grade {
            query = query.filter(supplier::Column::Grade.eq(v.as_str()));
        }
        if let Some(v) = &params.status {
            query = query.filter(supplier::Column::Status.eq(v.as_str()));
        }
        if let Some(v) = &params.keyword {
            query = query.filter(
                supplier::Column::SupplierName
                    .contains(v.as_str())
                    .or(supplier::Column::SupplierCode.contains(v.as_str()))
                    .or(supplier::Column::CreditCode.contains(v.as_str())),
            );
        }
        if let Some(is_enabled) = params.is_enabled {
            query = query.filter(supplier::Column::IsEnabled.eq(is_enabled));
        }
        if let Some(category_id) = params.category_id {
            query = query.filter(supplier::Column::CategoryId.eq(category_id));
        }
        if let Some(is_processor) = params.is_processor {
            query = query.filter(supplier::Column::IsProcessor.eq(is_processor));
        }
        if let Some(ref processor_type) = params.processor_type {
            query = query.filter(supplier::Column::ProcessorType.eq(processor_type.as_str()));
        }
        query
    }

    /// 应用供应商排序（按 sort_by 字段与 sort_order 方向）
    fn apply_supplier_sorting(
        query: sea_orm::Select<supplier::Entity>,
        sort_by: &str,
        sort_order: &str,
    ) -> sea_orm::Select<supplier::Entity> {
        let is_asc = sort_order == "ASC";
        match sort_by {
            "supplier_code" => {
                if is_asc {
                    query.order_by(supplier::Column::SupplierCode, Order::Asc)
                } else {
                    query.order_by(supplier::Column::SupplierCode, Order::Desc)
                }
            }
            "supplier_name" => {
                if is_asc {
                    query.order_by(supplier::Column::SupplierName, Order::Asc)
                } else {
                    query.order_by(supplier::Column::SupplierName, Order::Desc)
                }
            }
            "grade" => {
                if is_asc {
                    query.order_by(supplier::Column::Grade, Order::Asc)
                } else {
                    query.order_by(supplier::Column::Grade, Order::Desc)
                }
            }
            _ => {
                if is_asc {
                    query.order_by(supplier::Column::CreatedAt, Order::Asc)
                } else {
                    query.order_by(supplier::Column::CreatedAt, Order::Desc)
                }
            }
        }
    }

    /// 获取供应商详情
    /// P0-D03（Batch 488）：接入 Redis 分布式缓存（5 分钟 TTL）；读穿透：先查 Redis，未命中查 DB 后回填 Redis；写失效：update/delete/toggle_status 时清除对应 key；权限校验：基于 supplier.created_by，缓存命中时同样校验
    pub async fn get_supplier(
        &self,
        id: i32,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<supplier::Model, AppError> {
        // P0-D03：先查 Redis 缓存
        let cache_key_str = cache_key("supplier", id);
        let cached: Option<supplier::Model> =
            redis_cache_get_json::<supplier::Model>(&cache_key_str).await;

        let supplier_model = if let Some(model) = cached {
            model
        } else {
            // 缓存未命中 → 查询 DB
            let model = supplier::Entity::find_by_id(id)
                .one(&*self.db)
                .await?
                .ok_or_else(|| AppError::not_found(format!("供应商 {} 不存在", id)))?;
            // 回填 Redis 缓存（5 分钟 TTL）
            redis_cache_set_json(&cache_key_str, &model, DEFAULT_CACHE_TTL_SECS).await;
            model
        };

        // V15 P0-S01：行级数据权限校验（IDOR 防护）
        // 供应商为主数据：created_by IS NULL 视为系统/共享种子记录，不受数据范围约束
        if let Some(ctx) = data_scope {
            if supplier_model.created_by.is_some()
                && !check_resource_owner(
                    ctx,
                    supplier_model.created_by,
                    supplier_model.department_id,
                )
            {
                return Err(AppError::permission_denied(format!(
                    "无权访问供应商 {}（数据范围限制）",
                    id
                )));
            }
        }

        Ok(supplier_model)
    }

    /// 更新供应商信息
    pub async fn update_supplier(
        &self,
        id: i32,
        req: UpdateSupplierRequest,
        user_id: i32,
    ) -> Result<supplier::Model, AppError> {
        // V15 P0-S01：内部调用传 None（权限校验已由 update_supplier 的 handler 入口完成）
        let supplier = self.get_supplier(id, None).await?;
        // 改名判重收敛到 check_supplier_name_unique 唯一实现（与 create_supplier 同
        // 口径，禁止两份规则）：改到其它既有供应商的名称 → 可外显业务拒绝；
        // 与自身当前名相同（幂等 PUT）放行，防过度收紧打错既有行为。
        // 本列无 DB UNIQUE（竞态窗需数据库专家补约束，见 wave 报告 §④），
        // 应用层预校验为当前唯一防线。
        if let Some(new_name) = req.supplier_name.as_ref() {
            if new_name != &supplier.supplier_name {
                self.check_supplier_name_unique(new_name).await?;
            }
        }
        let mut supplier_active: supplier::ActiveModel = supplier.into();
        let mut req = req;
        // 更新字段（分组应用，每组 ≤50 行；使用 .take() 从 &mut req 移出字段值）
        Self::apply_supplier_basic_fields(&mut supplier_active, &mut req);
        Self::apply_supplier_contact_fields(&mut supplier_active, &mut req);
        Self::apply_supplier_business_fields(&mut supplier_active, &mut req);
        supplier_active.updated_by = Set(Some(user_id));
        supplier_active.updated_at = Set(Utc::now().into());
        // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            supplier_active,
            Some(user_id),
        )
        .await?;
        // B-P1-3 修复（批次 384 v13 复审）：供应商主数据变更后发布事件，触发关联单据冗余字段刷新
        crate::services::event_bus::EVENT_BUS.publish(
            crate::services::event_bus::BusinessEvent::SupplierUpdated {
                supplier_id: updated.id,
                supplier_name: updated.supplier_name.clone(),
                user_id,
            },
        );
        // P0-D03：失效供应商缓存（供应商信息已更新）
        redis_cache_del(&cache_key("supplier", id)).await;
        Ok(updated)
    }

    /// 应用供应商基本字段更新（名称/类型/地址/法人/资本/成立日期，共 9 字段）。
    fn apply_supplier_basic_fields(
        active: &mut supplier::ActiveModel,
        req: &mut UpdateSupplierRequest,
    ) {
        if let Some(v) = req.supplier_name.take() {
            active.supplier_name = Set(v);
        }
        if let Some(v) = req.supplier_short_name.take() {
            active.supplier_short_name = Set(v);
        }
        if let Some(v) = req.supplier_type.take() {
            active.supplier_type = Set(v);
        }
        if let Some(v) = req.credit_code.take() {
            active.credit_code = Set(v);
        }
        if let Some(v) = req.registered_address.take() {
            active.registered_address = Set(v);
        }
        if let Some(v) = req.business_address.take() {
            active.business_address = Set(Some(v));
        }
        if let Some(v) = req.legal_representative.take() {
            active.legal_representative = Set(v);
        }
        if let Some(v) = req.registered_capital.take() {
            active.registered_capital = Set(v);
        }
        if let Some(v) = req.establishment_date.take() {
            active.establishment_date = Set(v);
        }
    }

    /// 应用供应商联系与银行字段更新（经营范围/纳税/银行/电话/传真/网址/邮箱，共 9 字段）。
    fn apply_supplier_contact_fields(
        active: &mut supplier::ActiveModel,
        req: &mut UpdateSupplierRequest,
    ) {
        if let Some(v) = req.business_term.take() {
            active.business_term = Set(Some(v));
        }
        if let Some(v) = req.business_scope.take() {
            active.business_scope = Set(Some(v));
        }
        if let Some(v) = req.taxpayer_type.take() {
            active.taxpayer_type = Set(v);
        }
        if let Some(v) = req.bank_name.take() {
            active.bank_name = Set(v);
        }
        if let Some(v) = req.bank_account.take() {
            active.bank_account = Set(v);
        }
        if let Some(v) = req.contact_phone.take() {
            active.contact_phone = Set(v);
        }
        if let Some(v) = req.fax.take() {
            active.fax = Set(Some(v));
        }
        if let Some(v) = req.website.take() {
            active.website = Set(Some(v));
        }
        if let Some(v) = req.email.take() {
            active.contact_email = Set(Some(v));
        }
    }

    /// 应用供应商业务与评级字段更新（主营/市场/员工/营收/等级/状态/备注，共 9 字段）。
    fn apply_supplier_business_fields(
        active: &mut supplier::ActiveModel,
        req: &mut UpdateSupplierRequest,
    ) {
        if let Some(v) = req.main_business.take() {
            active.main_business = Set(Some(v));
        }
        if let Some(v) = req.main_market.take() {
            active.main_market = Set(Some(v));
        }
        if let Some(v) = req.employee_count.take() {
            active.employee_count = Set(Some(v));
        }
        if let Some(v) = req.annual_revenue.take() {
            active.annual_revenue = Set(Some(v));
        }
        if let Some(v) = req.category_id.take() {
            active.category_id = Set(Some(v));
        }
        if let Some(v) = req.is_processor.take() {
            active.is_processor = Set(v);
        }
        if let Some(v) = req.processor_type.take() {
            active.processor_type = Set(Some(v));
        }
        if let Some(v) = req.grade.take() {
            active.grade = Set(Some(v));
        }
        if let Some(v) = req.grade_score.take() {
            active.grade_score = Set(Some(v));
        }
        if let Some(v) = req.status.take() {
            active.status = Set(Some(v));
        }
        if let Some(v) = req.is_enabled.take() {
            active.is_enabled = Set(Some(v));
        }
        if let Some(v) = req.remarks.take() {
            active.remarks = Set(Some(v));
        }
    }

    /// 删除供应商
    // 批次 93 P1-5 修复：补 user_id 参数 + txn + lock_exclusive + 审计日志
    pub async fn delete_supplier(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        // 引用校验前置：供应商被任一业务/档案单据引用时，返回带具体引用类型的业务错误，
        // 而非依赖数据库 FK 约束在 delete 阶段裸抛 DATABASE_ERROR(500)（见 CI #4653：
        // DELETE /purchase/suppliers/3 被 fk_ap_payment_request_supplier 拒后 500）。
        if let Some(ref_label) = self.find_supplier_reference(id).await? {
            return Err(AppError::business_displayable(format!(
                "该供应商已被{}引用，无法删除",
                ref_label
            )));
        }

        // 批次 93 P1-5 修复：get + delete 移入同一事务，补 lock_exclusive 串行化并发
        // 原实现 get_supplier 在 self.db → delete 在 self.db，两步非原子，
        // 并发场景下 get 与 delete 之间可能出现关联交易记录插入，绕过引用门控。
        let txn = (*self.db).begin().await?;

        // lock_exclusive 串行化并发删除；锁持有至 txn 提交，model 仅用于持锁与存在性校验
        supplier::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("供应商 {} 不存在", id)))?;

        // 删除供应商（含审计日志）
        let delete_result =
            crate::services::audit_log_service::AuditLogService::delete_with_audit::<
                supplier::Entity,
                _,
            >(&txn, "supplier", id, Some(user_id))
            .await;
        // 兜底：count 与 delete 之间存在并发窗口，或引用表未纳入前置枚举，
        // 数据库 FK(23503/数据关联错误) 仍可能命中；映射为业务错误，绝不放行裸 500。
        if let Err(e) = delete_result {
            return Err(Self::map_supplier_fk_error(e));
        }

        txn.commit()
            .await
            .map_err(|e| Self::map_supplier_fk_error(AppError::from(e)))?;

        // P0-D03：失效供应商缓存（供应商已硬删除）
        redis_cache_del(&cache_key("supplier", id)).await;

        Ok(())
    }

    /// 将删除阶段命中数据库 FK/关联错误（`AppError::DatabaseError(DB_RELATION)`，
    /// 由 `From<DbErr>` 对 foreign key 违规归类）映射为可外显业务错误；
    /// 非 FK 类数据库错误原样返回（属真实内部故障，须保留 500，不吞异常）。
    fn map_supplier_fk_error(err: AppError) -> AppError {
        match &err {
            AppError::DatabaseError(m) if m == err_msg::DB_RELATION => {
                AppError::business_displayable("该供应商已被业务单据引用，无法删除")
            }
            _ => err,
        }
    }

    /// 枚举所有对 suppliers(id) 建外键（或语义上引用供应商）的业务/档案表，
    /// 返回首个存在引用的表的可读名称。任一命中即不应删除供应商。
    /// 与数据库 FK 集对齐：迁移 grep `REFERENCES "suppliers" ("id")` 得到的引用表清单
    /// （purchase_orders / purchase_receipt / purchase_contracts / ap_payment_request /
    /// ap_invoice / ap_payment / ap_reconciliation / ap_verification /
    /// product_supplier_mappings / supplier_evaluation_records / greige_fabric）。
    /// 说明：purchase_orders 原实现仅统计“未完成”状态，但 FK 不区分状态，
    /// 只要存在任意历史订单删除即会 500，故此处改为统计全部状态以与 FK 对齐。
    async fn find_supplier_reference(&self, id: i32) -> Result<Option<&'static str>, AppError> {
        use crate::models::{
            ap_invoice, ap_payment, ap_payment_request, ap_reconciliation, ap_verification,
            greige_fabric, product_supplier_mapping, purchase_contract, purchase_order,
            purchase_receipt, supplier_evaluation_record,
        };

        // (实体查询闭包, 引用类型名称)；顺序即用户看到的优先级（先财务硬引用后档案）。
        macro_rules! count_refs {
            ($entity:ident, $col:expr) => {{
                $entity::Entity::find()
                    .filter($col.eq(id))
                    .count(&*self.db)
                    .await?
            }};
        }

        let checks: Vec<(u64, &'static str)> = vec![
            (
                count_refs!(ap_payment_request, ap_payment_request::Column::SupplierId),
                "应付付款申请",
            ),
            (
                count_refs!(ap_payment, ap_payment::Column::SupplierId),
                "付款记录",
            ),
            (
                count_refs!(ap_invoice, ap_invoice::Column::SupplierId),
                "应付发票",
            ),
            (
                count_refs!(ap_reconciliation, ap_reconciliation::Column::SupplierId),
                "应付对账单",
            ),
            (
                count_refs!(ap_verification, ap_verification::Column::SupplierId),
                "应付核销单",
            ),
            (
                count_refs!(purchase_order, purchase_order::Column::SupplierId),
                "采购订单",
            ),
            (
                count_refs!(purchase_receipt, purchase_receipt::Column::SupplierId),
                "采购收货单",
            ),
            (
                count_refs!(purchase_contract, purchase_contract::Column::SupplierId),
                "采购合同",
            ),
            (
                count_refs!(
                    product_supplier_mapping,
                    product_supplier_mapping::Column::SupplierId
                ),
                "供应商商品对照表",
            ),
            (
                count_refs!(
                    supplier_evaluation_record,
                    supplier_evaluation_record::Column::SupplierId
                ),
                "供应商评估记录",
            ),
            (
                count_refs!(greige_fabric, greige_fabric::Column::SupplierId),
                "坯布台账",
            ),
        ];

        Ok(checks.into_iter().find(|(n, _)| *n > 0).map(|(_, l)| l))
    }

    /// 切换供应商状态
    pub async fn toggle_supplier_status(
        &self,
        id: i32,
        enable: bool,
        user_id: i32,
    ) -> Result<supplier::Model, AppError> {
        // V15 P0-S01：内部调用传 None（权限校验已由 toggle_supplier_status 的 handler 入口完成）
        let supplier = self.get_supplier(id, None).await?;
        let mut supplier_active: supplier::ActiveModel = supplier.into();

        supplier_active.is_enabled = Set(Some(enable));
        supplier_active.status = Set(Some(if enable {
            crate::models::status::master_data::ACTIVE.to_string()
        } else {
            crate::models::status::master_data::INACTIVE.to_string()
        }));
        supplier_active.updated_by = Set(Some(user_id));
        supplier_active.updated_at = Set(Utc::now().into());

        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            supplier_active,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        // P0-D03：失效供应商缓存（启用/禁用状态已变更）
        redis_cache_del(&cache_key("supplier", id)).await;

        Ok(updated)
    }

    // ==================== 供应商联系人管理方法 ====================

    /// 获取供应商联系人列表
    pub async fn list_supplier_contacts(
        &self,
        supplier_id: i32,
    ) -> Result<Vec<supplier_contact::Model>, AppError> {
        let contacts = supplier_contact::Entity::find()
            .filter(supplier_contact::Column::SupplierId.eq(supplier_id))
            .order_by(supplier_contact::Column::IsPrimary, Order::Desc)
            .order_by(supplier_contact::Column::ContactName, Order::Asc)
            .all(&*self.db)
            .await?;
        Ok(contacts)
    }

    /// 创建供应商联系人（P2-6 修复（v12 复审）：clear_primary_contacts + insert 包入同一事务，；保证"取消旧主联系人 + 新建联系人"原子性，避免中途失败留下脏数据。）
    pub async fn create_supplier_contact(
        &self,
        supplier_id: i32,
        req: CreateContactRequest,
        // P2-6 修复后 user_id 不再用于 clear_primary_contacts_txn（防审计日志膨胀）；
        // supplier_contact 模型无 created_by 字段，参数保留以维持 API 签名兼容。
        _user_id: i32,
    ) -> Result<supplier_contact::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 如果设置为主要联系人，先将其他联系人取消主要联系人状态
        if req.is_primary {
            self.clear_primary_contacts_txn(supplier_id, &txn).await?;
        }

        let now: sea_orm::prelude::DateTimeWithTimeZone = Utc::now().into();
        let contact = supplier_contact::ActiveModel {
            supplier_id: Set(supplier_id),
            contact_name: Set(req.contact_name),
            department: Set(req.department),
            position: Set(req.position),
            mobile_phone: Set(req.mobile_phone),
            tel_phone: Set(req.tel_phone),
            email: Set(req.email),
            wechat: Set(req.wechat),
            qq: Set(req.qq),
            is_primary: Set(req.is_primary),
            remarks: Set(req.remarks),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;
        Ok(contact)
    }

    /// 更新供应商联系人（P2-6 修复（v12 复审）：clear_primary_contacts + update 包入同一事务，；保证"取消旧主联系人 + 更新联系人"原子性。）
    /// 越权修复：路径 (supplier_id, contact_id) 必须一致——联系人不属于路径供应商时
    /// 返回用户可见业务错误（business_displayable，business 会被出参脱敏），
    /// 防止以 contact_id 单键跨供应商改写他人联系人。
    pub async fn update_supplier_contact(
        &self,
        supplier_id: i32,
        contact_id: i32,
        req: UpdateContactRequest,
        user_id: i32,
    ) -> Result<supplier_contact::Model, AppError> {
        let txn = (*self.db).begin().await?;

        let contact = supplier_contact::Entity::find_by_id(contact_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("联系人 {} 不存在", contact_id)))?;

        if contact.supplier_id != supplier_id {
            return Err(AppError::business_displayable(
                "该联系人不属于此供应商，无法更新，请刷新后重试",
            ));
        }

        let owner_supplier_id = contact.supplier_id;
        let mut contact_active: supplier_contact::ActiveModel = contact.into();

        // 如果设置为主要联系人，先将其他联系人取消主要联系人状态
        if let Some(true) = req.is_primary {
            self.clear_primary_contacts_txn(owner_supplier_id, &txn)
                .await?;
        }

        if let Some(name) = req.contact_name {
            contact_active.contact_name = Set(name);
        }
        if let Some(department) = req.department {
            contact_active.department = Set(Some(department));
        }
        if let Some(position) = req.position {
            contact_active.position = Set(Some(position));
        }
        if let Some(mobile_phone) = req.mobile_phone {
            contact_active.mobile_phone = Set(mobile_phone);
        }
        if let Some(tel_phone) = req.tel_phone {
            contact_active.tel_phone = Set(Some(tel_phone));
        }
        if let Some(email) = req.email {
            contact_active.email = Set(Some(email));
        }
        if let Some(wechat) = req.wechat {
            contact_active.wechat = Set(Some(wechat));
        }
        if let Some(qq) = req.qq {
            contact_active.qq = Set(Some(qq));
        }
        if let Some(is_primary) = req.is_primary {
            contact_active.is_primary = Set(is_primary);
        }
        if let Some(remarks) = req.remarks {
            contact_active.remarks = Set(Some(remarks));
        }
        contact_active.updated_at = Set(Utc::now().into());

        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            contact_active,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;
        Ok(updated)
    }

    /// 删除供应商联系人
    /// 越权修复：路径 (supplier_id, contact_id) 必须一致——联系人不属于路径供应商时
    /// 返回用户可见业务错误（business_displayable），防止以 contact_id 单键跨供应商删除他人联系人。
    pub async fn delete_supplier_contact(
        &self,
        supplier_id: i32,
        contact_id: i32,
        user_id: i32,
    ) -> Result<(), AppError> {
        // get + delete 在同一事务内，lock_exclusive 串行化并发删除
        let txn = (*self.db).begin().await?;

        // lock_exclusive 串行化并发删除；锁持有至 txn 提交，model 用于持锁、
        // 存在性校验与归属校验
        let contact = supplier_contact::Entity::find_by_id(contact_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("联系人 {} 不存在", contact_id)))?;

        if contact.supplier_id != supplier_id {
            return Err(AppError::business_displayable(
                "该联系人不属于此供应商，无法删除，请刷新后重试",
            ));
        }

        // 删除联系人（含审计日志）
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            supplier_contact::Entity,
            _,
        >(&txn, "supplier_contact", contact_id, Some(user_id))
        .await?;

        txn.commit().await?;
        Ok(())
    }

    /// 取消指定供应商的所有主联系人状态（事务内）
    /// P2-6 修复（v12 复审）：原 `clear_primary_contacts` 使用 `&*self.db`（非事务连接），；且循环内调用 `update_with_audit`（每次 = 1 UPDATE + 1 audit INSERT = 2N 次写）。；新 `clear_primary_contacts_txn` 接受 `txn` 参数，循环内仅 `update(txn)`，；防止审计日志膨胀（参照 customer_service::clear_primary_contacts_txn）。
    async fn clear_primary_contacts_txn(
        &self,
        supplier_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        let primary_contacts = supplier_contact::Entity::find()
            .filter(supplier_contact::Column::SupplierId.eq(supplier_id))
            .filter(supplier_contact::Column::IsPrimary.eq(true))
            .all(txn)
            .await?;

        for contact in primary_contacts {
            let mut active: supplier_contact::ActiveModel = contact.into();
            active.is_primary = Set(false);
            active.updated_at = Set(Utc::now().into());
            // 取消主联系人状态属内部状态调整，不走 update_with_audit 防止审计日志膨胀
            active.update(txn).await?;
        }

        Ok(())
    }

    // ==================== 供应商资质管理方法 ====================

    /// is_expired 的唯一派生源：valid_until（含当日）之后视为过期。
    /// 写入时落真实值；读取输出时同样按此重算，保证列表/详情的过期语义与 valid_until 一致，
    /// 避免出现"永不更新的僵尸布尔列"（历史存量行无需回填迁移即可输出正确语义）。
    /// pub(crate)：资质过期门控（services/supplier_qualification_gate.rs）必须复用同一派生源，
    /// 不允许在别处再写第二套日期比较。
    pub(crate) fn qualification_is_expired(valid_until: NaiveDate) -> bool {
        chrono::Utc::now().date_naive() > valid_until
    }

    /// 资质日期交叉校验：有效期至不得早于发证日期（脏数据拒绝入库，文案外显）。
    /// 使用 business_displayable：仅涉及用户自己提交的日期，满足脱敏安全边界。
    fn check_qualification_dates(
        issue_date: NaiveDate,
        valid_until: NaiveDate,
    ) -> Result<(), AppError> {
        if valid_until < issue_date {
            return Err(AppError::business_displayable(
                "资质「有效期至」不能早于发证日期",
            ));
        }
        Ok(())
    }

    /// 获取供应商资质列表
    /// 批次 118 P2-9 修复：移除 `#[allow(dead_code)]` 标记，；handler 已真实接入（supplier_handler.rs::list_supplier_qualifications）。
    pub async fn list_supplier_qualifications(
        &self,
        supplier_id: i32,
    ) -> Result<Vec<supplier_qualification::Model>, AppError> {
        let qualifications = supplier_qualification::Entity::find()
            .filter(supplier_qualification::Column::SupplierId.eq(supplier_id))
            .order_by(supplier_qualification::Column::ValidUntil, Order::Asc)
            .all(&*self.db)
            .await?;
        // 读取输出按 valid_until 重算 is_expired，语义与 valid_until 保持一致
        Ok(qualifications
            .into_iter()
            .map(|mut q| {
                q.is_expired = Self::qualification_is_expired(q.valid_until);
                q
            })
            .collect())
    }

    /// 创建供应商资质
    pub async fn create_supplier_qualification(
        &self,
        supplier_id: i32,
        req: CreateQualificationRequest,
        _user_id: i32,
    ) -> Result<supplier_qualification::Model, AppError> {
        Self::check_qualification_dates(req.issue_date, req.valid_until)?;
        let qualification = supplier_qualification::ActiveModel {
            supplier_id: Set(supplier_id),
            qualification_name: Set(req.qualification_name),
            qualification_type: Set(req.qualification_type),
            qualification_no: Set(req.qualification_no),
            issuing_authority: Set(req.issuing_authority),
            issue_date: Set(req.issue_date),
            valid_until: Set(req.valid_until),
            attachment_path: Set(req.attachment_path),
            need_annual_check: Set(req.need_annual_check),
            annual_check_record: Set(req.annual_check_record),
            // is_expired 由 valid_until 与当前日期如实派生，不再依赖 DB 默认 false
            is_expired: Set(Self::qualification_is_expired(req.valid_until)),
            ..Default::default()
        }
        .insert(&*self.db)
        .await?;

        Ok(qualification)
    }

    /// 更新供应商资质
    /// 越权修复：路径 (supplier_id, qualification_id) 必须一致——资质不属于路径供应商时
    /// 返回用户可见业务错误（business_displayable，business 会被出参脱敏），防止单键枚举改写他人资质。
    pub async fn update_supplier_qualification(
        &self,
        supplier_id: i32,
        qualification_id: i32,
        req: CreateQualificationRequest,
    ) -> Result<supplier_qualification::Model, AppError> {
        Self::check_qualification_dates(req.issue_date, req.valid_until)?;
        let existing = supplier_qualification::Entity::find_by_id(qualification_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("资质 ID {} 不存在", qualification_id)))?;
        if existing.supplier_id != supplier_id {
            return Err(AppError::business_displayable(
                "该资质不属于此供应商，无法更新，请刷新后重试",
            ));
        }
        let mut qual: supplier_qualification::ActiveModel = existing.into();
        qual.qualification_name = Set(req.qualification_name);
        qual.qualification_type = Set(req.qualification_type);
        qual.qualification_no = Set(req.qualification_no);
        qual.issuing_authority = Set(req.issuing_authority);
        qual.issue_date = Set(req.issue_date);
        qual.valid_until = Set(req.valid_until);
        qual.attachment_path = Set(req.attachment_path);
        qual.need_annual_check = Set(req.need_annual_check);
        qual.annual_check_record = Set(req.annual_check_record);
        // update 重算 is_expired：valid_until 变更后过期语义同步
        qual.is_expired = Set(Self::qualification_is_expired(req.valid_until));
        Ok(qual.update(&*self.db).await?)
    }

    /// 删除供应商资质
    /// 越权修复：与 update 同源，先校验资质归属再删除。
    pub async fn delete_supplier_qualification(
        &self,
        supplier_id: i32,
        qualification_id: i32,
    ) -> Result<(), AppError> {
        let existing = supplier_qualification::Entity::find_by_id(qualification_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("资质 ID {} 不存在", qualification_id)))?;
        if existing.supplier_id != supplier_id {
            return Err(AppError::business_displayable(
                "该资质不属于此供应商，无法删除，请刷新后重试",
            ));
        }
        supplier_qualification::Entity::delete_by_id(qualification_id)
            .exec(&*self.db)
            .await?;
        Ok(())
    }

    /// attachment_path 列宽上限（DDL VARCHAR(500)）：落库前必须应用层显式拒绝越界，
    /// 不允许把越界留给 DB 层报裸 500。
    pub const MAX_ATTACHMENT_PATH_LEN: usize = 500;

    /// 按路径 (supplier_id, qualification_id) 双键读取资质。
    /// 与 update/delete 同源的归属校验：错配返回用户可见业务错误（business 会被出参脱敏），
    /// 防止以 qualification_id 单键跨供应商上传/读取他人证照附件。供附件上传/下载 handler 复用。
    pub async fn get_supplier_qualification(
        &self,
        supplier_id: i32,
        qualification_id: i32,
    ) -> Result<supplier_qualification::Model, AppError> {
        let existing = supplier_qualification::Entity::find_by_id(qualification_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("资质 ID {} 不存在", qualification_id)))?;
        if existing.supplier_id != supplier_id {
            return Err(AppError::business_displayable(
                "该资质不属于此供应商，无法操作附件，请刷新后重试",
            ));
        }
        Ok(existing)
    }

    /// 附件上传成功后仅回写 attachment_path（服务端生成的受控 URL），
    /// 不做全字段覆盖，避免上传动作把资质其他字段意外重置。
    /// 列宽 VARCHAR(500)：越界在应用层显式拒绝（business_displayable 外显真实原因，不裸 500）。
    pub async fn update_supplier_qualification_attachment_path(
        &self,
        qualification_id: i32,
        attachment_url: &str,
    ) -> Result<supplier_qualification::Model, AppError> {
        if attachment_url.len() > Self::MAX_ATTACHMENT_PATH_LEN {
            return Err(AppError::business_displayable(format!(
                "生成的附件访问地址超过 {} 字符上限，请联系管理员检查部署配置",
                Self::MAX_ATTACHMENT_PATH_LEN
            )));
        }
        let existing = supplier_qualification::Entity::find_by_id(qualification_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("资质 ID {} 不存在", qualification_id)))?;
        let mut qual: supplier_qualification::ActiveModel = existing.into();
        qual.attachment_path = Set(Some(attachment_url.to_string()));
        Ok(qual.update(&*self.db).await?)
    }
}

/// 创建供应商请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateSupplierRequest {
    #[validate(length(min = 2, max = 200, message = "供应商名称长度必须在2到200个字符之间"))]
    pub supplier_name: String,
    #[validate(length(min = 2, max = 100, message = "供应商简称长度必须在2到100个字符之间"))]
    pub supplier_short_name: Option<String>,
    pub supplier_type: Option<String>,
    #[validate(length(equal = 18, message = "统一社会信用代码长度必须为18位"))]
    pub credit_code: Option<String>,
    pub registered_address: Option<String>,
    pub business_address: Option<String>,
    pub legal_representative: Option<String>,
    pub registered_capital: Option<Decimal>,
    pub establishment_date: Option<NaiveDate>,
    pub business_term: Option<String>,
    pub business_scope: Option<String>,
    pub taxpayer_type: Option<String>,
    pub bank_name: Option<String>,
    pub bank_account: Option<String>,
    pub contact_phone: Option<String>,
    pub fax: Option<String>,
    pub website: Option<String>,
    pub email: Option<String>,
    pub main_business: Option<String>,
    pub main_market: Option<String>,
    pub employee_count: Option<i32>,
    pub annual_revenue: Option<Decimal>,
    pub category_id: Option<i32>,
    pub is_processor: Option<bool>,
    pub processor_type: Option<String>,
    /// 备注（DDL TEXT NULL）。与 `UpdateSupplierRequest.remarks` 同名同类型：
    /// 创建与更新的字段集合必须对称，否则前端建档弹窗填的备注会在建单时被静默丢弃。
    pub remarks: Option<String>,
    #[validate(nested)]
    pub contacts: Option<Vec<CreateContactRequest>>,
    #[validate(nested)]
    pub qualifications: Option<Vec<CreateQualificationRequest>>,
}

/// 更新供应商请求
#[derive(Debug, Deserialize)]
pub struct UpdateSupplierRequest {
    pub supplier_name: Option<String>,
    pub supplier_short_name: Option<String>,
    pub supplier_type: Option<String>,
    pub credit_code: Option<String>,
    pub registered_address: Option<String>,
    pub business_address: Option<String>,
    pub legal_representative: Option<String>,
    pub registered_capital: Option<Decimal>,
    pub establishment_date: Option<NaiveDate>,
    pub business_term: Option<String>,
    pub business_scope: Option<String>,
    pub taxpayer_type: Option<String>,
    pub bank_name: Option<String>,
    pub bank_account: Option<String>,
    pub contact_phone: Option<String>,
    pub fax: Option<String>,
    pub website: Option<String>,
    pub email: Option<String>,
    pub main_business: Option<String>,
    pub main_market: Option<String>,
    pub employee_count: Option<i32>,
    pub annual_revenue: Option<Decimal>,
    pub category_id: Option<i32>,
    pub is_processor: Option<bool>,
    pub processor_type: Option<String>,
    pub grade: Option<String>,
    pub grade_score: Option<Decimal>,
    pub status: Option<String>,
    pub is_enabled: Option<bool>,
    pub remarks: Option<String>,
}

/// 创建联系人请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateContactRequest {
    #[validate(length(min = 1, max = 50, message = "联系人名称长度必须在1到50个字符之间"))]
    pub contact_name: String,
    pub department: Option<String>,
    pub position: Option<String>,
    #[validate(custom(function = "validate_mobile_phone", message = "手机号格式不正确"))]
    pub mobile_phone: String,
    pub tel_phone: Option<String>,
    pub email: Option<String>,
    pub wechat: Option<String>,
    pub qq: Option<String>,
    pub is_primary: bool,
    pub remarks: Option<String>,
}

fn validate_mobile_phone(phone: &str) -> Result<(), validator::ValidationError> {
    // P2 1-8 修复：使用全局编译的 LazyLock<Regex>，避免每次调用都编译正则
    // 批次 404 修复：正则编译失败时优雅降级（返回验证失败而非 panic）
    if MOBILE_PHONE_RE
        .as_ref()
        .is_some_and(|re| re.is_match(phone))
    {
        Ok(())
    } else {
        Err(validator::ValidationError::new("mobile_phone"))
    }
}

/// 更新联系人请求
#[derive(Debug, Deserialize)]
pub struct UpdateContactRequest {
    pub contact_name: Option<String>,
    pub department: Option<String>,
    pub position: Option<String>,
    pub mobile_phone: Option<String>,
    pub tel_phone: Option<String>,
    pub email: Option<String>,
    pub wechat: Option<String>,
    pub qq: Option<String>,
    pub is_primary: Option<bool>,
    pub remarks: Option<String>,
}

/// 创建资质请求
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct CreateQualificationRequest {
    #[validate(length(min = 1, max = 200, message = "资质名称长度必须在1到200个字符之间"))]
    pub qualification_name: String,
    pub qualification_type: String,
    pub qualification_no: String,
    pub issuing_authority: String,
    pub issue_date: NaiveDate,
    pub valid_until: NaiveDate,
    pub attachment_path: Option<String>,
    pub need_annual_check: bool,
    pub annual_check_record: Option<String>,
}

/// 供应商查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct SupplierQueryParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub supplier_type: Option<String>,
    pub grade: Option<String>,
    pub status: Option<String>,
    pub keyword: Option<String>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub is_enabled: Option<bool>,
    pub category_id: Option<i32>,
    pub is_processor: Option<bool>,
    pub processor_type: Option<String>,
    /// 敏感导出 fail-closed：导出审批令牌
    pub download_token: Option<String>,
}

/// batch-13 P2：供应商账户余额
#[derive(Debug, Serialize)]
pub struct SupplierBalance {
    pub supplier_id: i32,
    pub supplier_name: String,
    pub total_amount: Decimal,
    pub paid_amount: Decimal,
    pub balance: Decimal,
    pub order_count: i64,
}

/// batch-13 P2：异常大额订单检测结果
#[derive(Debug, Serialize)]
pub struct AbnormalOrderDetection {
    pub order_id: i32,
    pub order_no: String,
    pub supplier_id: i32,
    pub supplier_name: String,
    pub order_amount: Decimal,
    pub average_amount: Decimal,
    pub deviation_ratio: Decimal,
    pub abnormal_type: String,
}

impl SupplierService {
    /// batch-13 P2：查询供应商账户余额
    pub async fn get_supplier_balance(
        &self,
        supplier_id: i32,
    ) -> Result<SupplierBalance, AppError> {
        use crate::models::ap_payment::{self, Entity as ApPaymentEntity};
        use crate::models::purchase_order::{self, Entity as PurchaseOrderEntity};

        // 查询供应商信息
        let supplier = supplier::Entity::find_by_id(supplier_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("供应商 {} 不存在", supplier_id)))?;

        // 查询采购订单总金额
        let orders = PurchaseOrderEntity::find()
            .filter(purchase_order::Column::SupplierId.eq(supplier_id))
            .all(&*self.db)
            .await?;

        let total_amount: Decimal = orders.iter().map(|o| o.total_amount).sum();
        let order_count = orders.len() as i64;

        // 查询已付款金额
        let payments = ApPaymentEntity::find()
            .filter(ap_payment::Column::SupplierId.eq(supplier_id))
            .all(&*self.db)
            .await?;

        let paid_amount: Decimal = payments.iter().map(|p| p.payment_amount).sum();
        let balance = total_amount - paid_amount;

        Ok(SupplierBalance {
            supplier_id,
            supplier_name: supplier.supplier_name,
            total_amount,
            paid_amount,
            balance,
            order_count,
        })
    }

    /// batch-13 P2：检测异常大额订单
    pub async fn detect_abnormal_orders(
        &self,
        threshold_ratio: Decimal,
    ) -> Result<Vec<AbnormalOrderDetection>, AppError> {
        use crate::models::purchase_order::{self, Entity as PurchaseOrderEntity};

        // 查询所有供应商
        let suppliers = supplier::Entity::find().all(&*self.db).await?;

        let mut abnormal_orders = Vec::new();

        for supplier in suppliers {
            // 查询该供应商的所有订单
            let orders = PurchaseOrderEntity::find()
                .filter(purchase_order::Column::SupplierId.eq(supplier.id))
                .all(&*self.db)
                .await?;

            if orders.is_empty() {
                continue;
            }

            // 计算平均订单金额
            let total: Decimal = orders.iter().map(|o| o.total_amount).sum();
            let average = total / Decimal::from(orders.len());

            // 检测异常大额订单
            for order in &orders {
                if average > Decimal::ZERO {
                    let ratio = order.total_amount / average;
                    if ratio > threshold_ratio {
                        abnormal_orders.push(AbnormalOrderDetection {
                            order_id: order.id,
                            order_no: order.order_no.clone(),
                            supplier_id: supplier.id,
                            supplier_name: supplier.supplier_name.clone(),
                            order_amount: order.total_amount,
                            average_amount: average,
                            deviation_ratio: ratio,
                            abnormal_type: "大额订单".to_string(),
                        });
                    }
                }
            }
        }

        Ok(abnormal_orders)
    }

    /// batch-13 P3: 供货历史查询
    pub async fn get_supplier_purchase_history(
        &self,
        supplier_id: i32,
        limit: Option<u64>,
    ) -> Result<Vec<PurchaseHistoryItem>, AppError> {
        use crate::models::purchase_order::{self, Entity as PurchaseOrderEntity};

        let limit = std::cmp::Ord::min(limit.unwrap_or(50), 200);

        let orders = PurchaseOrderEntity::find()
            .filter(purchase_order::Column::SupplierId.eq(supplier_id))
            .order_by_desc(purchase_order::Column::OrderDate)
            .limit(limit)
            .all(&*self.db)
            .await?;

        let history: Vec<PurchaseHistoryItem> = orders
            .into_iter()
            .map(|o| PurchaseHistoryItem {
                order_id: o.id,
                order_no: o.order_no,
                order_date: o.order_date.to_string(),
                total_amount: o.total_amount,
                status: o.order_status,
                item_count: 0, // TODO: 查询明细数量
            })
            .collect();

        Ok(history)
    }
}

/// 供货历史记录
#[derive(Debug, Serialize)]
pub struct PurchaseHistoryItem {
    pub order_id: i32,
    pub order_no: String,
    pub order_date: String,
    pub total_amount: Decimal,
    pub status: String,
    pub item_count: i32,
}
