import re

with open('backend/src/services/ar_invoice_service.rs', 'r') as f:
    content = f.read()

# add UpdateArInvoiceRequest struct
update_struct = '''pub struct UpdateArInvoiceRequest {
    pub invoice_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub invoice_amount: Option<Decimal>,
}'''

if 'UpdateArInvoiceRequest' not in content:
    content = content.replace('pub struct CreateArInvoiceRequest {', update_struct + '\n\n#[derive(Debug, Deserialize)]\npub struct CreateArInvoiceRequest {')

# Add missing functions
new_funcs = '''
    pub async fn update(&self, id: i32, req: UpdateArInvoiceRequest, user_id: i32) -> Result<ar_invoice::Model, AppError> {
        let invoice = ar_invoice::Entity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("应收单不存在".to_string()))?;

        if invoice.status != "DRAFT" {
            return Err(AppError::BadRequest("非草稿状态的应收单无法修改".to_string()));
        }

        let mut active_invoice: ar_invoice::ActiveModel = invoice.into();
        
        if let Some(date) = req.invoice_date {
            active_invoice.invoice_date = Set(date);
        }
        if let Some(date) = req.due_date {
            active_invoice.due_date = Set(date);
        }
        if let Some(amt) = req.invoice_amount {
            active_invoice.invoice_amount = Set(amt);
            active_invoice.unpaid_amount = Set(amt - active_invoice.received_amount.unwrap_or(Decimal::from(0)));
        }
        
        active_invoice.updated_at = Set(Utc::now());

        let result = active_invoice.update(&*self.db).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(result)
    }

    pub async fn delete(&self, id: i32, _user_id: i32) -> Result<(), AppError> {
        let invoice = ar_invoice::Entity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("应收单不存在".to_string()))?;

        if invoice.status != "DRAFT" {
            return Err(AppError::BadRequest("非草稿状态的应收单无法删除".to_string()));
        }

        ar_invoice::Entity::delete_by_id(id)
            .exec(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub async fn approve(&self, id: i32, user_id: i32) -> Result<ar_invoice::Model, AppError> {
        let invoice = ar_invoice::Entity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("应收单不存在".to_string()))?;

        if invoice.status != "DRAFT" {
            return Err(AppError::BadRequest("只能审批草稿状态的应收单".to_string()));
        }

        let mut active_invoice: ar_invoice::ActiveModel = invoice.into();
        active_invoice.status = Set("APPROVED".to_string());
        active_invoice.approval_status = Set("APPROVED".to_string());
        active_invoice.reviewed_by = Set(Some(user_id));
        active_invoice.reviewed_at = Set(Some(Utc::now()));
        active_invoice.updated_at = Set(Utc::now());

        let result = active_invoice.update(&*self.db).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(result)
    }

    pub async fn cancel(&self, id: i32, _reason: String, _user_id: i32) -> Result<ar_invoice::Model, AppError> {
        let invoice = ar_invoice::Entity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("应收单不存在".to_string()))?;

        if invoice.status == "CANCELLED" {
            return Err(AppError::BadRequest("单据已取消".to_string()));
        }

        let mut active_invoice: ar_invoice::ActiveModel = invoice.into();
        active_invoice.status = Set("CANCELLED".to_string());
        active_invoice.updated_at = Set(Utc::now());

        let result = active_invoice.update(&*self.db).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(result)
    }
'''

# insert before the last '}'
if 'pub async fn delete' not in content:
    content = content.rsplit('}', 1)[0] + new_funcs + '}\n'

with open('backend/src/services/ar_invoice_service.rs', 'w') as f:
    f.write(content)
