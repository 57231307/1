//! 报表模板 Service
//!
//! 提供报表模板的CRUD操作和持久化功能

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::services::role_permission_service::RolePermissionService;

use crate::models::report_template::{
    ActiveModel, Entity as ReportTemplateEntity, Model as ReportTemplateModel,
};
use crate::models::report_template_version::{
    ActiveModel as ReportTemplateVersionActiveModel, Entity as ReportTemplateVersionEntity,
    Model as ReportTemplateVersionModel,
};
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;

/// 报表类型到权限码的映射（缺陷 1.2 修复）
fn report_type_permission(report_type: &str) -> Option<&'static str> {
    match report_type.to_lowercase().as_str() {
        "sales" | "sales_daily" | "销售" => Some("report:sales:view"),
        "purchase" | "purchase_summary" | "采购" => Some("report:purchase:view"),
        "inventory" | "inventory_status" | "库存" => Some("report:inventory:view"),
        "financial" | "finance" | "财务" => Some("report:finance:view"),
        _ => None,
    }
}

/// 创建报表模板请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateReportTemplateRequest {
    pub name: String,
    pub code: String,
    pub report_type: String,
    pub template_id: Option<String>,
    pub category: Option<String>,
    pub data_source: Option<String>,
    pub columns: serde_json::Value,
    pub filters: Option<serde_json::Value>,
    pub parameters: Option<serde_json::Value>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub data_source_sql: Option<String>,
    pub description: Option<String>,
    pub is_public: Option<bool>,
    pub supported_formats: Option<Vec<String>>,
    /// 刷新策略（REALTIME/HOURLY/DAILY，缺陷 1.3 修复）
    pub refresh_strategy: Option<String>,
    /// 缓存 TTL 秒数（缺陷 1.3 修复）
    pub cache_ttl_seconds: Option<i32>,
}

/// 更新报表模板请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateReportTemplateRequest {
    pub name: Option<String>,
    pub report_type: Option<String>,
    pub columns: Option<serde_json::Value>,
    pub filters: Option<serde_json::Value>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub data_source_sql: Option<String>,
    pub description: Option<String>,
    pub is_public: Option<bool>,
    pub status: Option<String>,
    /// 刷新策略（REALTIME/HOURLY/DAILY，缺陷 1.3 修复）
    pub refresh_strategy: Option<String>,
    /// 缓存 TTL 秒数（缺陷 1.3 修复）
    pub cache_ttl_seconds: Option<i32>,
}

/// 报表模板查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct ReportTemplateQuery {
    pub report_type: Option<String>,
    // v11 批次 149 P2-A：接入 status filter（list 方法中默认 ACTIVE，支持传入 INACTIVE 查看已删除模板）
    pub status: Option<String>,
    pub keyword: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 报表字段定义（字段元数据绑定 DB schema，静态配置化，替代 report_enhanced_handler 硬编码 json! 字段定义）
#[derive(Debug, Clone, Serialize)]
pub struct ReportFieldDefinition {
    /// 字段名（对应 SQL 查询列名）
    pub field: &'static str,
    /// 字段标题（中文，前端展示）
    pub title: &'static str,
    /// 数据类型（string/decimal/date/datetime）
    pub data_type: &'static str,
}

/// 报表模板 Service
pub struct ReportTemplateService {
    db: Arc<DatabaseConnection>,
}

// P0-B 安全修复：DANGEROUS_KEYWORDS / SENSITIVE_TABLES 常量及配套检查方法
// （check_dangerous_keywords / check_sensitive_tables / log_sql_execution）
// 全部删除。execute_sql_report 走 SimpleQuery 协议，黑名单无法阻止分号切割攻击；
// 统一在 create / update / execute 入口拒绝 data_source_sql，彻底关闭 SQL 注入攻击面。

impl ReportTemplateService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取指定模板类型可用的字段定义
    /// 批次 128 v8 复审 P2 修复：替代 report_enhanced_handler get_available_fields 中的；硬编码 serde_json::json! 字段定义。字段元数据绑定 DB schema（sales_orders 表有；order_no 列、purchase_orders 表有 order_no 列等），不宜放数据库动态管理，；采用静态配置化模式（与 print_handler 批次 126 一致）。；支持的模板类型：sales / sales_daily / 销售：销售订单字段（订单编号/客户名称/订单日期/订单金额/状态）；purchase / purchase_summary / 采购：采购订单字段（采购单号/供应商/下单日期/采购金额/交期）；inventory / inventory_status / 库存：库存字段（产品编码/产品名称/可用库存/预留库存/仓库）；financial / finance / 财务：财务字段（付款单号/金额/付款方式/状态/创建时间）；custom / 自定义：通用字段（ID/名称/创建时间）；其他：返回通配符字段 `*`
    pub fn available_fields_for_type(template_type: &str) -> Vec<ReportFieldDefinition> {
        match template_type.to_lowercase().as_str() {
            "sales" | "sales_daily" | "销售" => vec![
                ReportFieldDefinition {
                    field: "order_no",
                    title: "订单编号",
                    data_type: "string",
                },
                ReportFieldDefinition {
                    field: "customer_name",
                    title: "客户名称",
                    data_type: "string",
                },
                ReportFieldDefinition {
                    field: "order_date",
                    title: "订单日期",
                    data_type: "date",
                },
                ReportFieldDefinition {
                    field: "total_amount",
                    title: "订单金额",
                    data_type: "decimal",
                },
                ReportFieldDefinition {
                    field: "status",
                    title: "状态",
                    data_type: "string",
                },
            ],
            "purchase" | "purchase_summary" | "采购" => vec![
                ReportFieldDefinition {
                    field: "order_no",
                    title: "采购单号",
                    data_type: "string",
                },
                ReportFieldDefinition {
                    field: "supplier_name",
                    title: "供应商",
                    data_type: "string",
                },
                ReportFieldDefinition {
                    field: "order_date",
                    title: "下单日期",
                    data_type: "date",
                },
                ReportFieldDefinition {
                    field: "total_amount",
                    title: "采购金额",
                    data_type: "decimal",
                },
                ReportFieldDefinition {
                    field: "delivery_date",
                    title: "交期",
                    data_type: "date",
                },
            ],
            "inventory" | "inventory_status" | "库存" => vec![
                ReportFieldDefinition {
                    field: "product_code",
                    title: "产品编码",
                    data_type: "string",
                },
                ReportFieldDefinition {
                    field: "product_name",
                    title: "产品名称",
                    data_type: "string",
                },
                ReportFieldDefinition {
                    field: "quantity_available",
                    title: "可用库存",
                    data_type: "decimal",
                },
                ReportFieldDefinition {
                    field: "quantity_reserved",
                    title: "预留库存",
                    data_type: "decimal",
                },
                ReportFieldDefinition {
                    field: "warehouse",
                    title: "仓库",
                    data_type: "string",
                },
            ],
            "financial" | "finance" | "财务" => vec![
                ReportFieldDefinition {
                    field: "payment_no",
                    title: "付款单号",
                    data_type: "string",
                },
                ReportFieldDefinition {
                    field: "amount",
                    title: "金额",
                    data_type: "decimal",
                },
                ReportFieldDefinition {
                    field: "payment_method",
                    title: "付款方式",
                    data_type: "string",
                },
                ReportFieldDefinition {
                    field: "status",
                    title: "状态",
                    data_type: "string",
                },
                ReportFieldDefinition {
                    field: "created_at",
                    title: "创建时间",
                    data_type: "datetime",
                },
            ],
            "custom" | "自定义" => vec![
                ReportFieldDefinition {
                    field: "id",
                    title: "ID",
                    data_type: "string",
                },
                ReportFieldDefinition {
                    field: "name",
                    title: "名称",
                    data_type: "string",
                },
                ReportFieldDefinition {
                    field: "created_at",
                    title: "创建时间",
                    data_type: "datetime",
                },
            ],
            _ => vec![ReportFieldDefinition {
                field: "*",
                title: "全部字段",
                data_type: "string",
            }],
        }
    }

    /// 创建报表模板
    pub async fn create(
        &self,
        user_id: i32,
        _role_id: Option<i32>,
        req: CreateReportTemplateRequest,
    ) -> Result<ReportTemplateModel, AppError> {
        // P0-B 安全修复：彻底关闭"自定义 SQL 报表"入口。
        // 历史实现 execute_sql_report 通过 Statement::from_string + query_all 走 SimpleQuery
        // 协议，允许多语句执行；关键词黑名单 + starts_with("SELECT") 都不能阻止分号切割，
        // 攻击者可利用 `SELECT 1; DROP TABLE ...` 实现 SQL 注入。
        // 修复策略：禁止所有角色在 create/update 中提交 data_source_sql；
        // execute_custom_report 也不再调用 execute_sql_report，统一返回功能禁用错误。
        // 后续如需 SQL 报表能力，必须改用预定义白名单模板（report_type + 模板 ID），
        // 由后端硬编码 SQL，前端仅传参数。
        if req.data_source_sql.is_some() {
            return Err(AppError::permission_denied(
                "出于安全考虑，自定义 SQL 报表功能已禁用，请使用预定义报表模板".to_string(),
            ));
        }

        // 检查编码是否已存在
        let existing = ReportTemplateEntity::find()
            .filter(crate::models::report_template::Column::Code.eq(&req.code))
            .one(&*self.db)
            .await?;

        if existing.is_some() {
            return Err(AppError::business(format!(
                "报表模板编码 {} 已存在",
                req.code
            )));
        }

        let now = Utc::now();
        let active_model = ActiveModel {
            id: Default::default(),
            template_id: Set(req.template_id),
            name: Set(req.name),
            code: Set(req.code),
            report_type: Set(req.report_type.clone()),
            category: Set(req.category),
            data_source: Set(req.data_source),
            columns: Set(req.columns),
            filters: Set(req.filters),
            parameters: Set(req.parameters),
            sort_by: Set(req.sort_by),
            sort_order: Set(req.sort_order.or(Some("asc".to_string()))),
            data_source_sql: Set(req.data_source_sql),
            description: Set(req.description),
            is_public: Set(req.is_public.unwrap_or(false)),
            supported_formats: Set(req.supported_formats.map(sea_orm::JsonValue::from)),
            status: Set("ACTIVE".to_string()),
            // 缺陷 1.1：初始版本为 1，update 时递增
            version: Set(1),
            // 缺陷 1.2：按报表类型自动绑定权限码（销售/采购/库存/财务）
            required_permission: Set(
                report_type_permission(&req.report_type).map(|s| s.to_string())
            ),
            // 缺陷 1.3：刷新策略和缓存 TTL
            refresh_strategy: Set(req.refresh_strategy),
            cache_ttl_seconds: Set(req.cache_ttl_seconds),
            created_by: Set(user_id),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let model = active_model.insert(&*self.db).await?;

        Ok(model)
    }

    /// 获取报表模板详情
    pub async fn get_by_id(
        &self,
        id: i32,
        user_id: i32,
        role_id: Option<i32>,
    ) -> Result<Option<ReportTemplateModel>, AppError> {
        let model = ReportTemplateEntity::find()
            .filter(crate::models::report_template::Column::Id.eq(id))
            .one(&*self.db)
            .await?;

        // 检查读取权限：公开或者自己创建的
        if let Some(ref t) = model {
            if !t.is_public && t.created_by != user_id {
                return Err(AppError::permission_denied("无权访问该私有报表模板"));
            }
            // 缺陷 1.2 修复：若有权限码要求，调用 RolePermissionService 校验当前用户角色
            self.check_template_permission(role_id, &t.required_permission)
                .await?;
        }

        Ok(model)
    }

    /// 更新报表模板
    pub async fn update(
        &self,
        id: i32,
        user_id: i32,
        _role_id: Option<i32>,
        req: UpdateReportTemplateRequest,
    ) -> Result<ReportTemplateModel, AppError> {
        let model = ReportTemplateEntity::find()
            .filter(crate::models::report_template::Column::Id.eq(id))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("报表模板不存在"))?;

        // 检查更新权限：只能更新自己创建的模板
        if model.created_by != user_id {
            return Err(AppError::permission_denied("只有创建者可以更新该报表模板"));
        }

        // P0-B 安全修复：禁止通过 update 提交自定义 SQL（与 create 一致）
        if req.data_source_sql.is_some() {
            return Err(AppError::permission_denied(
                "出于安全考虑，自定义 SQL 报表功能已禁用，请使用预定义报表模板".to_string(),
            ));
        }

        // 缺陷 1.1：保存历史版本快照到 report_template_versions 表，支持回滚
        let previous_version = model.version;
        let snapshot = ReportTemplateVersionActiveModel {
            template_id: Set(id),
            version: Set(previous_version),
            name: Set(model.name.clone()),
            code: Set(model.code.clone()),
            report_type: Set(model.report_type.clone()),
            category: Set(model.category.clone()),
            data_source: Set(model.data_source.clone()),
            columns: Set(model.columns.clone()),
            filters: Set(model.filters.clone()),
            parameters: Set(model.parameters.clone()),
            supported_formats: Set(model.supported_formats.clone()),
            sort_by: Set(model.sort_by.clone()),
            sort_order: Set(model.sort_order.clone()),
            data_source_sql: Set(model.data_source_sql.clone()),
            description: Set(model.description.clone()),
            is_public: Set(model.is_public),
            required_permission: Set(model.required_permission.clone()),
            snapshot_by: Set(user_id),
            snapshot_at: Set(Utc::now()),
            ..Default::default()
        };
        let _ = snapshot.insert(&*self.db).await?;
        tracing::info!(
            template_id = id,
            previous_version,
            "报表模板版本快照：update 前写入 report_template_versions 表，支持回滚"
        );

        let mut active_model: ActiveModel = model.into();

        if let Some(name) = req.name {
            active_model.name = Set(name);
        }
        if let Some(report_type) = req.report_type {
            // 缺陷 1.2：变更报表类型时同步更新权限码
            let new_perm = report_type_permission(&report_type).map(|s| s.to_string());
            active_model.report_type = Set(report_type);
            active_model.required_permission = Set(new_perm);
        }
        if let Some(columns) = req.columns {
            active_model.columns = Set(columns);
        }
        if let Some(filters) = req.filters {
            active_model.filters = Set(Some(filters));
        }
        if let Some(sort_by) = req.sort_by {
            active_model.sort_by = Set(Some(sort_by));
        }
        if let Some(sort_order) = req.sort_order {
            active_model.sort_order = Set(Some(sort_order));
        }
        if let Some(data_source_sql) = req.data_source_sql {
            active_model.data_source_sql = Set(Some(data_source_sql));
        }
        if let Some(description) = req.description {
            active_model.description = Set(Some(description));
        }
        if let Some(is_public) = req.is_public {
            active_model.is_public = Set(is_public);
        }
        if let Some(status) = req.status {
            active_model.status = Set(status);
        }
        // 缺陷 1.3：刷新策略和缓存 TTL
        if let Some(refresh_strategy) = req.refresh_strategy {
            active_model.refresh_strategy = Set(Some(refresh_strategy));
        }
        if let Some(cache_ttl_seconds) = req.cache_ttl_seconds {
            active_model.cache_ttl_seconds = Set(Some(cache_ttl_seconds));
        }

        // 缺陷 1.1：版本号递增，支持历史回滚
        active_model.version = Set(previous_version + 1);
        active_model.updated_at = Set(Utc::now());

        let updated = active_model.update(&*self.db).await?;

        Ok(updated)
    }

    /// 删除报表模板（软删除）
    pub async fn delete(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        let model = ReportTemplateEntity::find()
            .filter(crate::models::report_template::Column::Id.eq(id))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("报表模板不存在"))?;

        if model.created_by != user_id {
            return Err(AppError::permission_denied("只有创建者可以删除该报表模板"));
        }

        let mut active_model: ActiveModel = model.into();
        active_model.status = Set("INACTIVE".to_string());
        active_model.updated_at = Set(Utc::now());

        active_model.update(&*self.db).await?;

        Ok(())
    }

    /// 查询报表模板列表
    /// 缺陷 1.2 修复：新增 `role_id` 参数，对返回结果按 `required_permission` 过滤。；由于 `required_permission` 存储在 DB 行中，无法在 SQL 层直接拼成 IN/EXISTS 子查询；（权限码需通过 RolePermissionService 解析为 resource_type/action 二元组），；因此采用"先取候选集 → 逐条 check_template_permission 过滤"的策略，；候选集规模受 page_size ≤ 100 限制，性能可接受。
    pub async fn list(
        &self,
        user_id: i32,
        role_id: Option<i32>,
        query: ReportTemplateQuery,
    ) -> Result<(Vec<ReportTemplateModel>, u64), AppError> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

        // v11 批次 149 P2-A：接入 status filter，默认 ACTIVE（软删除语义），管理员可传 INACTIVE 查看已删除模板
        let status_filter = query.status.unwrap_or_else(|| "ACTIVE".to_string());
        let mut select = ReportTemplateEntity::find()
            .filter(crate::models::report_template::Column::Status.eq(status_filter));

        // 只显示公开模板或用户自己创建的模板
        select = select.filter(
            crate::models::report_template::Column::IsPublic
                .eq(true)
                .or(crate::models::report_template::Column::CreatedBy.eq(user_id)),
        );

        if let Some(report_type) = query.report_type {
            select =
                select.filter(crate::models::report_template::Column::ReportType.eq(report_type));
        }

        if let Some(keyword) = query.keyword {
            select = select.filter(
                crate::models::report_template::Column::Name
                    .contains(&keyword)
                    .or(crate::models::report_template::Column::Code.contains(&keyword)),
            );
        }

        // 批次 256 修复：接入 paginate_with_total 统一分页逻辑（内部已处理 saturating_sub(1) 偏移）
        let paginator = select
            .order_by_desc(crate::models::report_template::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;

        // 缺陷 1.2 修复：对每个模板按 required_permission 进行权限过滤
        // 创建者本人直接放行（与 get_by_id 语义一致），其余按角色校验
        let mut filtered = Vec::with_capacity(items.len());
        for item in items {
            if item.created_by == user_id {
                filtered.push(item);
                continue;
            }
            if self
                .check_template_permission(role_id, &item.required_permission)
                .await
                .is_ok()
            {
                filtered.push(item);
            }
        }
        // 过滤后 total 字段无法精确反映过滤后总数（仍按候选集总数返回，
        // 前端分页器按候选集总数展示，单页内条目数可能小于 page_size，业务可接受）
        Ok((filtered, total))
    }

    /// 执行自定义报表
    /// 安全策略：自定义 SQL 报表功能已禁用，统一返回功能禁用错误。
    /// 移除分页参数（方法恒返回错误，参数签名不应欺骗调用方）。
    pub async fn execute_custom_report(
        &self,
        template_id: i32,
        user_id: i32,
        role_id: Option<i32>,
    ) -> Result<(Vec<String>, Vec<Vec<String>>, u64), AppError> {
        let _template = self
            .get_by_id(template_id, user_id, role_id)
            .await?
            .ok_or_else(|| AppError::not_found("报表模板不存在"))?;

        // P0-B 安全修复：彻底关闭"自定义 SQL 报表"执行入口。
        // 任何带 data_source_sql 的模板统一返回功能禁用错误，
        // 避免攻击者通过创建/更新已存在的模板字段来触发 SQL 执行。
        if _template.data_source_sql.is_some() {
            return Err(AppError::permission_denied(
                "出于安全考虑，自定义 SQL 报表功能已禁用，请使用预定义报表模板".to_string(),
            ));
        }

        // 否则使用预定义的报表类型
        Err(AppError::business(
            "自定义报表需要配置数据源SQL".to_string(),
        ))
    }

    /// 缺陷 1.1 修复：列出指定模板的所有历史版本（按版本号倒序）
    pub async fn list_versions(
        &self,
        template_id: i32,
        user_id: i32,
        role_id: Option<i32>,
    ) -> Result<Vec<ReportTemplateVersionModel>, AppError> {
        // 校验模板存在且当前用户可见（复用 get_by_id 的访问控制 + 权限校验）
        let _ = self.get_by_id(template_id, user_id, role_id).await?;

        let items = ReportTemplateVersionEntity::find()
            .filter(crate::models::report_template_version::Column::TemplateId.eq(template_id))
            .order_by_desc(crate::models::report_template_version::Column::Version)
            .all(&*self.db)
            .await?;
        Ok(items)
    }

    /// 缺陷 1.1 修复：回滚到指定历史版本
    /// 实现策略：先将当前模板状态写入版本表（保存为 latest+1 的快照），；再用历史版本字段覆盖当前模板，version 设为 max(existing) + 1。；这样保证回滚操作本身可被再次回滚。
    pub async fn rollback_version(
        &self,
        template_id: i32,
        target_version: i32,
        user_id: i32,
        role_id: Option<i32>,
    ) -> Result<ReportTemplateModel, AppError> {
        // 校验模板存在 + 当前用户对模板有访问权限
        let current = self
            .get_by_id(template_id, user_id, role_id)
            .await?
            .ok_or_else(|| AppError::not_found("报表模板不存在"))?;

        // 仅创建者可回滚（与 update / delete 一致）
        if current.created_by != user_id {
            return Err(AppError::permission_denied("只有创建者可以回滚该报表模板"));
        }

        // 找到目标历史版本
        let target = ReportTemplateVersionEntity::find()
            .filter(crate::models::report_template_version::Column::TemplateId.eq(template_id))
            .filter(crate::models::report_template_version::Column::Version.eq(target_version))
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!(
                    "报表模板 {} 的历史版本 {} 不存在",
                    template_id, target_version
                ))
            })?;

        // 回滚前先保存当前状态作为新快照（保证回滚操作可逆）
        let snapshot = ReportTemplateVersionActiveModel {
            template_id: Set(template_id),
            version: Set(current.version),
            name: Set(current.name.clone()),
            code: Set(current.code.clone()),
            report_type: Set(current.report_type.clone()),
            category: Set(current.category.clone()),
            data_source: Set(current.data_source.clone()),
            columns: Set(current.columns.clone()),
            filters: Set(current.filters.clone()),
            parameters: Set(current.parameters.clone()),
            supported_formats: Set(current.supported_formats.clone()),
            sort_by: Set(current.sort_by.clone()),
            sort_order: Set(current.sort_order.clone()),
            data_source_sql: Set(current.data_source_sql.clone()),
            description: Set(current.description.clone()),
            is_public: Set(current.is_public),
            required_permission: Set(current.required_permission.clone()),
            snapshot_by: Set(user_id),
            snapshot_at: Set(Utc::now()),
            ..Default::default()
        };
        let _ = snapshot.insert(&*self.db).await?;
        tracing::info!(
            template_id = template_id,
            current_version = current.version,
            target_version,
            "报表模板回滚：当前版本已写入快照表，准备覆盖为目标版本字段"
        );

        // 查询当前最大版本号（包括刚写入的快照），新版本号 = max + 1
        let max_version_row = ReportTemplateVersionEntity::find()
            .filter(crate::models::report_template_version::Column::TemplateId.eq(template_id))
            .order_by_desc(crate::models::report_template_version::Column::Version)
            .one(&*self.db)
            .await?;
        let new_version = max_version_row
            .as_ref()
            .map(|v| v.version.max(current.version) + 1)
            .unwrap_or(current.version + 1);

        // 用历史版本字段覆盖当前模板
        let mut active_model: ActiveModel = current.into();
        active_model.name = Set(target.name.clone());
        active_model.code = Set(target.code.clone());
        active_model.report_type = Set(target.report_type.clone());
        active_model.category = Set(target.category.clone());
        active_model.data_source = Set(target.data_source.clone());
        active_model.columns = Set(target.columns.clone());
        active_model.filters = Set(target.filters.clone());
        active_model.parameters = Set(target.parameters.clone());
        active_model.supported_formats = Set(target.supported_formats.clone());
        active_model.sort_by = Set(target.sort_by.clone());
        active_model.sort_order = Set(target.sort_order.clone());
        active_model.data_source_sql = Set(target.data_source_sql.clone());
        active_model.description = Set(target.description.clone());
        active_model.is_public = Set(target.is_public);
        active_model.required_permission = Set(target.required_permission.clone());
        active_model.version = Set(new_version);
        active_model.updated_at = Set(Utc::now());

        let rolled_back = active_model.update(&*self.db).await?;
        tracing::info!(
            template_id = template_id,
            target_version,
            new_version,
            "报表模板回滚成功：新版本号 = {}",
            new_version
        );
        Ok(rolled_back)
    }

    /// 缺陷 1.2 修复：解析 `required_permission` 字符串（如 "report:sales:view"）为 (resource_type, action)
    /// 约定：3 段格式 `report:sales:view` → resource_type="report-sales", action="view"；2 段格式 `report:view` → resource_type="report", action="view"；其他格式返回 None，跳过权限校验（向后兼容）
    fn parse_required_permission(perm: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = perm.split(':').collect();
        match parts.as_slice() {
            [domain, sub, action] => Some((format!("{}-{}", domain, sub), (*action).to_string())),
            [resource, action] => Some(((*resource).to_string(), (*action).to_string())),
            _ => None,
        }
    }

    /// 缺陷 1.2 修复：根据模板的 `required_permission` 字段校验用户权限
    /// 若 `required_permission` 为 None → 跳过校验（向后兼容）；若用户为 admin 角色 → 直接放行（RolePermissionService::check_permission 内部已处理）；否则调用 RolePermissionService::check_permission 校验
    pub async fn check_template_permission(
        &self,
        role_id: Option<i32>,
        required_permission: &Option<String>,
    ) -> Result<(), AppError> {
        let Some(perm) = required_permission.as_ref() else {
            return Ok(());
        };
        let Some(role_id) = role_id else {
            // 无角色 ID 视为匿名用户，若有权限要求则拒绝
            return Err(AppError::permission_denied(format!(
                "访问该报表需要权限：{}（当前用户无角色）",
                perm
            )));
        };
        let Some((resource_type, action)) = Self::parse_required_permission(perm) else {
            tracing::warn!(
                required_permission = %perm,
                "无法解析报表模板的 required_permission，跳过权限校验"
            );
            return Ok(());
        };
        let svc = RolePermissionService::new(self.db.clone());
        let allowed = svc
            .check_permission(role_id, &resource_type, &action, None)
            .await?;
        if !allowed {
            return Err(AppError::permission_denied(format!(
                "无权访问该报表，需要权限：{}",
                perm
            )));
        }
        Ok(())
    }
}
