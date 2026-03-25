use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, TransactionTrait, PaginatorTrait, Set, Order, ModelTrait,
};
use std::sync::Arc;
use crate::models::{supplier, supplier_contact, supplier_qualification};
use crate::utils::error::AppError;
use chrono::{Utc, NaiveDate};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

/// 供应商服务
pub struct SupplierService {
    db: Arc<DatabaseConnection>,
}

impl SupplierService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成供应商编码
    pub async fn generate_supplier_code(&self) -> Result<String, AppError> {
        let today = Utc::now().format("%Y%m%d").to_string();
        let prefix = format!("SUP{}", today);
        
        // 查询今日已有数量
        let count = supplier::Entity::find()
            .filter(supplier::Column::SupplierCode.starts_with(&prefix))
            .count(&*self.db)
            .await?;
        
        Ok(format!("{}{:03}", prefix, count + 1))
    }

    /// 创建供应商（含联系人和资质）
    pub async fn create_supplier(
        &self,
        req: CreateSupplierRequest,
        user_id: i32,
    ) -> Result<supplier::Model, AppError> {
        let txn = (&*self.db).begin().await?;
        
        // 1. 生成供应商编码
        let supplier_code = self.generate_supplier_code().await?;
        
        // 2. 创建供应商
        let supplier = supplier::ActiveModel {
            supplier_code: Set(supplier_code),
            supplier_name: Set(req.supplier_name),
            supplier_short_name: Set(req.supplier_short_name),
            supplier_type: Set(req.supplier_type),
            credit_code: Set(req.credit_code),
            registered_address: Set(req.registered_address),
            business_address: Set(req.business_address),
            legal_representative: Set(req.legal_representative),
            registered_capital: Set(req.registered_capital),
            establishment_date: Set(req.establishment_date),
            business_term: Set(req.business_term),
            business_scope: Set(req.business_scope),
            taxpayer_type: Set(req.taxpayer_type),
            bank_name: Set(req.bank_name),
            bank_account: Set(req.bank_account),
            contact_phone: Set(req.contact_phone),
            fax: Set(req.fax),
            website: Set(req.website),
            email: Set(req.email),
            main_business: Set(req.main_business),
            main_market: Set(req.main_market),
            employee_count: Set(req.employee_count),
            annual_revenue: Set(req.annual_revenue),
            created_by: Set(Some(user_id)),
            ..Default::default()
        }.insert(&txn).await?;
        
        // 3. 创建联系人
        for contact_req in req.contacts {
            supplier_contact::ActiveModel {
                supplier_id: Set(supplier.id),
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
            }.insert(&txn).await?;
        }
        
        // 4. 创建资质
        for qual_req in req.qualifications {
            supplier_qualification::ActiveModel {
                supplier_id: Set(supplier.id),
                qualification_name: Set(qual_req.qualification_name),
                qualification_type: Set(qual_req.qualification_type),
                qualification_no: Set(qual_req.qualification_no),
                issuing_authority: Set(qual_req.issuing_authority),
                issue_date: Set(qual_req.issue_date),
                valid_until: Set(qual_req.valid_until),
                attachment_path: Set(qual_req.attachment_path),
                need_annual_check: Set(qual_req.need_annual_check),
                annual_check_record: Set(qual_req.annual_check_record),
                ..Default::default()
            }.insert(&txn).await?;
        }
        
        txn.commit().await?;
        Ok(supplier)
    }

    /// 查询供应商列表（分页、筛选、排序）
    pub async fn list_suppliers(
        &self,
        params: SupplierQueryParams,
    ) -> Result<PaginatedResponse<supplier::Model>, AppError> {
        let mut query = supplier::Entity::find();
        
        // 筛选
        if let Some(supplier_type) = params.supplier_type {
            query = query.filter(supplier::Column::SupplierType.eq(supplier_type));
        }
        if let Some(grade) = params.grade {
            query = query.filter(supplier::Column::Grade.eq(grade));
        }
        if let Some(status) = params.status {
            query = query.filter(supplier::Column::Status.eq(status));
        }
        if let Some(keyword) = params.keyword {
            query = query.filter(
                supplier::Column::SupplierName.contains(&keyword)
                    .or(supplier::Column::SupplierCode.contains(&keyword))
                    .or(supplier::Column::CreditCode.contains(&keyword))
            );
        }
        if let Some(is_enabled) = params.is_enabled {
            query = query.filter(supplier::Column::IsEnabled.eq(is_enabled));
        }
        
        // 排序
        let sort_by = params.sort_by.unwrap_or_else(|| "created_at".to_string());
        let sort_order = params.sort_order.unwrap_or_else(|| "DESC".to_string());
        
        query = match sort_by.as_str() {
            "supplier_code" => {
                if sort_order == "ASC" {
                    query.order_by(supplier::Column::SupplierCode, Order::Asc)
                } else {
                    query.order_by(supplier::Column::SupplierCode, Order::Desc)
                }
            }
            "supplier_name" => {
                if sort_order == "ASC" {
                    query.order_by(supplier::Column::SupplierName, Order::Asc)
                } else {
                    query.order_by(supplier::Column::SupplierName, Order::Desc)
                }
            }
            "grade" => {
                if sort_order == "ASC" {
                    query.order_by(supplier::Column::Grade, Order::Asc)
                } else {
                    query.order_by(supplier::Column::Grade, Order::Desc)
                }
            }
            _ => {
                if sort_order == "ASC" {
                    query.order_by(supplier::Column::CreatedAt, Order::Asc)
                } else {
                    query.order_by(supplier::Column::CreatedAt, Order::Desc)
                }
            }
        };
        
        // 分页
        let page = params.page.unwrap_or(1);
        let page_size = params.page_size.unwrap_or(20);
        
        let paginator = query.paginate(&*self.db, page_size);
        let num_pages = paginator.num_pages().await?;
        let data = paginator.fetch_page(page - 1).await?;
        let total = num_pages as u64 * page_size;
        
        Ok(PaginatedResponse {
            data,
            page,
            page_size,
            total,
        })
    }

    /// 获取供应商详情
    pub async fn get_supplier(&self, id: i32) -> Result<supplier::Model, AppError> {
        supplier::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::NotFound(format!("供应商 {} 不存在", id)))
    }

    /// 更新供应商信息
    pub async fn update_supplier(
        &self,
        id: i32,
        req: UpdateSupplierRequest,
        user_id: i32,
    ) -> Result<supplier::Model, AppError> {
        let supplier = self.get_supplier(id).await?;
        let mut supplier_active: supplier::ActiveModel = supplier.into();
        
        // 更新字段
        if let Some(name) = req.supplier_name {
            supplier_active.supplier_name = Set(name);
        }
        if let Some(short_name) = req.supplier_short_name {
            supplier_active.supplier_short_name = Set(short_name);
        }
        if let Some(supplier_type) = req.supplier_type {
            supplier_active.supplier_type = Set(supplier_type);
        }
        if let Some(credit_code) = req.credit_code {
            supplier_active.credit_code = Set(credit_code);
        }
        if let Some(registered_address) = req.registered_address {
            supplier_active.registered_address = Set(registered_address);
        }
        if let Some(business_address) = req.business_address {
            supplier_active.business_address = Set(Some(business_address));
        }
        if let Some(legal_representative) = req.legal_representative {
            supplier_active.legal_representative = Set(legal_representative);
        }
        if let Some(registered_capital) = req.registered_capital {
            supplier_active.registered_capital = Set(registered_capital);
        }
        if let Some(establishment_date) = req.establishment_date {
            supplier_active.establishment_date = Set(establishment_date);
        }
        if let Some(business_term) = req.business_term {
            supplier_active.business_term = Set(Some(business_term));
        }
        if let Some(business_scope) = req.business_scope {
            supplier_active.business_scope = Set(Some(business_scope));
        }
        if let Some(taxpayer_type) = req.taxpayer_type {
            supplier_active.taxpayer_type = Set(taxpayer_type);
        }
        if let Some(bank_name) = req.bank_name {
            supplier_active.bank_name = Set(bank_name);
        }
        if let Some(bank_account) = req.bank_account {
            supplier_active.bank_account = Set(bank_account);
        }
        if let Some(contact_phone) = req.contact_phone {
            supplier_active.contact_phone = Set(contact_phone);
        }
        if let Some(fax) = req.fax {
            supplier_active.fax = Set(Some(fax));
        }
        if let Some(website) = req.website {
            supplier_active.website = Set(Some(website));
        }
        if let Some(email) = req.email {
            supplier_active.email = Set(Some(email));
        }
        if let Some(main_business) = req.main_business {
            supplier_active.main_business = Set(Some(main_business));
        }
        if let Some(main_market) = req.main_market {
            supplier_active.main_market = Set(Some(main_market));
        }
        if let Some(employee_count) = req.employee_count {
            supplier_active.employee_count = Set(Some(employee_count));
        }
        if let Some(annual_revenue) = req.annual_revenue {
            supplier_active.annual_revenue = Set(Some(annual_revenue));
        }
        if let Some(grade) = req.grade {
            supplier_active.grade = Set(grade);
        }
        if let Some(grade_score) = req.grade_score {
            supplier_active.grade_score = Set(grade_score);
        }
        if let Some(status) = req.status {
            supplier_active.status = Set(status);
        }
        if let Some(is_enabled) = req.is_enabled {
            supplier_active.is_enabled = Set(is_enabled);
        }
        if let Some(remarks) = req.remarks {
            supplier_active.remarks = Set(Some(remarks));
        }
        
        supplier_active.updated_by = Set(Some(user_id));
        supplier_active.updated_at = Set(Utc::now().into());
        
        let updated = supplier_active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 删除供应商
    pub async fn delete_supplier(&self, id: i32) -> Result<(), AppError> {
        // 检查是否有交易记录
        let can_delete = self.can_delete_supplier(id).await?;
        if !can_delete {
            return Err(AppError::ValidationError("供应商有交易记录，无法删除".to_string()));
        }
        
        let supplier = self.get_supplier(id).await?;
        supplier.delete(&*self.db).await?;
        Ok(())
    }

    /// 检查供应商是否可删除
    pub async fn can_delete_supplier(&self, _id: i32) -> Result<bool, AppError> {
        // TODO: 检查是否有采购订单
        // 这里需要查询采购订单表，暂时返回 true
        Ok(true)
    }

    /// 切换供应商状态
    pub async fn toggle_supplier_status(
        &self,
        id: i32,
        enable: bool,
        user_id: i32,
    ) -> Result<supplier::Model, AppError> {
        let supplier = self.get_supplier(id).await?;
        let mut supplier_active: supplier::ActiveModel = supplier.into();
        
        supplier_active.is_enabled = Set(enable);
        supplier_active.status = Set(if enable { "active".to_string() } else { "inactive".to_string() });
        supplier_active.updated_by = Set(Some(user_id));
        supplier_active.updated_at = Set(Utc::now().into());
        
        let updated = supplier_active.update(&*self.db).await?;
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
            .order_by(supplier_contact::Column::IsPrimary, Order::Asc)
            .order_by(supplier_contact::Column::ContactName, Order::Asc)
            .all(&*self.db)
            .await?;
        Ok(contacts)
    }

    /// 创建供应商联系人
    pub async fn create_supplier_contact(
        &self,
        supplier_id: i32,
        req: CreateContactRequest,
        _user_id: i32,
    ) -> Result<supplier_contact::Model, AppError> {
        // 如果设置为主要联系人，先将其他联系人取消主要联系人状态
        if req.is_primary {
            self.clear_primary_contacts(supplier_id).await?;
        }

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
            ..Default::default()
        }.insert(&*self.db).await?;

        Ok(contact)
    }

    /// 更新供应商联系人
    pub async fn update_supplier_contact(
        &self,
        contact_id: i32,
        req: UpdateContactRequest,
        _user_id: i32,
    ) -> Result<supplier_contact::Model, AppError> {
        let contact = self.get_contact(contact_id).await?;
        let supplier_id = contact.supplier_id;
        let mut contact_active: supplier_contact::ActiveModel = contact.into();

        // 如果设置为主要联系人，先将其他联系人取消主要联系人状态
        if let Some(true) = req.is_primary {
            self.clear_primary_contacts(supplier_id).await?;
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

        let updated = contact_active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 删除供应商联系人
    pub async fn delete_supplier_contact(&self, contact_id: i32) -> Result<(), AppError> {
        let contact = self.get_contact(contact_id).await?;
        contact.delete(&*self.db).await?;
        Ok(())
    }

    /// 获取联系人详情
    async fn get_contact(&self, contact_id: i32) -> Result<supplier_contact::Model, AppError> {
        supplier_contact::Entity::find_by_id(contact_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::NotFound(format!("联系人 {} 不存在", contact_id)))
    }

    /// 取消所有主要联系人
    async fn clear_primary_contacts(&self, supplier_id: i32) -> Result<(), AppError> {
        use sea_orm::ActiveModelTrait;
        
        let contacts = supplier_contact::Entity::find()
            .filter(supplier_contact::Column::SupplierId.eq(supplier_id))
            .filter(supplier_contact::Column::IsPrimary.eq(true))
            .all(&*self.db)
            .await?;

        for contact in contacts {
            let mut contact_active: supplier_contact::ActiveModel = contact.into();
            contact_active.is_primary = Set(false);
            contact_active.update(&*self.db).await?;
        }

        Ok(())
    }
}

/// 分页响应
#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub page: u64,
    pub page_size: u64,
    pub total: u64,
}

/// 创建供应商请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateSupplierRequest {
    #[validate(length(min = 2, max = 200))]
    pub supplier_name: String,
    #[validate(length(min = 2, max = 100))]
    pub supplier_short_name: String,
    pub supplier_type: String,
    #[validate(length(equal = 18))]
    pub credit_code: String,
    pub registered_address: String,
    pub business_address: Option<String>,
    pub legal_representative: String,
    pub registered_capital: Decimal,
    pub establishment_date: NaiveDate,
    pub business_term: Option<String>,
    pub business_scope: Option<String>,
    pub taxpayer_type: String,
    pub bank_name: String,
    pub bank_account: String,
    pub contact_phone: String,
    pub fax: Option<String>,
    pub website: Option<String>,
    pub email: Option<String>,
    pub main_business: Option<String>,
    pub main_market: Option<String>,
    pub employee_count: Option<i32>,
    pub annual_revenue: Option<Decimal>,
    #[validate]
    pub contacts: Vec<CreateContactRequest>,
    #[validate]
    pub qualifications: Vec<CreateQualificationRequest>,
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
    pub grade: Option<String>,
    pub grade_score: Option<Decimal>,
    pub status: Option<String>,
    pub is_enabled: Option<bool>,
    pub remarks: Option<String>,
}

/// 创建联系人请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateContactRequest {
    #[validate(length(min = 1, max = 50))]
    pub contact_name: String,
    pub department: Option<String>,
    pub position: Option<String>,
    #[validate(custom(function = "validate_mobile_phone"))]
    pub mobile_phone: String,
    pub tel_phone: Option<String>,
    pub email: Option<String>,
    pub wechat: Option<String>,
    pub qq: Option<String>,
    pub is_primary: bool,
    pub remarks: Option<String>,
}

fn validate_mobile_phone(phone: &str) -> Result<(), validator::ValidationError> {
    let re = regex::Regex::new(r"^1[3-9]\d{9}$").unwrap();
    if re.is_match(phone) {
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
    #[validate(length(min = 1, max = 200))]
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
#[derive(Debug, Deserialize)]
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
}
