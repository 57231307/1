//! CRM 线索服务（crm/lead）
//!
//! 包含线索 CRUD、状态更新、线索转客户等。
//! 拆分自原 `crm_service.rs`。

use crate::models::{crm_lead, crm_opportunity, customer};
// 批次 212 P2-5 修复（v12 复审）：硬编码 "active" 替换为 master_data 常量
use crate::models::status::master_data;
// 批次 236 v13 P1-1：线索状态常量接入（规则 0）
use crate::models::status::crm_lead as lead_status;
// V15 P0-S01：行级数据权限工具
use crate::utils::data_scope::{apply_data_scope, check_resource_owner, DataScopeContext};
use crate::utils::error::AppError;
use crate::utils::xlsx_export::XlsxTable;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
    Set, TransactionTrait,
};

use super::cust::CrmService;

impl CrmService {
    /// 创建线索
    pub async fn create_lead(
        &self,
        req: crate::models::dto::crm_dto::CreateLeadRequest,
        user_id: i32,
    ) -> Result<crm_lead::Model, AppError> {
        // P1 3-13 修复（批次 60）：包裹事务，确保单号生成的 advisory_xact_lock
        // 与 INSERT 在同一事务内，锁覆盖完整临界区
        let txn = (*self.db).begin().await?;

        // 生成线索编号（如果用户提供则用用户的，否则用 DocumentNumberGenerator 生成）
        // P1 3-13 修复（批次 60）：原实现基于时间戳，同秒并发会产生重复单号
        let lead_no = if let Some(custom_no) = req.lead_no {
            custom_no
        } else {
            crate::utils::number_generator::DocumentNumberGenerator::generate_no_with_txn(
                &txn,
                "LD",
                crm_lead::Entity,
                crm_lead::Column::LeadNo,
            )
            .await?
        };
        let lead_source = req.lead_source.unwrap_or_else(|| "OTHER".to_string());
        let owner_id = user_id;
        let owner_name = format!("用户{}", user_id);
        let contact_name = req.contact_name.unwrap_or_else(|| {
            req.company_name
                .clone()
                .unwrap_or_else(|| "未知".to_string())
        });
        let lead_status = req.lead_status.clone();
        let now = chrono::Utc::now();

        let lead = crm_lead::ActiveModel {
            id: Default::default(),
            lead_no: Set(lead_no),
            lead_source: Set(lead_source),
            lead_status: Set(lead_status),
            company_name: Set(req.company_name),
            contact_name: Set(contact_name),
            contact_title: Set(req.contact_title),
            mobile_phone: Set(req.mobile_phone),
            tel_phone: Set(req.tel_phone),
            email: Set(req.email),
            wechat: Set(req.wechat),
            qq: Set(req.qq),
            address: Set(req.address),
            product_interest: Set(req.product_interest),
            estimated_quantity: Set(req.estimated_quantity),
            estimated_amount: Set(req.estimated_amount),
            expected_delivery_date: Set(req.expected_delivery_date),
            requirement_desc: Set(req.requirement_desc),
            owner_id: Set(owner_id),
            owner_name: Set(owner_name),
            priority: Set(req.priority),
            rating: Set(req.rating),
            tags: Set(req.tags),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;
        Ok(lead)
    }

    /// 列出线索（返回分页结果）
    pub async fn list_leads(
        &self,
        query: crate::models::dto::crm_dto::LeadQuery,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<serde_json::Value, AppError> {
        let page = query.page.unwrap_or(1).clamp(1, 1000);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100); // v10 P2-3 修复：crm 模块统一 clamp(1,100) 防 DoS

        let mut q = crm_lead::Entity::find();

        if let Some(s) = query.lead_status {
            q = q.filter(crm_lead::Column::LeadStatus.eq(s));
        }

        // 批次 111 P1-10：接入 source 过滤（精确匹配 lead_source 列）
        if let Some(source) = query.source {
            q = q.filter(crm_lead::Column::LeadSource.eq(source));
        }

        // 批次 111 P1-10：接入 keyword 模糊搜索
        // 匹配 company_name / contact_name / mobile_phone / email 四个字段（OR 关系）
        if let Some(keyword) = query.keyword {
            let pattern = format!("%{}%", keyword);
            q = q.filter(
                sea_orm::Condition::any()
                    .add(crm_lead::Column::CompanyName.like(&pattern))
                    .add(crm_lead::Column::ContactName.like(&pattern))
                    .add(crm_lead::Column::MobilePhone.like(&pattern))
                    .add(crm_lead::Column::Email.like(&pattern)),
            );
        }

        // v11 批次 153 P2-A：接入 industry 过滤（精确匹配 industry 列）
        if let Some(industry) = query.industry {
            q = q.filter(crm_lead::Column::Industry.eq(industry));
        }

        // V15 P0-S01：行级数据权限过滤
        // crm_lead 表无 department_id，Dept 退化为 Self；
        // CRM 业务数据权限语义为"我负责的线索"，使用 owner_id（i32 必填）作为 owner_column，
        // 比 created_by（Option<i32>，create_lead 未显式设置）更可靠且符合业务语义。
        if let Some(ctx) = data_scope {
            q = apply_data_scope(
                q,
                ctx,
                crm_lead::Column::OwnerId,
                crm_lead::Column::OwnerId, // 无 department_id，Dept 退化为 Self，复用 owner_id
            );
        }

        let paginator = q
            .order_by(crm_lead::Column::CreatedAt, sea_orm::Order::Desc)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        // 批次 98 P2-A 修复（v5 复审）：page clamp 防 DoS
        let items: Vec<crm_lead::Model> = paginator.fetch_page(page.clamp(1, 1000).saturating_sub(1)).await?;

        Ok(serde_json::json!({
            "data": items,
            "total": total,
            "page": page,
            "page_size": page_size,
        }))
    }

    /// 导出线索为 xlsx（v11 批次 142 升级：CSV → xlsx，规则 3 强制要求）
    ///
    /// v11 批次 141 新增：前端 exportLeads API 真实接入。
    /// v11 批次 142 升级：导出格式从 CSV 升级为 xlsx（Excel 标准格式）。
    /// 查询所有匹配条件（不分页）的线索，生成 XlsxTable。
    /// 导出字段：线索编号/公司名称/联系人/职位/手机号/座机/邮箱/线索来源/线索状态/负责人/优先级/创建时间
    pub async fn export_leads(
        &self,
        query: crate::models::dto::crm_dto::LeadQuery,
    ) -> Result<XlsxTable, AppError> {
        let mut q = crm_lead::Entity::find();

        if let Some(s) = query.lead_status {
            q = q.filter(crm_lead::Column::LeadStatus.eq(s));
        }
        if let Some(source) = query.source {
            q = q.filter(crm_lead::Column::LeadSource.eq(source));
        }
        if let Some(keyword) = query.keyword {
            let pattern = format!("%{}%", keyword);
            q = q.filter(
                sea_orm::Condition::any()
                    .add(crm_lead::Column::CompanyName.like(&pattern))
                    .add(crm_lead::Column::ContactName.like(&pattern))
                    .add(crm_lead::Column::MobilePhone.like(&pattern))
                    .add(crm_lead::Column::Email.like(&pattern)),
            );
        }

        // 限制导出最大 10000 条，防止 DoS
        let leads: Vec<crm_lead::Model> = q
            .order_by(crm_lead::Column::CreatedAt, sea_orm::Order::Desc)
            .limit(10000)
            .all(&*self.db)
            .await?;

        let headers = vec![
            "线索编号".to_string(),
            "公司名称".to_string(),
            "联系人".to_string(),
            "职位".to_string(),
            "手机号".to_string(),
            "座机".to_string(),
            "邮箱".to_string(),
            "线索来源".to_string(),
            "线索状态".to_string(),
            "负责人".to_string(),
            "优先级".to_string(),
            "创建时间".to_string(),
        ];

        let rows: Vec<Vec<String>> = leads
            .iter()
            .map(|lead| {
                vec![
                    lead.lead_no.clone(),
                    lead.company_name.clone().unwrap_or_default(),
                    lead.contact_name.clone(),
                    lead.contact_title.clone().unwrap_or_default(),
                    lead.mobile_phone.clone().unwrap_or_default(),
                    lead.tel_phone.clone().unwrap_or_default(),
                    lead.email.clone().unwrap_or_default(),
                    lead.lead_source.clone(),
                    lead.lead_status.clone().unwrap_or_default(),
                    lead.owner_name.clone(),
                    lead.priority.clone().unwrap_or_default(),
                    lead.created_at
                        .map(|t| t.to_rfc3339())
                        .unwrap_or_default(),
                ]
            })
            .collect();

        Ok(XlsxTable {
            sheet_name: "线索列表".to_string(),
            headers,
            rows,
        })
    }

    /// 批量导入线索（v11 批次 157d-4 新增）：解析 xlsx 字节并逐行创建线索
    /// xlsx 列顺序与 export_leads 一致：线索编号/公司名称/联系人/职位/手机号/座机/邮箱/线索来源/线索状态/负责人/优先级/创建时间
    /// 失败行不影响其他行，最终返回成功/失败统计与错误详情
    pub async fn import_leads(
        &self,
        file_bytes: Vec<u8>,
        user_id: i32,
    ) -> Result<crate::models::dto::crm_dto::ImportLeadsResult, AppError> {
        use calamine::{open_workbook_auto_from_rs, Data, Reader};
        use std::io::Cursor;

        let cursor = Cursor::new(file_bytes);
        let mut workbook = open_workbook_auto_from_rs(cursor)
            .map_err(|e| AppError::bad_request(format!("无法解析 xlsx 文件：{}", e)))?;

        // 读取第一个 sheet
        let sheet_name = workbook
            .sheet_names()
            .first()
            .cloned()
            .ok_or_else(|| AppError::bad_request("xlsx 文件无工作表".to_string()))?;
        let range = workbook
            .worksheet_range(&sheet_name)
            .map_err(|e| AppError::bad_request(format!("读取工作表失败：{}", e)))?;

        let mut rows = range.rows();
        // 第一行为表头，跳过
        let _header = rows.next();
        let data_rows: Vec<&[Data]> = rows.collect();

        let total = data_rows.len() as u32;
        let mut success_count: u32 = 0;
        let mut errors: Vec<crate::models::dto::crm_dto::ImportLeadError> = Vec::new();

        for (idx, row) in data_rows.iter().enumerate() {
            let row_no = (idx + 2) as u32; // 行号从 2 开始（1 为表头）
            // 安全提取单元格字符串值
            let cell_str = |i: usize| -> Option<String> {
                row.get(i).and_then(|c| match c {
                    Data::String(s) => {
                        let t = s.trim();
                        if t.is_empty() {
                            None
                        } else {
                            Some(t.to_string())
                        }
                    }
                    Data::Int(n) => Some(n.to_string()),
                    Data::Float(f) => Some(f.to_string()),
                    Data::DateTimeIso(s) | Data::DurationIso(s) => Some(s.clone()),
                    _ => None,
                })
            };

            let req = crate::models::dto::crm_dto::CreateLeadRequest {
                lead_no: cell_str(0),
                lead_source: cell_str(7),
                lead_status: cell_str(8),
                company_name: cell_str(1),
                contact_name: cell_str(2),
                contact_title: cell_str(3),
                mobile_phone: cell_str(4),
                tel_phone: cell_str(5),
                email: cell_str(6),
                wechat: None,
                qq: None,
                address: None,
                product_interest: None,
                estimated_quantity: None,
                estimated_amount: None,
                expected_delivery_date: None,
                requirement_desc: None,
                priority: cell_str(10),
                rating: None,
                tags: None,
            };

            match self.create_lead(req, user_id).await {
                Ok(_) => success_count += 1,
                Err(e) => errors.push(crate::models::dto::crm_dto::ImportLeadError {
                    row: row_no,
                    message: format!("{}", e),
                }),
            }
        }

        let failed_count = total - success_count;
        Ok(crate::models::dto::crm_dto::ImportLeadsResult {
            total,
            success_count,
            failed_count,
            errors,
        })
    }

    /// 获取线索详情
    pub async fn get_lead(
        &self,
        lead_id: i32,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<crm_lead::Model, AppError> {
        let lead = crm_lead::Entity::find_by_id(lead_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("线索 {} 不存在", lead_id)))?;
        // V15 P0-S01：行级数据权限校验（IDOR 防护）
        // crm_lead 表无 department_id，Dept 退化为 Self；
        // 使用 owner_id（业务负责人）作为归属判定字段。
        if let Some(ctx) = data_scope {
            if !check_resource_owner(ctx, Some(lead.owner_id), None) {
                return Err(AppError::permission_denied(format!(
                    "无权访问线索 {}（数据范围限制）", lead_id
                )));
            }
        }
        Ok(lead)
    }

    /// 更新线索
    pub async fn update_lead(
        &self,
        lead_id: i32,
        req: crate::models::dto::crm_dto::UpdateLeadRequest,
        user_id: i32,
    ) -> Result<crm_lead::Model, AppError> {
        let lead = self.get_lead(lead_id, None).await?;
        let mut lead_active: crm_lead::ActiveModel = lead.into();

        if let Some(v) = req.lead_source {
            lead_active.lead_source = Set(v);
        }
        if let Some(v) = req.lead_status {
            lead_active.lead_status = Set(Some(v));
        }
        if let Some(v) = req.company_name {
            lead_active.company_name = Set(Some(v));
        }
        if let Some(v) = req.contact_name {
            lead_active.contact_name = Set(v);
        }
        if let Some(v) = req.contact_title {
            lead_active.contact_title = Set(Some(v));
        }
        if let Some(v) = req.mobile_phone {
            lead_active.mobile_phone = Set(Some(v));
        }
        if let Some(v) = req.tel_phone {
            lead_active.tel_phone = Set(Some(v));
        }
        if let Some(v) = req.email {
            lead_active.email = Set(Some(v));
        }
        if let Some(v) = req.wechat {
            lead_active.wechat = Set(Some(v));
        }
        if let Some(v) = req.qq {
            lead_active.qq = Set(Some(v));
        }
        if let Some(v) = req.address {
            lead_active.address = Set(Some(v));
        }
        if let Some(v) = req.product_interest {
            lead_active.product_interest = Set(Some(v));
        }
        if let Some(v) = req.estimated_quantity {
            lead_active.estimated_quantity = Set(Some(v));
        }
        if let Some(v) = req.estimated_amount {
            lead_active.estimated_amount = Set(Some(v));
        }
        if let Some(v) = req.expected_delivery_date {
            lead_active.expected_delivery_date = Set(Some(v));
        }
        if let Some(v) = req.requirement_desc {
            lead_active.requirement_desc = Set(Some(v));
        }
        if let Some(v) = req.priority {
            lead_active.priority = Set(Some(v));
        }
        if let Some(v) = req.rating {
            lead_active.rating = Set(Some(v));
        }
        if let Some(v) = req.tags {
            lead_active.tags = Set(Some(v));
        }

        lead_active.updated_at = Set(Some(chrono::Utc::now()));

        let lead = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            lead_active,
            // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
            Some(user_id),
        )
        .await?;

        Ok(lead)
    }

    /// 删除线索
    pub async fn delete_lead(&self, lead_id: i32, user_id: i32) -> Result<(), AppError> {
        // P0 8-3 修复：delete 操作补审计日志
        // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            crm_lead::Entity,
            _,
        >(&*self.db, "crm_lead", lead_id, Some(user_id))
        .await
    }

    /// 更新线索状态
    pub async fn update_lead_status(
        &self,
        lead_id: i32,
        status: &str,
        user_id: i32,
    ) -> Result<(), AppError> {
        let lead = self.get_lead(lead_id, None).await?;
        let mut lead_active: crm_lead::ActiveModel = lead.into();
        lead_active.lead_status = Set(Some(status.to_string()));
        lead_active.updated_at = Set(Some(chrono::Utc::now()));

        // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            lead_active,
            Some(user_id),
        )
        .await?;

        Ok(())
    }

    /// 将线索转换为客户（同时创建一条对应的"初步接洽"商机）
    pub async fn convert_lead_to_customer(
        &self,
        lead_id: i32,
        req: crate::models::dto::crm_dto::ConvertLeadRequest,
        user_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        // 1. 查询线索
        let lead = self.get_lead(lead_id, None).await?;

        if lead.lead_status.as_deref() == Some(lead_status::CONVERTED) {
            return Err(AppError::business("线索已转换为客户".to_string()));
        }

        let txn = self.db.begin().await?;

        // 2. 创建客户
        let customer_code = format!("C{}", chrono::Utc::now().timestamp());
        let customer_name = lead
            .company_name
            .clone()
            .unwrap_or_else(|| lead.contact_name.clone());
        let contact_person = Some(lead.contact_name.clone());
        let contact_phone = lead.mobile_phone.clone().or(lead.tel_phone.clone());
        let customer_industry: Option<String> = None;
        let customer_type = req.customer_type.unwrap_or_else(|| "POTENTIAL".to_string());

        let new_customer = customer::ActiveModel {
            id: Default::default(),
            customer_code: Set(customer_code.clone()),
            customer_name: Set(customer_name.clone()),
            contact_person: Set(contact_person),
            contact_phone: Set(contact_phone),
            contact_email: Set(lead.email.clone()),
            address: Set(lead.address.clone()),
            city: Set(None),
            province: Set(None),
            country: Set(None),
            postal_code: Set(None),
            credit_limit: Set(rust_decimal::Decimal::ZERO),
            payment_terms: Set(crate::constants::DEFAULT_PAYMENT_TERMS_DAYS),
            tax_id: Set(None),
            bank_name: Set(None),
            bank_account: Set(None),
            status: Set(master_data::ACTIVE.to_string()),
            customer_type: Set(customer_type),
            notes: Set(req.notes.clone().or(lead.requirement_desc.clone())),
            created_by: Set(Some(user_id)),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
            customer_industry: Set(customer_industry),
            main_products: Set(None),
            annual_purchase: Set(None),
            quality_requirement: Set(None),
            inspection_standard: Set(None),
        }
        .insert(&txn)
        .await?;

        // 3. 更新线索状态
        let mut lead_active: crm_lead::ActiveModel = lead.clone().into();
        lead_active.lead_status = Set(Some(lead_status::CONVERTED.to_string()));
        lead_active.converted_customer_id = Set(Some(new_customer.id));
        lead_active.converted_at = Set(Some(chrono::Utc::now()));
        lead_active.updated_at = Set(Some(chrono::Utc::now()));
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            lead_active,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        // 4. 创建初步商机
        let opportunity_no = format!("OPP{}", chrono::Utc::now().format("%Y%m%d%H%M%S"));
        let opportunity_name = format!("{} - 初步接洽", customer_name);
        let _opportunity = crm_opportunity::ActiveModel {
            id: Default::default(),
            opportunity_no: Set(opportunity_no),
            opportunity_name: Set(opportunity_name),
            customer_id: Set(new_customer.id),
            lead_id: Set(Some(lead_id)),
            opportunity_type: Set(Some("NEW".to_string())),
            opportunity_stage: Set(Some("QUALIFICATION".to_string())),
            win_probability: Set(Some(rust_decimal::Decimal::new(20, 0))),
            estimated_amount: Set(lead.estimated_amount),
            actual_amount: Set(None),
            currency: Set(Some(crate::constants::DEFAULT_CURRENCY.to_string())),
            expected_close_date: Set(lead.expected_delivery_date),
            actual_close_date: Set(None),
            product_ids: Set(None),
            product_names: Set(None),
            product_desc: Set(lead.product_interest),
            owner_id: Set(lead.owner_id),
            owner_name: Set(lead.owner_name.clone()),
            opportunity_status: Set(Some("OPEN".to_string())),
            created_by: Set(Some(user_id)),
            created_at: Set(Some(chrono::Utc::now())),
            updated_at: Set(Some(chrono::Utc::now())),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        // 5. 提交事务
        txn.commit().await?;

        Ok(serde_json::json!({
            "customer_id": new_customer.id,
            "customer_code": new_customer.customer_code,
            "customer_name": new_customer.customer_name,
        }))
    }
}
