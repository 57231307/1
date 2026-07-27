//! 客户联系人管理 impl 子模块（customer_ops/contact）
//!
//! 拆分：从原 `customer_service.rs` 迁移 CustomerService 的客户联系人方法：
//! - list_customer_contacts（列表，主联系人优先）
//! - create_customer_contact（创建，若 is_primary 事务内先清空其他主联系人）
//! - update_customer_contact（更新，事务内更新 + 审计）
//! - delete_customer_contact（删除）
//! - clear_primary_contacts_txn（事务内取消指定客户所有主联系人状态）

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, ModelTrait, Order, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};

use crate::models::customer_contact;
use crate::services::customer_ops::types::{
    CreateCustomerContactRequest, UpdateCustomerContactRequest,
};
use crate::services::customer_service::CustomerService;
use crate::utils::error::AppError;

impl CustomerService {
    /// 获取客户联系人列表（主联系人优先，其次按姓名升序，补 LIMIT 兜底）
    pub async fn list_customer_contacts(
        &self,
        customer_id: i32,
    ) -> Result<Vec<customer_contact::Model>, AppError> {
        let contacts = customer_contact::Entity::find()
            .filter(customer_contact::Column::CustomerId.eq(customer_id))
            .order_by(customer_contact::Column::IsPrimary, Order::Desc)
            .order_by(customer_contact::Column::Name, Order::Asc)
            .limit(10_000)
            .all(&*self.db)
            .await?;
        Ok(contacts)
    }

    /// 创建客户联系人（若 is_primary=true，事务内先清空其他主联系人状态）
    pub async fn create_customer_contact(
        &self,
        customer_id: i32,
        req: CreateCustomerContactRequest,
        user_id: i32,
    ) -> Result<customer_contact::Model, AppError> {
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

    /// 更新客户联系人（若 is_primary 由非主改为主，事务内先清空其他主联系人）
    pub async fn update_customer_contact(
        &self,
        contact_id: i32,
        req: UpdateCustomerContactRequest,
        user_id: i32,
    ) -> Result<customer_contact::Model, AppError> {
        let txn = (*self.db).begin().await?;

        let contact = customer_contact::Entity::find_by_id(contact_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("联系人 {} 不存在", contact_id)))?;

        let customer_id = contact.customer_id;
        let mut contact_active: customer_contact::ActiveModel = contact.into();

        if let Some(true) = req.is_primary {
            self.clear_primary_contacts_txn(customer_id, &txn).await?;
        }

        Self::apply_contact_updates(&mut contact_active, req);

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

    fn apply_contact_updates(
        contact_active: &mut customer_contact::ActiveModel,
        req: UpdateCustomerContactRequest,
    ) {
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
    }

    /// 删除客户联系人
    pub async fn delete_customer_contact(&self, contact_id: i32) -> Result<(), AppError> {
        let contact = customer_contact::Entity::find_by_id(contact_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("联系人 {} 不存在", contact_id)))?;
        contact.delete(&*self.db).await?;
        Ok(())
    }

/// 取消指定客户的所有主联系人状态（事务内，保证"每客户最多一个主联系人"约束）
    pub(crate) async fn clear_primary_contacts_txn(
        &self,
        customer_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        let primary_contacts = customer_contact::Entity::find()
            .filter(customer_contact::Column::CustomerId.eq(customer_id))
            .filter(customer_contact::Column::IsPrimary.eq(true))
            .all(txn)
            .await?;

        for contact in primary_contacts {
            let mut active: customer_contact::ActiveModel = contact.into();
            active.is_primary = Set(false);
            active.updated_at = Set(Utc::now().into());
            active.update(txn).await?;
        }

        Ok(())
    }
}
