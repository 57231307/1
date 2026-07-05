// 批次 101 v6 复审 P2 修复：update_customer / delete_customer 改为事务 + lock_exclusive +
// update_with_audit，补全审计日志与 TOCTOU 防护（P2-1 / P2-2）。

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, Order,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use std::sync::Arc;

use crate::models::customer::{self, Entity as CustomerEntity};
use crate::models::dto::PageRequest;
use crate::search::{CustomerDoc, SearchClient, SearchSyncer};
use crate::utils::data_permission::{DataPermissionFilter, CUSTOMER_ALL_FIELDS};
use crate::utils::error::AppError;
use crate::utils::PaginatedResponse;

/// 将字段名映射到客户实体的列枚举
///
/// 用于数据库层面的字段选择，将字符串字段名转换为 SeaORM 的 Column 枚举
fn select_customer_column(
    query: sea_orm::Select<CustomerEntity>,
    field: &str,
) -> sea_orm::Select<CustomerEntity> {
    use customer::Column;
    match field {
        "id" => query.column(Column::Id),
        "customer_code" => query.column(Column::CustomerCode),
        "customer_name" => query.column(Column::CustomerName),
        "contact_person" => query.column(Column::ContactPerson),
        "contact_phone" => query.column(Column::ContactPhone),
        "contact_email" => query.column(Column::ContactEmail),
        "address" => query.column(Column::Address),
        "city" => query.column(Column::City),
        "province" => query.column(Column::Province),
        "country" => query.column(Column::Country),
        "postal_code" => query.column(Column::PostalCode),
        "credit_limit" => query.column(Column::CreditLimit),
        "payment_terms" => query.column(Column::PaymentTerms),
        "tax_id" => query.column(Column::TaxId),
        "bank_name" => query.column(Column::BankName),
        "bank_account" => query.column(Column::BankAccount),
        "status" => query.column(Column::Status),
        "customer_type" => query.column(Column::CustomerType),
        "notes" => query.column(Column::Notes),
        "created_by" => query.column(Column::CreatedBy),
        "created_at" => query.column(Column::CreatedAt),
        "updated_at" => query.column(Column::UpdatedAt),
        "customer_industry" => query.column(Column::CustomerIndustry),
        "main_products" => query.column(Column::MainProducts),
        "annual_purchase" => query.column(Column::AnnualPurchase),
        "quality_requirement" => query.column(Column::QualityRequirement),
        "inspection_standard" => query.column(Column::InspectionStandard),
        _ => query,
    }
}

/// 根据数据权限过滤器构建只选择指定字段的查询
fn build_select_only_query(
    query: sea_orm::Select<CustomerEntity>,
    filter: &DataPermissionFilter,
) -> sea_orm::Select<CustomerEntity> {
    let select_fields = filter.get_select_fields(CUSTOMER_ALL_FIELDS);
    let mut select_query = query.select_only();
    for field in &select_fields {
        select_query = select_customer_column(select_query, field);
    }
    select_query
}

/// 客户服务
///
/// 批次 124 v8 复审 P1 修复：注入 search_syncer 实现 PG→ES 写入同步。
/// - create/update/delete 事务提交后调用 sync_customer 将最新数据同步到 ES
/// - ES 同步失败仅记录 tracing::warn!（最终一致性），不回滚 PG 事务
/// - mock 模式（CI 环境）下同步到内存 HashMap，real 模式同步到真实 ES
pub struct CustomerService {
    db: Arc<DatabaseConnection>,
    /// ES 同步器（PG→ES 写入同步），批次 124 接入
    search_syncer: Arc<SearchSyncer>,
}

impl CustomerService {
    pub fn new(db: Arc<DatabaseConnection>, search_client: Arc<dyn SearchClient>) -> Self {
        Self {
            db,
            search_syncer: Arc::new(SearchSyncer::new(search_client)),
        }
    }

    /// 将 customer::Model 转换为 CustomerDoc 用于 ES 索引
    ///
    /// 批次 124 v8 复审 P1 修复：字段映射规则
    /// - tier 映射 customer_type（retail/wholesale/vip，更接近"等级"语义）
    /// - 其余字段直接从 Model 取值
    fn build_customer_doc(model: &customer::Model) -> CustomerDoc {
        CustomerDoc {
            id: model.id,
            code: model.customer_code.clone(),
            name: model.customer_name.clone(),
            contact_person: model.contact_person.clone(),
            phone: model.contact_phone.clone(),
            email: model.contact_email.clone(),
            address: model.address.clone(),
            tier: model.customer_type.clone(),
        }
    }

    /// 同步客户到 ES（最终一致性策略）
    ///
    /// 批次 124 v8 复审 P1 修复：ES 同步失败仅记录日志，不回滚 PG 事务。
    /// 设计原因：PG 是主数据源（事务、关联查询），ES 是搜索副本（全文搜索）。
    /// ES 同步失败时 PG 数据已正确写入，后续可通过定时补偿任务修复 ES 缺失。
    async fn sync_customer_to_es(&self, model: &customer::Model, operation: &str) {
        let doc = Self::build_customer_doc(model);
        if let Err(e) = self.search_syncer.sync_customer(&doc).await {
            tracing::warn!(
                error = %e,
                customer_id = model.id,
                customer_code = %model.customer_code,
                operation = operation,
                "ES 客户同步失败（PG 已提交，最终一致性靠补偿任务修复）"
            );
        }
    }

    /// 生成客户编码
    pub async fn generate_customer_code(&self) -> Result<String, crate::utils::error::AppError> {
        use crate::utils::number_generator::DocumentNumberGenerator;
        DocumentNumberGenerator::generate_no(
            &*self.db,
            "CUS",
            customer::Entity,
            customer::Column::CustomerCode,
        )
        .await
    }

    /// 创建客户
    ///
    /// 批次 85 v2 复审 P1-6 修复：检查编码存在 + insert 移入单一事务 + lock_exclusive 串行化
    /// 原实现检查编码和 insert 在 self.db 上分别执行，无 txn 无 lock，并发创建相同编码的客户会通过检查后重复插入
    #[allow(clippy::too_many_arguments)]
    pub async fn create_customer(
        &self,
        customer_code: String,
        customer_name: String,
        contact_person: Option<String>,
        contact_phone: Option<String>,
        contact_email: Option<String>,
        address: Option<String>,
        city: Option<String>,
        province: Option<String>,
        country: Option<String>,
        postal_code: Option<String>,
        credit_limit: rust_decimal::Decimal,
        payment_terms: i32,
        tax_id: Option<String>,
        bank_name: Option<String>,
        bank_account: Option<String>,
        customer_type: String,
        notes: Option<String>,
        created_by: Option<i32>,
    ) -> Result<customer::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 检查客户编码是否已存在（加 lock_exclusive 防止并发创建相同编码）
        let existing = CustomerEntity::find()
            .filter(customer::Column::CustomerCode.eq(&customer_code))
            .lock_exclusive()
            .one(&txn)
            .await?;

        if existing.is_some() {
            return Err(AppError::business("客户编码已存在"));
        }

        let customer = customer::ActiveModel {
            id: Default::default(),
            customer_code: sea_orm::ActiveValue::Set(customer_code),
            customer_name: sea_orm::ActiveValue::Set(customer_name),
            contact_person: sea_orm::ActiveValue::Set(contact_person),
            contact_phone: sea_orm::ActiveValue::Set(contact_phone),
            contact_email: sea_orm::ActiveValue::Set(contact_email),
            address: sea_orm::ActiveValue::Set(address),
            city: sea_orm::ActiveValue::Set(city),
            province: sea_orm::ActiveValue::Set(province),
            country: sea_orm::ActiveValue::Set(Some(country.unwrap_or_else(|| "中国".to_string()))),
            postal_code: sea_orm::ActiveValue::Set(postal_code),
            credit_limit: sea_orm::ActiveValue::Set(credit_limit),
            payment_terms: sea_orm::ActiveValue::Set(payment_terms),
            tax_id: sea_orm::ActiveValue::Set(tax_id),
            bank_name: sea_orm::ActiveValue::Set(bank_name),
            bank_account: sea_orm::ActiveValue::Set(bank_account),
            status: sea_orm::ActiveValue::Set("active".to_string()),
            customer_type: sea_orm::ActiveValue::Set(customer_type),
            notes: sea_orm::ActiveValue::Set(notes),
            created_by: sea_orm::ActiveValue::Set(created_by),
            created_at: sea_orm::ActiveValue::Set(Utc::now()),
            updated_at: sea_orm::ActiveValue::Set(Utc::now()),
            customer_industry: sea_orm::ActiveValue::NotSet,
            main_products: sea_orm::ActiveValue::NotSet,
            annual_purchase: sea_orm::ActiveValue::NotSet,
            quality_requirement: sea_orm::ActiveValue::NotSet,
            inspection_standard: sea_orm::ActiveValue::NotSet,
        };

        let result = customer.insert(&txn).await.map_err(AppError::from)?;
        txn.commit().await?;

        // 批次 124 v8 复审 P1 修复：PG 事务提交后同步到 ES（最终一致性）
        // ES 同步失败仅记录日志，不回滚 PG（PG 是主数据源，ES 是搜索副本）
        self.sync_customer_to_es(&result, "create").await;

        Ok(result)
    }

    /// 获取客户详情
    pub async fn get_customer(&self, customer_id: i32) -> Result<customer::Model, AppError> {
        CustomerEntity::find_by_id(customer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 未找到", customer_id)))
    }

    /// 获取客户列表
    #[allow(dead_code)] // TODO(tech-debt): CRM 客户模块统一迁移后接入，或删除
    pub async fn list_customers(
        &self,
        page_req: PageRequest,
        status: Option<String>,
        customer_type: Option<String>,
        keyword: Option<String>,
    ) -> Result<PaginatedResponse<customer::Model>, AppError> {
        let mut query = CustomerEntity::find();

        // 状态筛选
        if let Some(status) = status {
            query = query.filter(customer::Column::Status.eq(status));
        }

        // 客户类型筛选
        if let Some(customer_type) = customer_type {
            query = query.filter(customer::Column::CustomerType.eq(customer_type));
        }

        // 关键词搜索
        if let Some(keyword) = keyword {
            query = query.filter(
                customer::Column::CustomerName
                    .contains(&keyword)
                    .or(customer::Column::CustomerCode.contains(&keyword)),
            );
        }

        // 总数
        let total = query.clone().count(&*self.db).await?;

        // 分页排序
        let offset = page_req.page.saturating_sub(1) * page_req.page_size;
        let customers = query
            .order_by(customer::Column::CreatedAt, Order::Desc)
            .offset(offset)
            .limit(page_req.page_size)
            .all(&*self.db)
            .await?;

        Ok(PaginatedResponse::new(
            customers,
            total,
            page_req.page,
            page_req.page_size,
        ))
    }

    /// 获取客户列表（带数据权限过滤）
    ///
    /// # 参数
    /// - `page_req`: 分页参数
    /// - `status`: 状态筛选
    /// - `customer_type`: 客户类型筛选
    /// - `keyword`: 关键词搜索
    /// - `permission_filter`: 数据权限过滤器，用于在数据库层面过滤字段
    pub async fn list_customers_with_filter(
        &self,
        page_req: PageRequest,
        status: Option<String>,
        customer_type: Option<String>,
        keyword: Option<String>,
        permission_filter: Option<DataPermissionFilter>,
    ) -> Result<PaginatedResponse<serde_json::Value>, AppError> {
        let mut query = CustomerEntity::find();

        // 状态筛选
        if let Some(status) = status {
            query = query.filter(customer::Column::Status.eq(status));
        }

        // 客户类型筛选
        if let Some(customer_type) = customer_type {
            query = query.filter(customer::Column::CustomerType.eq(customer_type));
        }

        // 关键词搜索
        if let Some(keyword) = keyword {
            query = query.filter(
                customer::Column::CustomerName
                    .contains(&keyword)
                    .or(customer::Column::CustomerCode.contains(&keyword)),
            );
        }

        // 总数
        let total = query.clone().count(&*self.db).await?;

        // 分页排序
        let offset = page_req.page.saturating_sub(1) * page_req.page_size;

        // 根据数据权限过滤字段
        let customers = if let Some(filter) = permission_filter {
            // 使用辅助函数构建只选择指定字段的查询
            let select_query = build_select_only_query(query, &filter);

            let rows = select_query
                .order_by(customer::Column::CreatedAt, Order::Desc)
                .offset(offset)
                .limit(page_req.page_size)
                .into_json()
                .all(&*self.db)
                .await?;

            rows
        } else {
            // 没有过滤器，查询所有字段
            let rows = query
                .order_by(customer::Column::CreatedAt, Order::Desc)
                .offset(offset)
                .limit(page_req.page_size)
                .all(&*self.db)
                .await?;

            // 转换为 JSON 值
            rows.into_iter()
                .map(|c| {
                    serde_json::to_value(c)
                        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))
                })
                .collect::<Result<Vec<_>, _>>()?
        };

        Ok(PaginatedResponse::new(
            customers,
            total,
            page_req.page,
            page_req.page_size,
        ))
    }

    /// 获取客户详情（带数据权限过滤）
    ///
    /// # 参数
    /// - `customer_id`: 客户ID
    /// - `permission_filter`: 数据权限过滤器，用于在数据库层面过滤字段
    pub async fn get_customer_with_filter(
        &self,
        customer_id: i32,
        permission_filter: Option<DataPermissionFilter>,
    ) -> Result<serde_json::Value, AppError> {
        let query = CustomerEntity::find_by_id(customer_id);

        let customer = if let Some(filter) = permission_filter {
            // 使用辅助函数构建只选择指定字段的查询
            let select_query = build_select_only_query(query, &filter);

            select_query
                .into_json()
                .one(&*self.db)
                .await?
                .ok_or_else(|| AppError::not_found(format!("客户 {} 未找到", customer_id)))?
        } else {
            // 没有过滤器，查询所有字段
            let model = query
                .one(&*self.db)
                .await?
                .ok_or_else(|| AppError::not_found(format!("客户 {} 未找到", customer_id)))?;

            serde_json::to_value(model)
                .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?
        };

        Ok(customer)
    }

    /// 更新客户信息
    ///
    /// 批次 101 v6 复审 P2-1 修复：原实现裸查询 + 裸更新无事务、无 lock_exclusive、无审计日志，
    /// 并发更新会基于过期状态写入，且无审计追溯。改为事务内 lock_exclusive 串行化 +
    /// update_with_audit 落审计日志。
    #[allow(clippy::too_many_arguments)]
    pub async fn update_customer(
        &self,
        customer_id: i32,
        customer_name: Option<String>,
        contact_person: Option<String>,
        contact_phone: Option<String>,
        contact_email: Option<String>,
        address: Option<String>,
        city: Option<String>,
        province: Option<String>,
        postal_code: Option<String>,
        credit_limit: Option<rust_decimal::Decimal>,
        payment_terms: Option<i32>,
        tax_id: Option<String>,
        bank_name: Option<String>,
        bank_account: Option<String>,
        customer_type: Option<String>,
        status: Option<String>,
        notes: Option<String>,
        user_id: i32,
    ) -> Result<customer::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 加 lock_exclusive 串行化并发更新，防止 TOCTOU
        let customer = CustomerEntity::find_by_id(customer_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 未找到", customer_id)))?;

        let mut customer_update: customer::ActiveModel = customer.into();

        if let Some(customer_name) = customer_name {
            customer_update.customer_name = sea_orm::ActiveValue::Set(customer_name);
        }
        if let Some(contact_person) = contact_person {
            customer_update.contact_person = sea_orm::ActiveValue::Set(Some(contact_person));
        }
        if let Some(contact_phone) = contact_phone {
            customer_update.contact_phone = sea_orm::ActiveValue::Set(Some(contact_phone));
        }
        if let Some(contact_email) = contact_email {
            customer_update.contact_email = sea_orm::ActiveValue::Set(Some(contact_email));
        }
        if let Some(address) = address {
            customer_update.address = sea_orm::ActiveValue::Set(Some(address));
        }
        if let Some(city) = city {
            customer_update.city = sea_orm::ActiveValue::Set(Some(city));
        }
        if let Some(province) = province {
            customer_update.province = sea_orm::ActiveValue::Set(Some(province));
        }
        if let Some(postal_code) = postal_code {
            customer_update.postal_code = sea_orm::ActiveValue::Set(Some(postal_code));
        }
        if let Some(credit_limit) = credit_limit {
            customer_update.credit_limit = sea_orm::ActiveValue::Set(credit_limit);
        }
        if let Some(payment_terms) = payment_terms {
            customer_update.payment_terms = sea_orm::ActiveValue::Set(payment_terms);
        }
        if let Some(tax_id) = tax_id {
            customer_update.tax_id = sea_orm::ActiveValue::Set(Some(tax_id));
        }
        if let Some(bank_name) = bank_name {
            customer_update.bank_name = sea_orm::ActiveValue::Set(Some(bank_name));
        }
        if let Some(bank_account) = bank_account {
            customer_update.bank_account = sea_orm::ActiveValue::Set(Some(bank_account));
        }
        if let Some(customer_type) = customer_type {
            customer_update.customer_type = sea_orm::ActiveValue::Set(customer_type);
        }
        if let Some(status) = status {
            customer_update.status = sea_orm::ActiveValue::Set(status);
        }
        if let Some(notes) = notes {
            customer_update.notes = sea_orm::ActiveValue::Set(Some(notes));
        }

        customer_update.updated_at = sea_orm::ActiveValue::Set(Utc::now());

        // 事务内 update_with_audit，原子写入客户变更 + 审计日志
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "customer",
            customer_update,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        // 批次 124 v8 复审 P1 修复：PG 事务提交后同步到 ES（最终一致性）
        // 注意：软删除场景下 ES 文档仍保留（status 字段同步），便于搜索历史客户
        self.sync_customer_to_es(&updated, "update").await;

        Ok(updated)
    }

    /// 删除客户（软删除，将状态改为 inactive）
    ///
    /// 批次 101 v6 复审 P2-2 修复：原实现无事务、无 lock_exclusive、无状态门、无审计日志，
    /// 并发软删除会重复写审计日志缺失。改为事务内 lock_exclusive + 状态门（已 inactive 拒绝）+
    /// update_with_audit 落审计日志。
    pub async fn delete_customer(
        &self,
        customer_id: i32,
        user_id: i32,
    ) -> Result<customer::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 加 lock_exclusive 串行化并发软删除，防止 TOCTOU
        let customer = CustomerEntity::find_by_id(customer_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 未找到", customer_id)))?;

        // 状态门：已 inactive 的客户拒绝重复软删除
        if customer.status == "inactive" {
            return Err(AppError::business(format!(
                "客户 {} 已删除，无需重复操作",
                customer_id
            )));
        }

        let mut customer_update: customer::ActiveModel = customer.into();
        customer_update.status = sea_orm::ActiveValue::Set("inactive".to_string());
        customer_update.updated_at = sea_orm::ActiveValue::Set(Utc::now());

        // 事务内 update_with_audit，原子写入软删除 + 审计日志
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "customer",
            customer_update,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        // 批次 124 v8 复审 P1 修复：软删除后同步 status=inactive 到 ES
        // 设计决策：软删除不删除 ES 文档，保留便于搜索历史客户（status 字段已同步）
        self.sync_customer_to_es(&updated, "delete").await;

        Ok(updated)
    }

    // ==================== 客户联系人管理方法（批次 90b P2-12） ====================

    /// 获取客户联系人列表
    ///
    /// 主联系人排在最前，其余按姓名升序。补 LIMIT 兜底（与批次 87 LIMIT 模式一致）。
    pub async fn list_customer_contacts(
        &self,
        customer_id: i32,
    ) -> Result<Vec<crate::models::customer_contact::Model>, AppError> {
        use crate::models::customer_contact;

        let contacts = customer_contact::Entity::find()
            .filter(customer_contact::Column::CustomerId.eq(customer_id))
            .order_by(customer_contact::Column::IsPrimary, Order::Desc)
            .order_by(customer_contact::Column::Name, Order::Asc)
            .limit(10_000)
            .all(&*self.db)
            .await?;
        Ok(contacts)
    }

    /// 创建客户联系人
    ///
    /// 若 is_primary=true，事务内先将其他联系人取消主联系人状态，再插入新主联系人，
    /// 保证"每个客户最多一个主联系人"的部分唯一索引约束不被触发。
    pub async fn create_customer_contact(
        &self,
        customer_id: i32,
        req: CreateCustomerContactRequest,
        user_id: i32,
    ) -> Result<crate::models::customer_contact::Model, AppError> {
        use crate::models::customer_contact;

        let txn = (*self.db).begin().await?;

        // 若设置为主联系人，先将其他联系人取消主联系人状态
        if req.is_primary {
            self.clear_primary_contacts_txn(customer_id, &txn).await?;
        }

        let now = Utc::now();
        let contact = customer_contact::ActiveModel {
            customer_id: Set(customer_id),
            name: Set(req.name),
            title: Set(req.title),
            phone: Set(req.phone),
            email: Set(req.email),
            is_primary: Set(req.is_primary),
            remarks: Set(req.remarks),
            created_by: Set(Some(user_id)),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;
        Ok(contact)
    }

    /// 更新客户联系人
    ///
    /// 若 is_primary 由非主改为主，事务内先将其他联系人取消主联系人状态。
    pub async fn update_customer_contact(
        &self,
        contact_id: i32,
        req: UpdateCustomerContactRequest,
        user_id: i32,
    ) -> Result<crate::models::customer_contact::Model, AppError> {
        use crate::models::customer_contact;

        let txn = (*self.db).begin().await?;

        let contact = customer_contact::Entity::find_by_id(contact_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("联系人 {} 不存在", contact_id)))?;

        let customer_id = contact.customer_id;
        let mut contact_active: customer_contact::ActiveModel = contact.into();

        // 若设置为主联系人，先将其他联系人取消主联系人状态
        if let Some(true) = req.is_primary {
            self.clear_primary_contacts_txn(customer_id, &txn).await?;
        }

        if let Some(name) = req.name {
            contact_active.name = Set(name);
        }
        if let Some(title) = req.title {
            contact_active.title = Set(Some(title));
        }
        if let Some(phone) = req.phone {
            contact_active.phone = Set(phone);
        }
        if let Some(email) = req.email {
            contact_active.email = Set(Some(email));
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

    /// 删除客户联系人
    pub async fn delete_customer_contact(&self, contact_id: i32) -> Result<(), AppError> {
        use crate::models::customer_contact;

        let contact = customer_contact::Entity::find_by_id(contact_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("联系人 {} 不存在", contact_id)))?;
        contact.delete(&*self.db).await?;
        Ok(())
    }

    /// 取消指定客户的所有主联系人状态（事务内）
    ///
    /// 由 create/update 调用，保证"每个客户最多一个主联系人"约束。
    async fn clear_primary_contacts_txn(
        &self,
        customer_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        use crate::models::customer_contact;

        let primary_contacts = customer_contact::Entity::find()
            .filter(customer_contact::Column::CustomerId.eq(customer_id))
            .filter(customer_contact::Column::IsPrimary.eq(true))
            .all(txn)
            .await?;

        for contact in primary_contacts {
            let mut active: customer_contact::ActiveModel = contact.into();
            active.is_primary = Set(false);
            active.updated_at = Set(Utc::now().into());
            // P3 注：取消主联系人状态属内部状态调整，不走 update_with_audit
            // 防止审计日志膨胀（一次 create_contact 即产生 N 条审计记录）
            active.update(txn).await?;
        }

        Ok(())
    }

}

/// 创建客户联系人请求 DTO（批次 90b P2-12）
#[derive(Debug, serde::Deserialize, validator::Validate)]
pub struct CreateCustomerContactRequest {
    /// 联系人姓名：必填，长度 1-50
    #[validate(length(min = 1, max = 50, message = "联系人姓名长度必须在1到50个字符之间"))]
    pub name: String,
    /// 职务：可选，长度上限 100
    #[validate(length(max = 100, message = "职务长度不能超过100个字符"))]
    pub title: Option<String>,
    /// 联系电话：必填，长度 1-50（兼容手机/座机/国际号码，宽松校验）
    #[validate(length(min = 1, max = 50, message = "联系电话长度必须在1到50个字符之间"))]
    pub phone: String,
    /// 联系邮箱：可选，需符合邮箱格式
    #[validate(email(message = "邮箱格式不正确"))]
    pub email: Option<String>,
    /// 是否主要联系人：默认 false
    #[serde(default)]
    pub is_primary: bool,
    /// 备注：可选，长度上限 500
    #[validate(length(max = 500, message = "备注长度不能超过500个字符"))]
    pub remarks: Option<String>,
}

/// 更新客户联系人请求 DTO（批次 90b P2-12）
#[derive(Debug, serde::Deserialize, validator::Validate)]
pub struct UpdateCustomerContactRequest {
    /// 联系人姓名：可选
    #[validate(length(min = 1, max = 50, message = "联系人姓名长度必须在1到50个字符之间"))]
    pub name: Option<String>,
    /// 职务：可选
    #[validate(length(max = 100, message = "职务长度不能超过100个字符"))]
    pub title: Option<String>,
    /// 联系电话：可选
    #[validate(length(min = 1, max = 50, message = "联系电话长度必须在1到50个字符之间"))]
    pub phone: Option<String>,
    /// 联系邮箱：可选
    #[validate(email(message = "邮箱格式不正确"))]
    pub email: Option<String>,
    /// 是否主要联系人：可选
    pub is_primary: Option<bool>,
    /// 备注：可选
    #[validate(length(max = 500, message = "备注长度不能超过500个字符"))]
    pub remarks: Option<String>,
}
