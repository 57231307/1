//! CRM 线索服务（crm/lead）
//!
//! 包含线索 CRUD、状态更新、线索转客户等。
//! 拆分自原 `crm_service.rs`。

use crate::models::{crm_lead, crm_opportunity, customer};
// 批次 212 P2-5 修复（v12 复审）：硬编码 "active" 替换为 master_data 常量
use crate::models::status::master_data;
// 批次 236 v13 P1-1：线索状态常量接入（规则 0）
use crate::models::status::crm_lead as lead_status;
// 商机状态/阶段常量接入（规则 0）：线索转商机、漏斗统计读写点统一引用权威词表
use crate::models::status::crm_opportunity as opp_status;
// V15 P0-S01：行级数据权限工具
use crate::utils::data_scope::{
    DataScope, DataScopeContext, PoolVisibility, apply_department_scope_with_pool,
    check_resource_owner, check_resource_write_owner,
};
use crate::utils::error::AppError;
use crate::utils::messages::err_msg;
use crate::utils::xlsx_export::XlsxTable;
use sea_orm::sea_query::{Expr, extension::postgres::PgExpr};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};

use super::cust::CrmService;

impl CrmService {
    /// 创建线索
    ///
    /// `operator_name`：真实操作人展示名，由调用方从 `AuthContext.username` 传入
    /// （与 `services/crm/pool.rs::claim_pool_customers` 同一口径：`AuthContext` 只有
    /// username 一个身份展示字段，无真实姓名，取 `users.real_name` 需新增跨模块查库）。
    /// 本参数是 `owner_name` 的唯一合法取值来源——修复前它由 `format!("用户{user_id}")`
    /// 拼出，属本仓硬规则禁止的造假展示名（既不可读也不可回查）。批量导入 `import_leads`
    /// 按行调用本方法，展示名同样由入口一次性传入，禁止为取名字在此逐行查库（N+1）。
    pub async fn create_lead(
        &self,
        req: crate::models::dto::crm_dto::CreateLeadRequest,
        user_id: i32,
        operator_name: &str,
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
        let owner_name = operator_name.to_string();
        let contact_name = req.contact_name.unwrap_or_else(|| {
            req.company_name
                .clone()
                .unwrap_or_else(|| "未知".to_string())
        });
        let lead_status = req.lead_status.clone();
        // 取值校验：lead_status 必须属于权威词表（models/status::crm_lead），
        // 非法值直接拒绝，不再裸落库产生脏值（DB 侧兜底见 chk_crm_lead_lead_status）
        if let Some(s) = &lead_status {
            Self::ensure_valid_lead_status(s)?;
        }
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

        // 行级数据权限过滤：owner=owner_id，dept=DepartmentId（m_rls_dept_domain），
        // 公海（lead_status='pool'）放行与 RLS 策略同口径
        if let Some(ctx) = data_scope {
            q = apply_department_scope_with_pool(
                q,
                ctx,
                crm_lead::Column::OwnerId,
                crm_lead::Column::DepartmentId,
                crm_lead::Column::LeadStatus.eq(lead_status::POOL),
                PoolVisibility::Open,
            );
        }

        let paginator = q
            .order_by(crm_lead::Column::CreatedAt, sea_orm::Order::Desc)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        // 批次 98 P2-A 修复（v5 复审）：page clamp 防 DoS
        let items: Vec<crm_lead::Model> = paginator
            .fetch_page(page.clamp(1, 1000).saturating_sub(1))
            .await?;

        Ok(serde_json::json!({
            "data": items,
            "total": total,
            "page": page,
            "page_size": page_size,
        }))
    }

    /// 线索导出列定义：`(crm_lead 列名, 中文表头)`，**列序/表头/取值的唯一事实来源**。
    ///
    /// 列名逐字取 `models/crm_lead.rs` 的字段名（= list_leads/get_lead 出参键），
    /// 因此 handler 侧可直接用同一份 `allowed_fields`/`hidden_fields`（按列名配置）
    /// 与同一份 `filter_fields` 判定，不为导出另造第二套字段权限规则（#207）；
    /// handler 需掩码/剔除某列时按列名经 `export_column_index` 定位，不重复硬编码下标。
    ///
    /// 顺序同时是 `import_leads` 的解析口径（`build_lead_request_from_row` 按下标取值），
    /// 调整本表必须同步调整导入，二者不得各写一套。
    pub const EXPORT_LEAD_COLUMNS: &[(&str, &str)] = &[
        ("lead_no", "线索编号"),
        ("company_name", "公司名称"),
        ("contact_name", "联系人"),
        ("contact_title", "职位"),
        ("mobile_phone", "手机号"),
        ("tel_phone", "座机"),
        ("email", "邮箱"),
        ("lead_source", "线索来源"),
        ("lead_status", "线索状态"),
        ("owner_name", "负责人"),
        ("priority", "优先级"),
        ("created_at", "创建时间"),
    ];

    /// 含个人信息（PII）的导出列：手机号/座机走 `field_mask::mask_phone`，
    /// 邮箱走 `field_mask::mask_email` —— 与列表/详情默认脱敏同一列集合、同一实现。
    pub const EXPORT_PII_PHONE_COLUMNS: &'static [&'static str] = &["mobile_phone", "tel_phone"];
    pub const EXPORT_PII_EMAIL_COLUMNS: &'static [&'static str] = &["email"];

    /// 线索默认字段脱敏的**唯一实现**（"无角色数据权限行且非 admin"分支，即 P1-08-5 分支），
    /// 供四个出口共用：`crm_handler::list_leads`（列表）、`crm_handler::get_lead`（详情）、
    /// `crm_pool_handler::list_pool`（公海列表）、`claim_from_pool`/`recycle_to_pool`（写响应）。
    ///
    /// 为什么必须收敛成一个函数而不是各写一份内联分支：掩码列集合一旦出现第二份手写实现
    /// 就会漂移。本波次实证到的两处漂移即此因：
    /// - 列表/详情漏 `tel_phone`（座机），而导出侧 `EXPORT_PII_PHONE_COLUMNS` 已含它
    ///   → 同一角色"导出被打码、列表/详情原文"；
    /// - 公海写响应整行原文回传（含 mobile_phone/tel_phone/email/address）
    ///   → "列表打码、写响应原文"的旁路。
    ///
    /// 电话/邮箱列集合不在本函数重写：直接复用本仓 PII 列集合的权威定义
    /// `utils::field_mask::mask_contact_fields_for_role`（电话类含 `mobile_phone` 与
    /// `tel_phone`，邮箱类含 `email`；该函数自身对 `role_id==Some(1)` 放行原文），
    /// 本函数只补它未覆盖的 `address` 整键移除（详情级个人信息，非 admin 不外显，
    /// 与列表既有口径一致）。角色数据权限"有权限行 → filter_fields"这一层不在此函数内：
    /// 判定源仍是 `data_permission_service.get_role_data_permission`，见
    /// `crm_handler::apply_lead_field_permission`。
    pub fn mask_lead_pii_defaults(
        value: serde_json::Value,
        role_id: Option<i32>,
    ) -> serde_json::Value {
        let mut masked = crate::utils::field_mask::mask_contact_fields_for_role(value, role_id);
        // admin 保持原文契约（含 address）：与 mask_contact_fields_for_role 自身的
        // role_id==Some(1) 放行判定同一口径，不在此处另判一次列集合。
        if role_id != Some(1) {
            if let Some(obj) = masked.as_object_mut() {
                obj.remove("address");
            }
        }
        masked
    }

    /// 在导出列定义表中按列名定位下标（供 handler 以列名而非魔法下标操作导出表）。
    /// 列定义表由调用方传入（线索 `EXPORT_LEAD_COLUMNS`、商机
    /// `EXPORT_OPP_COLUMNS`（定义在 `services/crm/opp.rs`）各自单一事实来源），
    /// 使两张导出表共用同一个定位实现，不再各写一份"按位置猜列"。
    pub fn export_column_index(columns: &[(&str, &str)], field: &str) -> Option<usize> {
        columns.iter().position(|(name, _)| *name == field)
    }

    /// 取单个导出单元格的原文（列名未命中定义表属编程错误，不静默放空值：
    /// 记录 error 日志后返回空串，调用方按列名取值前应已用 export_column_index 校验）
    fn export_cell(lead: &crm_lead::Model, field: &str) -> String {
        match field {
            "lead_no" => lead.lead_no.clone(),
            "company_name" => lead.company_name.clone().unwrap_or_default(),
            "contact_name" => lead.contact_name.clone(),
            "contact_title" => lead.contact_title.clone().unwrap_or_default(),
            "mobile_phone" => lead.mobile_phone.clone().unwrap_or_default(),
            "tel_phone" => lead.tel_phone.clone().unwrap_or_default(),
            "email" => lead.email.clone().unwrap_or_default(),
            "lead_source" => lead.lead_source.clone(),
            "lead_status" => lead.lead_status.clone().unwrap_or_default(),
            "owner_name" => lead.owner_name.clone(),
            "priority" => lead.priority.clone().unwrap_or_default(),
            "created_at" => lead.created_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
            other => {
                tracing::error!(
                    field = %other,
                    "导出列定义 EXPORT_LEAD_COLUMNS 与 export_cell 取值分支不一致"
                );
                String::new()
            }
        }
    }

    /// 导出线索为 xlsx（v11 批次 142 升级：CSV → xlsx，规则 3 强制要求）
    /// v11 批次 141 新增：前端 exportLeads API 真实接入。；v11 批次 142 升级：导出格式从 CSV 升级为 xlsx（Excel 标准格式）。；查询所有匹配条件（不分页）的线索，生成 XlsxTable。；导出字段见 `EXPORT_LEAD_COLUMNS`
    ///
    /// #207 行级数据权限：`data_scope` 与 `list_leads` 同语义 —— 传入 ctx 时套用同一个
    /// `apply_department_scope_with_pool`（含公海放行分支），使导出的行集合与该用户
    /// 列表可见集严格一致；传 None 则整体跳过行级过滤（修复前 export 恒为此路径，
    /// self/dept 用户可一次导出全库线索，属越权读 + 个人信息外泄）。
    /// 字段级掩码不在此处做：判定源（角色数据权限 + admin 例外）在 handler，
    /// 与列表/详情共用同一函数，见 `crm_handler::export_leads`。
    pub async fn export_leads(
        &self,
        query: crate::models::dto::crm_dto::LeadQuery,
        data_scope: Option<&DataScopeContext>,
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
        // 与 list_leads（本文件 :136-139）同口径的 industry 过滤：修复前导出漏接该
        // 条件，用户按行业筛选后导出得到的行集与列表不一致（多导出行 = 可见集漂移）
        if let Some(industry) = query.industry {
            q = q.filter(crm_lead::Column::Industry.eq(industry));
        }

        // 行级数据权限过滤：与 list_leads(:143-151) 完全同一函数、同一公海条件
        if let Some(ctx) = data_scope {
            q = apply_department_scope_with_pool(
                q,
                ctx,
                crm_lead::Column::OwnerId,
                crm_lead::Column::DepartmentId,
                crm_lead::Column::LeadStatus.eq(lead_status::POOL),
                PoolVisibility::Open,
            );
        }

        // 限制导出最大 10000 条，防止 DoS
        let leads: Vec<crm_lead::Model> = q
            .order_by(crm_lead::Column::CreatedAt, sea_orm::Order::Desc)
            .limit(10000)
            .all(&*self.db)
            .await?;

        let headers = Self::EXPORT_LEAD_COLUMNS
            .iter()
            .map(|(_, label)| label.to_string())
            .collect::<Vec<String>>();

        let rows: Vec<Vec<String>> = leads
            .iter()
            .map(|lead| {
                Self::EXPORT_LEAD_COLUMNS
                    .iter()
                    .map(|(field, _)| Self::export_cell(lead, field))
                    .collect::<Vec<String>>()
            })
            .collect();

        Ok(XlsxTable {
            sheet_name: "线索列表".to_string(),
            headers,
            rows,
        })
    }

    /// 读取 xlsx 字节，返回首个 sheet 的数据行（已跳过表头）
    async fn read_xlsx_rows(file_bytes: Vec<u8>) -> Result<Vec<Vec<calamine::Data>>, AppError> {
        use calamine::{Reader, open_workbook_auto_from_rs};
        use std::io::Cursor;

        let cursor = Cursor::new(file_bytes);
        let mut workbook = open_workbook_auto_from_rs(cursor)
            .map_err(|e| AppError::bad_request(format!("无法解析 xlsx 文件：{}", e)))?;

        let sheet_name = workbook
            .sheet_names()
            .first()
            .cloned()
            .ok_or_else(|| AppError::bad_request("xlsx 文件无工作表".to_string()))?;
        let range = workbook
            .worksheet_range(&sheet_name)
            .map_err(|e| AppError::bad_request(format!("读取工作表失败：{}", e)))?;

        // 第一行为表头，跳过
        let mut rows = range.rows();
        let _header = rows.next();
        Ok(rows.map(|r| r.to_vec()).collect())
    }

    /// 从行数据中提取指定列的字符串值
    fn extract_cell_string(row: &[calamine::Data], i: usize) -> Option<String> {
        row.get(i).and_then(|c| match c {
            calamine::Data::String(s) => {
                let t = s.trim();
                if t.is_empty() {
                    None
                } else {
                    Some(t.to_string())
                }
            }
            calamine::Data::Int(n) => Some(n.to_string()),
            calamine::Data::Float(f) => Some(f.to_string()),
            calamine::Data::DateTimeIso(s) | calamine::Data::DurationIso(s) => Some(s.clone()),
            _ => None,
        })
    }

    /// 根据行数据构造 CreateLeadRequest
    fn build_lead_request_from_row(
        row: &[calamine::Data],
    ) -> crate::models::dto::crm_dto::CreateLeadRequest {
        crate::models::dto::crm_dto::CreateLeadRequest {
            lead_no: Self::extract_cell_string(row, 0),
            lead_source: Self::extract_cell_string(row, 7),
            lead_status: Self::extract_cell_string(row, 8),
            company_name: Self::extract_cell_string(row, 1),
            contact_name: Self::extract_cell_string(row, 2),
            contact_title: Self::extract_cell_string(row, 3),
            mobile_phone: Self::extract_cell_string(row, 4),
            tel_phone: Self::extract_cell_string(row, 5),
            email: Self::extract_cell_string(row, 6),
            wechat: None,
            qq: None,
            address: None,
            product_interest: None,
            estimated_quantity: None,
            estimated_amount: None,
            expected_delivery_date: None,
            requirement_desc: None,
            priority: Self::extract_cell_string(row, 10),
            rating: None,
            tags: None,
        }
    }

    /// 批量导入线索（v11 批次 157d-4 新增）：解析 xlsx 字节并逐行创建线索
    /// xlsx 列顺序与 export_leads 一致：线索编号/公司名称/联系人/职位/手机号/座机/邮箱/线索来源/线索状态/负责人/优先级/创建时间；失败行不影响其他行，最终返回成功/失败统计与错误详情
    ///
    /// `operator_name`：真实操作人登录名，由入口 handler 传 `&auth.username`，一次性
    /// 透传给每一行 `create_lead` 落 `owner_name`——禁止为取名字在 `create_lead` 内逐行查库。
    pub async fn import_leads(
        &self,
        file_bytes: Vec<u8>,
        user_id: i32,
        operator_name: &str,
    ) -> Result<crate::models::dto::crm_dto::ImportLeadsResult, AppError> {
        let data_rows = Self::read_xlsx_rows(file_bytes).await?;
        let total = data_rows.len() as u32;
        let mut success_count: u32 = 0;
        let mut errors: Vec<crate::models::dto::crm_dto::ImportLeadError> = Vec::new();

        for (idx, row) in data_rows.iter().enumerate() {
            let row_no = (idx + 2) as u32; // 行号从 2 开始（1 为表头）
            let req = Self::build_lead_request_from_row(row);
            match self.create_lead(req, user_id, operator_name).await {
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
        // 行级数据权限校验（IDOR 防护）：owner=owner_id，dept=lead.department_id
        //（m_rls_dept_domain，与 RLS 策略口径一致）
        if let Some(ctx) = data_scope {
            if !check_resource_owner(ctx, Some(lead.owner_id), lead.department_id) {
                return Err(AppError::permission_denied(format!(
                    "无权访问线索 {}（数据范围限制）",
                    lead_id
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
        // 取值校验：与 create_lead 同口径，非法状态拒绝，不裸落库
        if let Some(s) = &req.lead_status {
            Self::ensure_valid_lead_status(s)?;
        }
        let mut lead_active: crm_lead::ActiveModel = lead.into();

        Self::apply_lead_update_fields(&mut lead_active, req);
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

    /// 应用线索更新字段到 ActiveModel（消费 req 各 Option 字段）
    fn apply_lead_update_fields(
        lead_active: &mut crm_lead::ActiveModel,
        req: crate::models::dto::crm_dto::UpdateLeadRequest,
    ) {
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
        // batch-15 P3: 线索自定义字段
        if let Some(v) = req.custom_fields {
            lead_active.custom_fields = Set(Some(v));
        }
    }

    /// 删除线索
    ///
    /// 删除前先做引用校验：`crm_opportunity.lead_id` 通过外键 `fk_crm_opportunity_lead`
    /// 引用 `crm_lead.id` 且无 `ON DELETE` 动作。若该线索已被任一存活商机引用，直接物理删除会
    /// 命中 FK 约束并被裸映射成 500 `DATABASE_ERROR`（违反失败信封只能单一 `AppError` 形状的红线）。
    /// 此处以与 DB 约束同口径的“是否存在引用”预校验，命中则返回可外显业务错误（HTTP 400 / `BUSINESS_ERROR`）。
    /// 说明：`crm_lead` 表无软删列，线索状态词表亦无 `deleted/cancelled` 态，故维持硬删语义——
    /// 已转商机的线索禁止直接删除，由调用方先处理关联商机（行业惯例口径，是否改软删保留引用待产品确认）。
    pub async fn delete_lead(&self, lead_id: i32, user_id: i32) -> Result<(), AppError> {
        // 引用预校验：与 fk_crm_opportunity_lead 同口径——只要存在任意一条引用该线索的商机即拒绝删除
        let referencing_opportunities = crm_opportunity::Entity::find()
            .filter(crm_opportunity::Column::LeadId.eq(lead_id))
            .count(&*self.db)
            .await?;
        if referencing_opportunities > 0 {
            return Err(AppError::business_displayable(
                "该线索已转商机，不可删除，请先处理关联商机",
            ));
        }

        // P0 8-3 修复：delete 操作补审计日志
        // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
        let result = crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            crm_lead::Entity,
            _,
        >(&*self.db, "crm_lead", lead_id, Some(user_id))
        .await;

        // 竞态兜底：预校验通过后，仍可能在删除瞬间被并发新建的引用商机命中 FK 约束，
        // 此时 delete_with_audit 内部经 From<DbErr> 把它映射成 DatabaseError(DB_RELATION)（500）。
        // 降级为与预校验一致的业务错误（400），确保失败信封不会外泄为 500。
        if let Err(e) = result {
            if matches!(e, AppError::DatabaseError(ref msg) if msg == err_msg::DB_RELATION) {
                tracing::warn!(
                    "线索删除命中外键约束（并发引用商机，lead_id={}）：降级为业务错误",
                    lead_id
                );
                return Err(AppError::business_displayable(
                    "该线索已转商机，不可删除，请先处理关联商机",
                ));
            }
            return Err(e);
        }
        Ok(())
    }

    /// 校验线索状态取值属于权威词表（models/status::crm_lead::ALL），
    /// 非法值返回 ValidationError（400），错误信息携带非法值与合法取值列表。；
    /// create_lead / update_lead / update_lead_status 三个写入口共用。
    fn ensure_valid_lead_status(status: &str) -> Result<(), AppError> {
        if !lead_status::ALL.contains(&status) {
            return Err(AppError::validation_displayable(format!(
                "非法线索状态 '{}'，合法取值为：{}",
                status,
                lead_status::ALL.join("/")
            )));
        }
        Ok(())
    }

    /// 更新线索状态
    pub async fn update_lead_status(
        &self,
        lead_id: i32,
        status: &str,
        user_id: i32,
    ) -> Result<(), AppError> {
        // 取值校验：不再原样落库，非法状态直接拒绝（DB 侧兜底见 chk_crm_lead_lead_status）
        Self::ensure_valid_lead_status(status)?;
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

    // ===== convert_lead_to_customer 私有 helpers（D08 拆分）=====

    /// 校验线索存在且未转换
    async fn validate_lead_for_conversion(
        &self,
        lead_id: i32,
    ) -> Result<crm_lead::Model, AppError> {
        let lead = self.get_lead(lead_id, None).await?;
        if lead.lead_status.as_deref() == Some(lead_status::CONVERTED) {
            return Err(AppError::business("线索已转换为客户".to_string()));
        }
        Ok(lead)
    }

    /// 从线索构造客户 ActiveModel（纯函数，无 IO；customer_code 由调用方经统一生成器取号后传入）
    fn build_customer_active(
        lead: &crm_lead::Model,
        req: &crate::models::dto::crm_dto::ConvertLeadRequest,
        customer_name: &str,
        user_id: i32,
        customer_code: String,
    ) -> customer::ActiveModel {
        // ⚠️ 已登记的大写脏值来源（波0 契约测钉死，值本波**不改**，等业务口径裁定）：
        // - 缺省 "POTENTIAL" 不在唯一词表 `constants::customer_type::ALLOWED` 内
        //   （该集合逐字符等于标准入口现行五值白名单，收编与否属待裁项）；
        // - `potential` 这一 token 已被 CLV 分层占用（services/crm/cust.rs:670-680
        //   segment 词表 champion/loyal/potential/at_risk/lost；
        //   models/customer_lifetime_value.rs:40）⇒ 属"渠道 vs 分层"**混维撞名**；
        // - 显式提供的 customer_type 已在 handler 层（crm_handler.rs convert_lead）经
        //   `constants::customer_type::validate` 拒绝非法值，此处只承接缺省分支。
        // 待裁后再定：改小写、并入 ALLOWED 或删除缺省（届时同步更新契约测登记锁）。
        let customer_type = req
            .customer_type
            .clone()
            .unwrap_or_else(|| "POTENTIAL".to_string());
        customer::ActiveModel {
            id: Default::default(),
            customer_code: Set(customer_code),
            customer_name: Set(customer_name.to_string()),
            contact_person: Set(Some(lead.contact_name.clone())),
            contact_phone: Set(lead.mobile_phone.clone().or(lead.tel_phone.clone())),
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
            customer_industry: Set(None),
            main_products: Set(None),
            annual_purchase: Set(None),
            quality_requirement: Set(None),
            inspection_standard: Set(None),
            owner_id: Set(lead.owner_id),
            owner_assigned_at: Set(Some(chrono::Utc::now())),
            // batch-13 P3：客户特殊工艺和来源字段
            special_process: Set(None),
            source: Set(Some("lead".to_string())),
            pool_recycle_reason: Set(None),
            // m_rls_dept_domain：department_id 由 trg_customers_dept 触发器自动维护
            department_id: sea_orm::ActiveValue::NotSet,
        }
    }

    /// 更新线索状态为已转换（含审计日志）
    async fn mark_lead_converted(
        txn: &sea_orm::DatabaseTransaction,
        lead: &crm_lead::Model,
        customer_id: i32,
        user_id: i32,
    ) -> Result<(), AppError> {
        let mut lead_active: crm_lead::ActiveModel = lead.clone().into();
        lead_active.lead_status = Set(Some(lead_status::CONVERTED.to_string()));
        lead_active.converted_customer_id = Set(Some(customer_id));
        lead_active.converted_at = Set(Some(chrono::Utc::now()));
        lead_active.updated_at = Set(Some(chrono::Utc::now()));
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            lead_active,
            Some(user_id),
        )
        .await?;
        Ok(())
    }

    /// 从线索构造初步接洽商机 ActiveModel（纯函数，无 IO；opportunity_no 由调用方经统一生成器取号后传入）
    fn build_opportunity_active(
        lead: &crm_lead::Model,
        customer_id: i32,
        customer_name: &str,
        user_id: i32,
        opportunity_no: String,
    ) -> crm_opportunity::ActiveModel {
        let opportunity_name = format!("{} - 初步接洽", customer_name);
        crm_opportunity::ActiveModel {
            id: Default::default(),
            opportunity_no: Set(opportunity_no),
            opportunity_name: Set(opportunity_name),
            customer_id: Set(customer_id),
            lead_id: Set(Some(lead.id)),
            opportunity_type: Set(Some("NEW".to_string())),
            opportunity_stage: Set(Some(opp_status::QUALIFICATION.to_string())),
            win_probability: Set(Some(rust_decimal::Decimal::new(20, 0))),
            estimated_amount: Set(lead.estimated_amount),
            actual_amount: Set(None),
            currency: Set(Some(crate::constants::DEFAULT_CURRENCY.to_string())),
            expected_close_date: Set(lead.expected_delivery_date),
            actual_close_date: Set(None),
            product_ids: Set(None),
            product_names: Set(None),
            product_desc: Set(lead.product_interest.clone()),
            owner_id: Set(lead.owner_id),
            owner_name: Set(lead.owner_name.clone()),
            opportunity_status: Set(Some(opp_status::OPEN.to_string())),
            created_by: Set(Some(user_id)),
            created_at: Set(Some(chrono::Utc::now())),
            updated_at: Set(Some(chrono::Utc::now())),
            ..Default::default()
        }
    }

    /// 将线索转换为客户（同时创建一条对应的"初步接洽"商机）
    pub async fn convert_lead_to_customer(
        &self,
        lead_id: i32,
        req: crate::models::dto::crm_dto::ConvertLeadRequest,
        user_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        let lead = self.validate_lead_for_conversion(lead_id).await?;
        let txn = self.db.begin().await?;
        let customer_name = lead
            .company_name
            .clone()
            .unwrap_or_else(|| lead.contact_name.clone());
        // 客户编码：统一生成器（CUS{YYYYMMDD}{3位流水}，与 customer_ops::generate_customer_code 同源）
        let customer_code =
            crate::utils::number_generator::DocumentNumberGenerator::generate_no_with_txn(
                &txn,
                "CUS",
                customer::Entity,
                customer::Column::CustomerCode,
            )
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "线索转换客户编码生成失败");
                AppError::business_displayable("客户编码生成失败，请稍后重试")
            })?;
        let new_customer =
            Self::build_customer_active(&lead, &req, &customer_name, user_id, customer_code)
                .insert(&txn)
                .await?;
        Self::mark_lead_converted(&txn, &lead, new_customer.id, user_id).await?;
        // 商机编号：统一生成器（OPP{YYYYMMDD}{3位流水}，与 crm/opp::create_opportunity 同源）
        let opportunity_no =
            crate::utils::number_generator::DocumentNumberGenerator::generate_no_with_txn(
                &txn,
                "OPP",
                crm_opportunity::Entity,
                crm_opportunity::Column::OpportunityNo,
            )
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "线索转换商机编号生成失败");
                AppError::business_displayable("商机编号生成失败，请稍后重试")
            })?;
        Self::build_opportunity_active(
            &lead,
            new_customer.id,
            &customer_name,
            user_id,
            opportunity_no,
        )
        .insert(&txn)
        .await?;
        txn.commit().await?;
        Ok(serde_json::json!({
            "customer_id": new_customer.id,
            "customer_code": new_customer.customer_code,
            "customer_name": new_customer.customer_name,
        }))
    }

    /// V15 P1 18.1-D1：线索评分
    /// 基于来源/行为/demographics 多维加权评分（0-100）：来源维度（最高 30 分）：REFERRAL=30, EXHIBITION=25, WEBSITE=20, AD=15, OTHER=10；行为维度（最高 40 分）：有预估金额 +15，有产品兴趣 +10，有需求描述 +10，有交付日期 +5；demographics 维度（最高 30 分）：有公司名 +10，有手机号 +10，有邮箱 +5，有职位 +5；评分写入 crm_lead.rating 列，>60 标记为高优先级。
    pub async fn score_lead(&self, lead_id: i32) -> Result<LeadScoreResult, AppError> {
        let lead = crm_lead::Entity::find_by_id(lead_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("线索不存在：{}", lead_id)))?;

        let mut score: i32 = 0;
        let mut breakdown = serde_json::Map::new();

        // 来源维度（最高 30 分）
        let source_score = match lead.lead_source.as_str() {
            "REFERRAL" => 30,
            "EXHIBITION" => 25,
            "WEBSITE" => 20,
            "AD" => 15,
            _ => 10,
        };
        score += source_score;
        breakdown.insert(
            "source".into(),
            serde_json::json!({"score": source_score, "lead_source": lead.lead_source.clone()}),
        );

        // 行为维度（最高 40 分）
        let mut behavior_score = 0;
        if lead.estimated_amount.is_some() {
            behavior_score += 15;
        }
        if lead.product_interest.is_some() {
            behavior_score += 10;
        }
        if lead.requirement_desc.is_some() {
            behavior_score += 10;
        }
        if lead.expected_delivery_date.is_some() {
            behavior_score += 5;
        }
        score += behavior_score;
        breakdown.insert(
            "behavior".into(),
            serde_json::json!({"score": behavior_score}),
        );

        // demographics 维度（最高 30 分）
        let mut demo_score = 0;
        if lead.company_name.is_some() {
            demo_score += 10;
        }
        if lead.mobile_phone.is_some() {
            demo_score += 10;
        }
        if lead.email.is_some() {
            demo_score += 5;
        }
        if lead.contact_title.is_some() {
            demo_score += 5;
        }
        score += demo_score;
        breakdown.insert(
            "demographics".into(),
            serde_json::json!({"score": demo_score}),
        );

        // 评分封顶 100
        let final_score = std::cmp::Ord::min(score, 100);

        // 更新线索评分与优先级
        let mut lead_active: crm_lead::ActiveModel = lead.into();
        lead_active.rating = Set(Some(final_score));
        // 评分 >60 标记为高优先级，>80 标记为紧急
        let new_priority = if final_score >= 80 {
            "urgent"
        } else if final_score >= 60 {
            "high"
        } else if final_score >= 30 {
            "medium"
        } else {
            "low"
        };
        lead_active.priority = Set(Some(new_priority.to_string()));
        lead_active.updated_at = Set(Some(chrono::Utc::now()));
        lead_active.update(&*self.db).await?;

        Ok(LeadScoreResult {
            lead_id,
            score: final_score,
            priority: new_priority.to_string(),
            breakdown: serde_json::Value::Object(breakdown),
        })
    }

    /// V15 P1 18.1-D2：线索去重检测（按手机号/公司名检测重复线索，返回重复组列表。；手机号完全匹配或公司名完全匹配（忽略前后空格+大小写）视为重复。）
    ///
    /// 行级数据权限（`data_scope`）为硬约束：出参组内含 `lead_no`/`company_names`，
    /// 不注入 scope 时任意登录用户可用一个手机号枚举该号下**他人名下**线索的编号与公司名
    /// （越权读）。两次查询均套用与 `list_leads` **同一个**
    /// `apply_department_scope_with_pool`（owner=OwnerId，dept=DepartmentId，公海放行同口径），
    /// 使可见集与列表严格一致。被过滤掉的行不进组：过滤后不足 2 条不构成重复组
    /// （`leads.len() > 1` 判据不变）。`match_key` 回显调用方自己提交的号码，非服务端查得数据。
    pub async fn detect_duplicate_leads(
        &self,
        mobile_phone: Option<&str>,
        company_name: Option<&str>,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<Vec<DuplicateLeadGroup>, AppError> {
        let mut groups: Vec<DuplicateLeadGroup> = Vec::new();

        // 按手机号去重
        if let Some(mobile) = mobile_phone {
            if !mobile.trim().is_empty() {
                let mut q = crm_lead::Entity::find()
                    .filter(crm_lead::Column::MobilePhone.eq(mobile))
                    .filter(crm_lead::Column::LeadStatus.is_not_null());
                // 行级数据权限：与 list_leads（本文件）同一个 scope 函数与同一放行条件
                if let Some(ctx) = data_scope {
                    q = apply_department_scope_with_pool(
                        q,
                        ctx,
                        crm_lead::Column::OwnerId,
                        crm_lead::Column::DepartmentId,
                        crm_lead::Column::LeadStatus.eq(lead_status::POOL),
                        PoolVisibility::Open,
                    );
                }
                let leads = q.all(&*self.db).await?;
                if leads.len() > 1 {
                    groups.push(DuplicateLeadGroup {
                        match_key: format!("mobile:{}", mobile),
                        match_type: "mobile_phone".to_string(),
                        lead_ids: leads.iter().map(|l| l.id).collect(),
                        lead_nos: leads.iter().map(|l| l.lead_no.clone()).collect(),
                        company_names: leads
                            .iter()
                            .map(|l| l.company_name.clone().unwrap_or_default())
                            .collect(),
                        count: leads.len() as i32,
                    });
                }
            }
        }

        // 按公司名去重（忽略大小写）
        if let Some(company) = company_name {
            let company_trimmed = company.trim();
            if !company_trimmed.is_empty() {
                let mut q = crm_lead::Entity::find()
                    .filter(Expr::col(crm_lead::Column::CompanyName).ilike(company_trimmed));
                // 行级数据权限：与手机号分支、list_leads 同一个 scope 函数与同一放行条件
                if let Some(ctx) = data_scope {
                    q = apply_department_scope_with_pool(
                        q,
                        ctx,
                        crm_lead::Column::OwnerId,
                        crm_lead::Column::DepartmentId,
                        crm_lead::Column::LeadStatus.eq(lead_status::POOL),
                        PoolVisibility::Open,
                    );
                }
                let leads = q.all(&*self.db).await?;
                if leads.len() > 1 {
                    groups.push(DuplicateLeadGroup {
                        match_key: format!("company:{}", company_trimmed),
                        match_type: "company_name".to_string(),
                        lead_ids: leads.iter().map(|l| l.id).collect(),
                        lead_nos: leads.iter().map(|l| l.lead_no.clone()).collect(),
                        company_names: leads
                            .iter()
                            .map(|l| l.company_name.clone().unwrap_or_default())
                            .collect(),
                        count: leads.len() as i32,
                    });
                }
            }
        }

        Ok(groups)
    }

    /// V15 P1 18.1-D2：合并重复线索（将多个重复线索合并到主线索（保留主线索数据，副线索标记为 lost 并记录合并原因）。）
    ///
    /// 行级数据权限（`data_scope`）是**写路径**硬约束：合并不可逆，不注入 scope 时
    /// 任何用户都可把自己看不到的他人线索合并掉（越权写）。主线索与每一条重复线索
    /// （存在行）都必须通过**写侧归属门**——方案 A（用户 2026-10-02 裁定）：读可 All、
    /// 写须 owner 本人或持有 `crm/cross_owner_write` 代表键（`behalf_granted`，由 handler
    /// 经 `crm_write_guard::cross_owner_write_behalf_granted` 查得，N 行只查一次键），
    /// 判定源 = `utils::data_scope::check_resource_write_owner`（Dept 代管本职无需键、
    /// Self_ 仅本人行；**读门 `check_resource_owner` 不得复用为写门**——那正是本缺陷成因）。
    /// **任一行未过写门即整笔拒绝**（403），禁止"跳过不可见行继续合并其余"的静默降级；
    /// 预校验全部通过后才落写，保证拒绝时零漂移。真实原因只进日志
    /// （`AppError::permission_denied` 出参恒为固定脱敏文案 + FORBIDDEN 码）。
    /// 代他人合并（All + 键 + 非本人行）逐行打 `tracing::info!` 留痕，与单行写入口
    /// `ensure_cross_owner_write_allowed` 同构口径。
    pub async fn merge_leads(
        &self,
        master_lead_id: i32,
        duplicate_lead_ids: Vec<i32>,
        user_id: i32,
        data_scope: Option<&DataScopeContext>,
        behalf_granted: bool,
    ) -> Result<MergeResult, AppError> {
        let txn = self.db.begin().await?;

        // 校验主线索存在
        let master = crm_lead::Entity::find_by_id(master_lead_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("主线索不存在：{}", master_lead_id)))?;

        // 预校验（先判后写）：主线索必须过**写侧**归属门（方案 A）
        if let Some(ctx) = data_scope {
            if !check_resource_write_owner(
                ctx,
                Some(master.owner_id),
                master.department_id,
                behalf_granted,
            ) {
                tracing::warn!(
                    actor = ctx.user_id,
                    lead_id = master_lead_id,
                    resource_owner = master.owner_id,
                    "合并主线索未过跨 owner 写门，整笔拒绝（原因不外显）"
                );
                return Err(AppError::permission_denied(format!(
                    "无权合并线索 {}（数据范围限制：主线索非本人行且未持代操作授权）",
                    master_lead_id
                )));
            }
            if behalf_granted && master.owner_id != ctx.user_id && ctx.scope == DataScope::All {
                tracing::info!(
                    actor = ctx.user_id,
                    resource = "线索合并（主线索）",
                    resource_owner = master.owner_id,
                    "代操作写放行（方案 A：All 范围 + 显式 cross_owner_write 键，合并入口）"
                );
            }
        }

        // 预校验每一条重复线索（存在的行）均过写侧归属门并收集待合并行；
        // 任一行被拒即整笔拒绝，此时尚无任何写操作，天然零漂移
        let mut dup_targets: Vec<crm_lead::Model> = Vec::new();
        for dup_id in &duplicate_lead_ids {
            if *dup_id == master_lead_id {
                continue;
            }
            let dup_lead = crm_lead::Entity::find_by_id(*dup_id)
                .lock_exclusive()
                .one(&txn)
                .await?;
            if let Some(dup) = dup_lead {
                if let Some(ctx) = data_scope {
                    if !check_resource_write_owner(
                        ctx,
                        Some(dup.owner_id),
                        dup.department_id,
                        behalf_granted,
                    ) {
                        tracing::warn!(
                            actor = ctx.user_id,
                            lead_id = *dup_id,
                            resource_owner = dup.owner_id,
                            "合并重复线索未过跨 owner 写门，整笔拒绝（原因不外显）"
                        );
                        return Err(AppError::permission_denied(format!(
                            "无权合并线索 {}（数据范围限制：重复线索非本人行且未持代操作授权）",
                            dup_id
                        )));
                    }
                    if behalf_granted && dup.owner_id != ctx.user_id && ctx.scope == DataScope::All
                    {
                        tracing::info!(
                            actor = ctx.user_id,
                            resource = "线索合并（重复线索）",
                            resource_owner = dup.owner_id,
                            "代操作写放行（方案 A：All 范围 + 显式 cross_owner_write 键，合并入口）"
                        );
                    }
                }
                dup_targets.push(dup);
            }
        }

        let mut merged_count = 0i32;
        let mut merged_lead_nos = Vec::new();

        for dup in dup_targets {
            let mut dup_active: crm_lead::ActiveModel = dup.into();
            dup_active.lead_status = Set(Some(lead_status::LOST.to_string()));
            dup_active.lost_reason = Set(Some(format!(
                "合并到主线索 {} ({})",
                master.lead_no, master_lead_id
            )));
            dup_active.updated_at = Set(Some(chrono::Utc::now()));
            dup_active.updated_by = Set(Some(user_id));
            let updated = dup_active.update(&txn).await?;
            merged_lead_nos.push(updated.lead_no);
            merged_count += 1;
        }

        txn.commit().await?;

        Ok(MergeResult {
            master_lead_id,
            master_lead_no: master.lead_no.clone(),
            merged_count,
            merged_lead_nos,
        })
    }

    /// V15 P1 18.1-D3：转化漏斗报表（统计线索→商机→客户→订单各阶段数量与转化率。）
    pub async fn lead_funnel_report(
        &self,
        start_date: Option<chrono::NaiveDate>,
        end_date: Option<chrono::NaiveDate>,
    ) -> Result<LeadFunnelReport, AppError> {
        use crate::models::{crm_opportunity, sales_order};

        // 线索总数
        let mut lead_query = crm_lead::Entity::find();
        if let Some(start) = start_date {
            lead_query = lead_query.filter(crm_lead::Column::CreatedAt.gte(
                chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                    start.and_hms_opt(0, 0, 0).unwrap_or_default(),
                    chrono::Utc,
                ),
            ));
        }
        if let Some(end) = end_date {
            lead_query = lead_query.filter(crm_lead::Column::CreatedAt.lte(
                chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                    end.and_hms_opt(23, 59, 59).unwrap_or_default(),
                    chrono::Utc,
                ),
            ));
        }
        let total_leads = lead_query.clone().count(&*self.db).await?;

        // 已转化线索数
        let converted_leads = lead_query
            .filter(crm_lead::Column::LeadStatus.eq(lead_status::CONVERTED))
            .count(&*self.db)
            .await?;

        // 商机总数
        let mut opp_query = crm_opportunity::Entity::find();
        if let Some(start) = start_date {
            opp_query =
                opp_query.filter(crm_opportunity::Column::CreatedAt.gte(chrono::DateTime::<
                    chrono::Utc,
                >::from_naive_utc_and_offset(
                    start.and_hms_opt(0, 0, 0).unwrap_or_default(),
                    chrono::Utc,
                )));
        }
        if let Some(end) = end_date {
            opp_query =
                opp_query.filter(crm_opportunity::Column::CreatedAt.lte(chrono::DateTime::<
                    chrono::Utc,
                >::from_naive_utc_and_offset(
                    end.and_hms_opt(23, 59, 59).unwrap_or_default(),
                    chrono::Utc,
                )));
        }
        let total_opportunities = opp_query.clone().count(&*self.db).await?;

        // 已成交商机数
        let won_opportunities = opp_query
            .filter(crm_opportunity::Column::OpportunityStage.eq(opp_status::CLOSED_WON))
            .count(&*self.db)
            .await?;

        // 客户总数
        let mut cust_query = customer::Entity::find();
        if let Some(start) = start_date {
            cust_query = cust_query.filter(customer::Column::CreatedAt.gte(
                chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                    start.and_hms_opt(0, 0, 0).unwrap_or_default(),
                    chrono::Utc,
                ),
            ));
        }
        if let Some(end) = end_date {
            cust_query = cust_query.filter(customer::Column::CreatedAt.lte(
                chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                    end.and_hms_opt(23, 59, 59).unwrap_or_default(),
                    chrono::Utc,
                ),
            ));
        }
        let total_customers = cust_query.count(&*self.db).await?;

        // 订单总数
        let mut order_query = sales_order::Entity::find();
        if let Some(start) = start_date {
            order_query =
                order_query.filter(sales_order::Column::CreatedAt.gte(chrono::DateTime::<
                    chrono::Utc,
                >::from_naive_utc_and_offset(
                    start.and_hms_opt(0, 0, 0).unwrap_or_default(),
                    chrono::Utc,
                )));
        }
        if let Some(end) = end_date {
            order_query =
                order_query.filter(sales_order::Column::CreatedAt.lte(chrono::DateTime::<
                    chrono::Utc,
                >::from_naive_utc_and_offset(
                    end.and_hms_opt(23, 59, 59).unwrap_or_default(),
                    chrono::Utc,
                )));
        }
        let total_orders = order_query.count(&*self.db).await?;

        // 计算转化率
        let lead_to_opp_rate = if total_leads > 0 {
            (total_opportunities as f64 / total_leads as f64) * 100.0
        } else {
            0.0
        };
        let opp_to_customer_rate = if total_opportunities > 0 {
            (total_customers as f64 / total_opportunities as f64) * 100.0
        } else {
            0.0
        };
        let opp_to_order_rate = if total_opportunities > 0 {
            (total_orders as f64 / total_opportunities as f64) * 100.0
        } else {
            0.0
        };
        let overall_conversion_rate = if total_leads > 0 {
            (total_customers as f64 / total_leads as f64) * 100.0
        } else {
            0.0
        };

        Ok(LeadFunnelReport {
            total_leads: total_leads as i64,
            converted_leads: converted_leads as i64,
            total_opportunities: total_opportunities as i64,
            won_opportunities: won_opportunities as i64,
            total_customers: total_customers as i64,
            total_orders: total_orders as i64,
            lead_to_opp_rate,
            opp_to_customer_rate,
            opp_to_order_rate,
            overall_conversion_rate,
        })
    }

    /// V15 P2 18.1-D4: 渠道 ROI 分析报表（统计各来源的线索数、转化数、收入、ROI）
    pub async fn channel_roi_report(
        &self,
        start_date: chrono::NaiveDate,
        end_date: chrono::NaiveDate,
    ) -> Result<Vec<ChannelRoiItem>, AppError> {
        use crate::models::lead_source_roi;

        let items = lead_source_roi::Entity::find()
            .filter(lead_source_roi::Column::PeriodStart.gte(start_date))
            .filter(lead_source_roi::Column::PeriodEnd.lte(end_date))
            .order_by(lead_source_roi::Column::Roi, sea_orm::Order::Desc)
            .all(&*self.db)
            .await?;

        Ok(items
            .into_iter()
            .map(|m| ChannelRoiItem {
                source: m.source,
                cost: m.cost,
                lead_count: m.lead_count,
                converted_count: m.converted_count,
                opportunity_count: m.opportunity_count,
                order_count: m.order_count,
                revenue: m.revenue,
                conversion_rate: m.conversion_rate,
                roi: m.roi,
            })
            .collect())
    }

    /// V15 P2 18.1-D4: 计算并记录渠道 ROI（汇总指定周期内的线索转化数据）
    pub async fn calculate_channel_roi(
        &self,
        source: &str,
        start_date: chrono::NaiveDate,
        end_date: chrono::NaiveDate,
        cost: rust_decimal::Decimal,
    ) -> Result<crate::models::lead_source_roi::Model, AppError> {
        use crate::models::{crm_opportunity, lead_source_roi, sales_order};

        let start_dt = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
            start_date.and_hms_opt(0, 0, 0).unwrap_or_default(),
            chrono::Utc,
        );
        let end_dt = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
            end_date.and_hms_opt(23, 59, 59).unwrap_or_default(),
            chrono::Utc,
        );

        // 统计线索数
        let lead_count = crm_lead::Entity::find()
            .filter(crm_lead::Column::LeadSource.eq(source))
            .filter(crm_lead::Column::CreatedAt.gte(start_dt))
            .filter(crm_lead::Column::CreatedAt.lte(end_dt))
            .count(&*self.db)
            .await? as i32;

        // 统计转化客户数
        let converted_count = crm_lead::Entity::find()
            .filter(crm_lead::Column::LeadSource.eq(source))
            .filter(crm_lead::Column::LeadStatus.eq(lead_status::CONVERTED))
            .filter(crm_lead::Column::ConvertedAt.is_not_null())
            .filter(crm_lead::Column::ConvertedAt.gte(start_dt))
            .filter(crm_lead::Column::ConvertedAt.lte(end_dt))
            .count(&*self.db)
            .await? as i32;

        // 统计商机数
        let opportunity_count = crm_opportunity::Entity::find()
            .filter(crm_opportunity::Column::CreatedAt.gte(start_dt))
            .filter(crm_opportunity::Column::CreatedAt.lte(end_dt))
            .count(&*self.db)
            .await? as i32;

        // 统计订单数和金额
        let orders = sales_order::Entity::find()
            .filter(sales_order::Column::CreatedAt.gte(start_dt))
            .filter(sales_order::Column::CreatedAt.lte(end_dt))
            .all(&*self.db)
            .await?;
        let order_count = orders.len() as i32;
        let revenue: rust_decimal::Decimal = orders.iter().map(|o| o.total_amount).sum();

        // 计算转化率和 ROI
        let conversion_rate = if lead_count > 0 {
            rust_decimal::Decimal::from(converted_count) / rust_decimal::Decimal::from(lead_count)
                * rust_decimal::Decimal::from(100)
        } else {
            rust_decimal::Decimal::ZERO
        };
        let roi = if cost > rust_decimal::Decimal::ZERO {
            (revenue - cost) / cost * rust_decimal::Decimal::from(100)
        } else {
            rust_decimal::Decimal::ZERO
        };

        // 保存记录
        let new_record = lead_source_roi::ActiveModel {
            id: Default::default(),
            source: sea_orm::Set(source.to_string()),
            period_start: sea_orm::Set(start_date),
            period_end: sea_orm::Set(end_date),
            cost: sea_orm::Set(cost),
            lead_count: sea_orm::Set(lead_count),
            converted_count: sea_orm::Set(converted_count),
            opportunity_count: sea_orm::Set(opportunity_count),
            order_count: sea_orm::Set(order_count),
            revenue: sea_orm::Set(revenue),
            conversion_rate: sea_orm::Set(conversion_rate),
            roi: sea_orm::Set(roi),
            created_at: sea_orm::Set(Some(chrono::Utc::now())),
        }
        .insert(&*self.db)
        .await?;

        Ok(new_record)
    }

    /// V15 P2 18.1-D5: 创建线索分配规则
    pub async fn create_allocation_rule(
        &self,
        req: CreateAllocationRuleRequest,
    ) -> Result<crate::models::lead_allocation_rule::Model, AppError> {
        use crate::models::lead_allocation_rule;

        let new_rule = lead_allocation_rule::ActiveModel {
            id: Default::default(),
            rule_name: sea_orm::Set(req.rule_name),
            rule_type: sea_orm::Set(req.rule_type),
            source_filter: sea_orm::Set(req.source_filter),
            industry_filter: sea_orm::Set(req.industry_filter),
            region_filter: sea_orm::Set(req.region_filter),
            assigned_user_ids: sea_orm::Set(req.assigned_user_ids),
            weights: sea_orm::Set(req.weights),
            daily_limit: sea_orm::Set(req.daily_limit.unwrap_or(0)),
            priority: sea_orm::Set(req.priority.unwrap_or(0)),
            is_active: sea_orm::Set(req.is_active.unwrap_or(true)),
            created_at: sea_orm::Set(Some(chrono::Utc::now())),
            updated_at: sea_orm::Set(Some(chrono::Utc::now())),
        }
        .insert(&*self.db)
        .await?;

        Ok(new_rule)
    }

    /// V15 P2 18.1-D5: 获取线索分配规则列表
    pub async fn list_allocation_rules(
        &self,
        is_active: Option<bool>,
    ) -> Result<Vec<crate::models::lead_allocation_rule::Model>, AppError> {
        use crate::models::lead_allocation_rule;

        let mut q = lead_allocation_rule::Entity::find();
        if let Some(active) = is_active {
            q = q.filter(lead_allocation_rule::Column::IsActive.eq(active));
        }
        let rules = q
            .order_by(lead_allocation_rule::Column::Priority, sea_orm::Order::Desc)
            .all(&*self.db)
            .await?;
        Ok(rules)
    }

    /// V15 P2 18.1-D5: 自动分配线索（按规则匹配并分配）
    pub async fn auto_assign_lead(
        &self,
        lead_id: i32,
        source: &str,
        industry: Option<&str>,
    ) -> Result<Option<i32>, AppError> {
        use crate::models::lead_allocation_rule;

        // 获取激活的规则，按优先级排序
        let rules = lead_allocation_rule::Entity::find()
            .filter(lead_allocation_rule::Column::IsActive.eq(true))
            .order_by(lead_allocation_rule::Column::Priority, sea_orm::Order::Desc)
            .all(&*self.db)
            .await?;

        for rule in &rules {
            // 匹配来源过滤
            if let Some(ref source_filter) = rule.source_filter {
                if source_filter != source && source_filter != "*" {
                    continue;
                }
            }
            // 匹配行业过滤
            if let Some(ref industry_filter) = rule.industry_filter {
                if let Some(ind) = industry {
                    if industry_filter != ind && industry_filter != "*" {
                        continue;
                    }
                }
            }

            // 匹配分配用户
            if let Some(ref user_ids) = rule.assigned_user_ids {
                if let Some(ids) = user_ids.as_array() {
                    if !ids.is_empty() {
                        // 规则匹配，执行分配
                        // 简单轮询：取第一个用户（实际应按 round_robin 或 weighted 逻辑）
                        let assigned_user = ids[0].as_i64().unwrap_or(0) as i32;
                        // 更新线索负责人
                        let lead = self.get_lead(lead_id, None).await?;
                        let mut lead_active: crm_lead::ActiveModel = lead.into();
                        lead_active.owner_id = sea_orm::Set(assigned_user);
                        lead_active.updated_at = sea_orm::Set(Some(chrono::Utc::now()));
                        lead_active.update(&*self.db).await?;
                        return Ok(Some(assigned_user));
                    }
                }
            }
        }

        Ok(None)
    }

    /// V15 P2 18.1-D6: 创建线索培育计划
    pub async fn create_nurture_plan(
        &self,
        req: CreateNurturePlanRequest,
        user_id: i32,
    ) -> Result<crate::models::lead_nurture_plan::Model, AppError> {
        use crate::models::lead_nurture_plan;

        let new_plan = lead_nurture_plan::ActiveModel {
            id: Default::default(),
            lead_id: sea_orm::Set(req.lead_id),
            plan_name: sea_orm::Set(req.plan_name),
            nurture_type: sea_orm::Set(req.nurture_type),
            trigger_condition: sea_orm::Set(req.trigger_condition),
            template_id: sea_orm::Set(req.template_id),
            scheduled_at: sea_orm::Set(req.scheduled_at),
            executed_at: sea_orm::Set(None),
            status: sea_orm::Set(Some("pending".to_string())),
            result: sea_orm::Set(None),
            created_by: sea_orm::Set(Some(user_id)),
            created_at: sea_orm::Set(Some(chrono::Utc::now())),
        }
        .insert(&*self.db)
        .await?;

        Ok(new_plan)
    }

    /// V15 P2 18.1-D6: 获取线索培育计划列表
    pub async fn list_nurture_plans(
        &self,
        lead_id: Option<i32>,
        status: Option<&str>,
    ) -> Result<Vec<crate::models::lead_nurture_plan::Model>, AppError> {
        use crate::models::lead_nurture_plan;

        let mut q = lead_nurture_plan::Entity::find();
        if let Some(lid) = lead_id {
            q = q.filter(lead_nurture_plan::Column::LeadId.eq(lid));
        }
        if let Some(s) = status {
            q = q.filter(lead_nurture_plan::Column::Status.eq(s));
        }
        let plans = q
            .order_by(lead_nurture_plan::Column::CreatedAt, sea_orm::Order::Desc)
            .all(&*self.db)
            .await?;
        Ok(plans)
    }

    /// V15 P2 18.1-D6: 执行线索培育计划
    pub async fn execute_nurture_plan(
        &self,
        plan_id: i32,
    ) -> Result<crate::models::lead_nurture_plan::Model, AppError> {
        use crate::models::lead_nurture_plan;

        let plan = lead_nurture_plan::Entity::find_by_id(plan_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("培育计划不存在：{}", plan_id)))?;

        if plan.status.as_deref() != Some("pending") {
            return Err(AppError::business(
                "培育计划状态不是 pending，无法执行".to_string(),
            ));
        }

        let mut plan_active: lead_nurture_plan::ActiveModel = plan.into();
        plan_active.status = sea_orm::Set(Some("executed".to_string()));
        plan_active.executed_at = sea_orm::Set(Some(chrono::Utc::now()));
        plan_active.result = sea_orm::Set(Some("执行成功".to_string()));
        let updated = plan_active.update(&*self.db).await?;

        Ok(updated)
    }
}

/// V15 P2 18.1-D4: 渠道 ROI 报表项
#[derive(Debug, Clone, serde::Serialize)]
pub struct ChannelRoiItem {
    pub source: String,
    pub cost: rust_decimal::Decimal,
    pub lead_count: i32,
    pub converted_count: i32,
    pub opportunity_count: i32,
    pub order_count: i32,
    pub revenue: rust_decimal::Decimal,
    pub conversion_rate: rust_decimal::Decimal,
    pub roi: rust_decimal::Decimal,
}

/// V15 P2 18.1-D5: 创建分配规则请求
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CreateAllocationRuleRequest {
    pub rule_name: String,
    pub rule_type: String,
    pub source_filter: Option<String>,
    pub industry_filter: Option<String>,
    pub region_filter: Option<String>,
    pub assigned_user_ids: Option<serde_json::Value>,
    pub weights: Option<serde_json::Value>,
    pub daily_limit: Option<i32>,
    pub priority: Option<i32>,
    pub is_active: Option<bool>,
}

/// V15 P2 18.1-D6: 创建培育计划请求
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CreateNurturePlanRequest {
    pub lead_id: i32,
    pub plan_name: String,
    pub nurture_type: String,
    pub trigger_condition: Option<String>,
    pub template_id: Option<String>,
    pub scheduled_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// V15 P1 18.1-D1：线索评分结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct LeadScoreResult {
    pub lead_id: i32,
    pub score: i32,
    pub priority: String,
    pub breakdown: serde_json::Value,
}

/// V15 P1 18.1-D2：重复线索组
#[derive(Debug, Clone, serde::Serialize)]
pub struct DuplicateLeadGroup {
    pub match_key: String,
    pub match_type: String,
    pub lead_ids: Vec<i32>,
    pub lead_nos: Vec<String>,
    pub company_names: Vec<String>,
    pub count: i32,
}

/// V15 P1 18.1-D2：合并结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct MergeResult {
    pub master_lead_id: i32,
    pub master_lead_no: String,
    pub merged_count: i32,
    pub merged_lead_nos: Vec<String>,
}

/// V15 P1 18.1-D3：线索转化漏斗报表
#[derive(Debug, Clone, serde::Serialize)]
pub struct LeadFunnelReport {
    pub total_leads: i64,
    pub converted_leads: i64,
    pub total_opportunities: i64,
    pub won_opportunities: i64,
    pub total_customers: i64,
    pub total_orders: i64,
    pub lead_to_opp_rate: f64,
    pub opp_to_customer_rate: f64,
    pub opp_to_order_rate: f64,
    pub overall_conversion_rate: f64,
}
