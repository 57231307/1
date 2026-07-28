//! 角色权限矩阵与互斥规则子模块（init_service_ops/permission）
//!
//! 从原 `init_service.rs` 迁移 2 个方法：
//! - create_default_role_permissions：为全部角色创建 role_permission 权限矩阵（覆盖 60+ 资源 × 11 操作码）
//! - create_default_role_conflicts：初始化默认角色互斥规则（SoD 职责分离）

use crate::models::{role, role_conflict, role_permission};
use crate::services::init_service::{InitError, InitService};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, Set};
use tracing::warn;

// ===== 类型别名（避免 clippy `type_complexity` 警告：嵌套引用切片类型过深）=====

/// 资源-操作对（如 ("users", "read")）
type PermPair = (&'static str, &'static str);

/// 角色权限定义组：角色代码 + 该角色的资源操作列表
type RoleResourceGroup = (&'static str, &'static [PermPair]);

/// 单个域的角色权限定义切片（多个角色权限组）
type RoleResourceSlice = &'static [RoleResourceGroup];

/// 全部域的角色权限定义分组列表
type RoleResourceGroups = Vec<RoleResourceSlice>;

impl InitService {
    /// 创建全部角色的 role_permission 权限矩阵（V15 P0-S03/S04/S20，覆盖 60+ 资源 × 11 操作码）。
    pub(crate) async fn create_default_role_permissions(&self) -> Result<(), InitError> {
        if Self::role_permissions_already_exist(self.db.as_ref()).await {
            return Ok(());
        }
        let now = chrono::Utc::now();
        let mut perms: Vec<role_permission::ActiveModel> = Vec::new();
        for definitions in Self::all_role_permission_definition_groups() {
            Self::extend_perms_from_definitions(self.db.as_ref(), definitions, now, &mut perms)
                .await?;
        }
        Self::batch_insert_permissions(self.db.as_ref(), perms).await;
        Ok(())
    }

    /// 检查 role_permission 表是否已有记录（幂等，查询错误时视为 0 条继续执行）。
    async fn role_permissions_already_exist(db: &DatabaseConnection) -> bool {
        let count = role_permission::Entity::find().count(db).await.unwrap_or(0);
        count > 0
    }

    /// 汇总全部域的角色权限定义分组（管理/高管/销售/采购/库存/生产/质量/财务/CRM物流HR/其他）。
    fn all_role_permission_definition_groups() -> RoleResourceGroups {
        vec![
            Self::management_role_resources(),
            Self::executive_role_resources(),
            Self::sales_role_resources(),
            Self::purchase_role_resources(),
            Self::inventory_role_resources(),
            Self::production_core_role_resources(),
            Self::production_extended_role_resources(),
            Self::quality_role_resources(),
            Self::finance_role_resources(),
            Self::crm_logistics_hr_role_resources(),
            Self::misc_role_resources(),
        ]
    }

    /// 遍历角色权限定义分组，查询角色 ID 并扩展权限 ActiveModel 列表。
    async fn extend_perms_from_definitions(
        db: &DatabaseConnection,
        definitions: &[(&str, &[(&str, &str)])],
        now: chrono::DateTime<chrono::Utc>,
        perms: &mut Vec<role_permission::ActiveModel>,
    ) -> Result<(), InitError> {
        for (role_code, resources) in definitions {
            let role_model = role::Entity::find()
                .filter(role::Column::Code.eq(*role_code))
                .one(db)
                .await
                .map_err(|e| {
                    InitError::DatabaseError(format!("查询 {} 角色失败: {}", role_code, e))
                })?;
            if let Some(r) = role_model {
                perms.extend(Self::make_permission_models(r.id, resources, now));
            }
        }
        Ok(())
    }

    /// 为指定角色按 resource×action 列表生成权限 ActiveModel（全部 allowed=true）。
    fn make_permission_models(
        role_id: i32,
        resources: &[(&str, &str)],
        now: chrono::DateTime<chrono::Utc>,
    ) -> Vec<role_permission::ActiveModel> {
        resources
            .iter()
            .map(|(resource, action)| role_permission::ActiveModel {
                id: Default::default(),
                role_id: Set(role_id),
                resource_type: Set(resource.to_string()),
                resource_id: Set(None),
                action: Set(action.to_string()),
                allowed: Set(true),
                created_at: Set(now),
                updated_at: Set(now),
            })
            .collect()
    }

    /// 批量插入角色权限记录（空列表跳过，失败仅 warn 不阻断初始化）。
    async fn batch_insert_permissions(
        db: &DatabaseConnection,
        perms: Vec<role_permission::ActiveModel>,
    ) {
        if perms.is_empty() {
            return;
        }
        if let Err(e) = role_permission::Entity::insert_many(perms).exec(db).await {
            warn!("批量创建角色权限失败: {}, 可能部分已存在", e);
        }
    }

    // ===== 角色权限数据定义（按域分组，每组 ≤50 行）=====

    /// 管理域角色权限定义（manager 部门经理跨域读取+本域全部操作 / operator 全业务域只读）。
    fn management_role_resources() -> RoleResourceSlice {
        &[
            // manager：部门经理，跨业务域读取 + 本域全部操作
            (
                "manager",
                &[
                    ("users", "read"),
                    ("roles", "read"),
                    ("departments", "read"),
                    ("products", "*"),
                    ("orders", "*"),
                    ("customers", "*"),
                    ("suppliers", "*"),
                    ("inventory", "read"),
                    ("stock-alerts", "read"),
                    ("purchase-orders", "read"),
                    ("production-orders", "read"),
                    ("dye-batches", "read"),
                    ("quality-inspections", "read"),
                    ("vouchers", "read"),
                    ("ar", "read"),
                    ("ap", "read"),
                    ("gl", "read"),
                    ("reports", "read"),
                    ("dashboard", "read"),
                ],
            ),
            // operator：操作员，全业务域只读
            (
                "operator",
                &[
                    ("products", "read"),
                    ("orders", "read"),
                    ("customers", "read"),
                    ("suppliers", "read"),
                    ("inventory", "read"),
                    ("purchase-orders", "read"),
                    ("production-orders", "read"),
                    ("dye-batches", "read"),
                    ("quality-inspections", "read"),
                    ("reports", "read"),
                ],
            ),
        ]
    }

    /// 高管域角色权限定义（gm 总经理 / deputy_gm 副总经理，全资源 read + AI 域只读）。
    fn executive_role_resources() -> RoleResourceSlice {
        // V15 P0-S26：AI 域只读权限（管理层可查看所有 AI 分析结果）
        &[
            (
                "gm",
                &[
                    ("users", "read"),
                    ("roles", "read"),
                    ("departments", "read"),
                    ("products", "read"),
                    ("orders", "read"),
                    ("customers", "read"),
                    ("suppliers", "read"),
                    ("inventory", "read"),
                    ("purchase-orders", "read"),
                    ("production-orders", "read"),
                    ("dye-batches", "read"),
                    ("dye-recipes", "read"),
                    ("quality-inspections", "read"),
                    ("fabric-inspections", "read"),
                    ("vouchers", "read"),
                    ("ar", "read"),
                    ("ap", "read"),
                    ("gl", "read"),
                    ("budgets", "read"),
                    ("cost-collections", "read"),
                    ("crm-leads", "read"),
                    ("crm-opportunities", "read"),
                    ("logistics", "read"),
                    ("employees", "read"),
                    ("wages", "read"),
                    ("reports", "read"),
                    ("bi-analysis", "read"),
                    ("dashboard", "read"),
                    ("ai-forecast", "read"),
                    ("ai-inventory-opt", "read"),
                    ("ai-anomaly", "read"),
                    ("ai-recommendation", "read"),
                    ("ai-recipe-opt", "read"),
                    ("ai-quality-pred", "read"),
                    ("ai-process-opt", "read"),
                    ("ai-summary", "read"),
                ],
            ),
            (
                "deputy_gm",
                &[
                    ("users", "read"),
                    ("roles", "read"),
                    ("departments", "read"),
                    ("products", "read"),
                    ("orders", "read"),
                    ("customers", "read"),
                    ("suppliers", "read"),
                    ("inventory", "read"),
                    ("purchase-orders", "read"),
                    ("production-orders", "read"),
                    ("dye-batches", "read"),
                    ("dye-recipes", "read"),
                    ("quality-inspections", "read"),
                    ("fabric-inspections", "read"),
                    ("vouchers", "read"),
                    ("ar", "read"),
                    ("ap", "read"),
                    ("gl", "read"),
                    ("budgets", "read"),
                    ("cost-collections", "read"),
                    ("crm-leads", "read"),
                    ("crm-opportunities", "read"),
                    ("logistics", "read"),
                    ("employees", "read"),
                    ("wages", "read"),
                    ("reports", "read"),
                    ("bi-analysis", "read"),
                    ("dashboard", "read"),
                    ("ai-forecast", "read"),
                    ("ai-inventory-opt", "read"),
                    ("ai-anomaly", "read"),
                    ("ai-recommendation", "read"),
                    ("ai-recipe-opt", "read"),
                    ("ai-quality-pred", "read"),
                    ("ai-process-opt", "read"),
                    ("ai-summary", "read"),
                ],
            ),
        ]
    }

    /// 销售域角色权限定义（V15 P1-14.3-D：sales_manager 仅审批不创建 / sales_rep 仅创建不审批）。
    fn sales_role_resources() -> RoleResourceSlice {
        &[
            // V15 P1-14.3-D：sales_manager 拆分 create 与 approve（SoD 职责分离）
            // 原为 ("orders", "*") 含 create+approve，现改为显式列出审批操作，移除 create
            (
                "sales_manager",
                &[
                    ("orders", "read"),
                    ("orders", "update"),
                    ("orders", "delete"),
                    ("orders", "approve"),
                    ("orders", "reject"),
                    ("fabric-orders", "read"),
                    ("fabric-orders", "approve"),
                    ("sales-contracts", "read"),
                    ("sales-contracts", "approve"),
                    ("sales-prices", "read"),
                    ("sales-prices", "approve"),
                    ("sales-returns", "read"),
                    ("sales-returns", "approve"),
                    ("sales-returns", "reject"),
                    ("quotations", "read"),
                    ("quotations", "approve"),
                    ("quotations", "reject"),
                    ("custom-orders", "*"),
                    ("color-cards", "*"),
                    ("color-prices", "*"),
                    ("customers", "*"),
                    ("customer-credits", "read"),
                    ("ar", "read"),
                    ("reports", "read"),
                    ("sales-analysis", "read"),
                ],
            ),
            (
                "sales_rep",
                &[
                    ("orders", "read"),
                    ("orders", "create"),
                    ("orders", "update"),
                    ("fabric-orders", "read"),
                    ("fabric-orders", "create"),
                    ("quotations", "read"),
                    ("quotations", "create"),
                    ("customers", "read"),
                    ("customers", "create"),
                    ("sales-returns", "read"),
                    ("sales-returns", "create"),
                    ("color-cards", "read"),
                    ("sales-prices", "read"),
                ],
            ),
        ]
    }

    /// 采购域角色权限定义（V15 P1-14.3-D：purchase_manager 仅审批不创建 / purchase_clerk 仅创建不审批）。
    fn purchase_role_resources() -> RoleResourceSlice {
        &[
            // V15 P1-14.3-D：purchase_manager 拆分 create 与 approve（SoD 职责分离）
            // 原为 ("purchase-orders", "*") 含 create+approve，现改为显式列出审批操作，移除 create
            (
                "purchase_manager",
                &[
                    ("purchase-orders", "read"),
                    ("purchase-orders", "update"),
                    ("purchase-orders", "delete"),
                    ("purchase-orders", "approve"),
                    ("purchase-orders", "reject"),
                    ("purchase-receipts", "read"),
                    ("purchase-receipts", "approve"),
                    ("purchase-returns", "read"),
                    ("purchase-returns", "approve"),
                    ("purchase-returns", "reject"),
                    ("purchase-contracts", "read"),
                    ("purchase-contracts", "approve"),
                    ("purchase-prices", "read"),
                    ("purchase-prices", "approve"),
                    ("suppliers", "*"),
                    ("supplier-evaluations", "*"),
                    ("ap", "read"),
                    ("inventory", "read"),
                    ("reports", "read"),
                ],
            ),
            (
                "purchase_clerk",
                &[
                    ("purchase-orders", "read"),
                    ("purchase-orders", "create"),
                    ("purchase-orders", "update"),
                    ("purchase-receipts", "read"),
                    ("purchase-receipts", "create"),
                    ("suppliers", "read"),
                    ("inventory", "read"),
                    ("purchase-prices", "read"),
                ],
            ),
            (
                "sourcing_specialist",
                &[
                    ("suppliers", "read"),
                    ("suppliers", "create"),
                    ("suppliers", "update"),
                    ("supplier-evaluations", "read"),
                    ("supplier-evaluations", "create"),
                    ("purchase-prices", "*"),
                    ("purchase-contracts", "read"),
                ],
            ),
        ]
    }

    /// 库存仓储域角色权限定义（inventory_manager 库存经理 / warehouse_keeper 仓管员）。
    fn inventory_role_resources() -> RoleResourceSlice {
        &[
            (
                "inventory_manager",
                &[
                    ("inventory", "*"),
                    ("stock", "*"),
                    ("piece-split", "*"),
                    ("transfers", "*"),
                    ("adjustments", "*"),
                    ("reservations", "*"),
                    ("counts", "*"),
                    ("batches", "*"),
                    ("stock-alerts", "*"),
                    ("products", "read"),
                    ("warehouses", "read"),
                    ("reports", "read"),
                ],
            ),
            (
                "warehouse_keeper",
                &[
                    ("inventory", "read"),
                    ("inventory", "create"),
                    ("inventory", "update"),
                    ("stock", "read"),
                    ("stock", "create"),
                    ("stock", "update"),
                    ("transfers", "read"),
                    ("transfers", "create"),
                    ("counts", "read"),
                    ("counts", "create"),
                    ("products", "read"),
                    ("warehouses", "read"),
                ],
            ),
        ]
    }

    /// 生产核心域角色权限定义（production_manager / dyeing_master / finishing_master / lab_technician）。
    fn production_core_role_resources() -> RoleResourceSlice {
        &[
            (
                "production_manager",
                &[
                    ("production-orders", "*"),
                    ("dye-batches", "*"),
                    ("dye-recipes", "*"),
                    ("dye-batch-lifecycle-logs", "*"),
                    ("dye-batch-state-rules", "read"),
                    ("dye-batch-reworks", "*"),
                    ("dye-batch-operations", "*"),
                    ("greige-fabrics", "read"),
                    ("lab-dip", "read"),
                    ("production-recipes", "*"),
                    ("process-routes", "*"),
                    ("flow-cards", "*"),
                    ("outsourcing-orders", "*"),
                    ("outsourcing-receipts", "*"),
                    ("business-modes", "read"),
                    ("mrp", "*"),
                    ("capacity", "*"),
                    ("scheduling", "*"),
                    ("material-shortage", "read"),
                ],
            ),
            (
                "dyeing_master",
                &[
                    ("dye-batches", "*"),
                    ("dye-batch-lifecycle-logs", "read"),
                    ("dye-batch-operations", "*"),
                    ("dye-batch-reworks", "*"),
                    ("dye-recipes", "read"),
                    ("dye-recipes", "create"),
                    ("production-recipes", "read"),
                    ("chemicals", "read"),
                    ("chemical-lots", "read"),
                    ("flow-cards", "read"),
                    ("flow-cards", "update"),
                ],
            ),
            (
                "finishing_master",
                &[
                    ("production-orders", "read"),
                    ("production-orders", "create"),
                    ("production-orders", "update"),
                    ("dye-batches", "read"),
                    ("dye-batch-operations", "create"),
                    ("flow-cards", "read"),
                    ("flow-cards", "update"),
                    ("fabric-inspections", "read"),
                ],
            ),
            (
                "lab_technician",
                &[
                    ("dye-recipes", "*"),
                    ("lab-dip", "*"),
                    ("dye-batches", "read"),
                    ("color-cards", "*"),
                    ("color-prices", "read"),
                    ("chemicals", "read"),
                    ("chemical-lots", "read"),
                ],
            ),
        ]
    }

    /// 生产扩展域角色权限定义（dye_recipe_master / greige_manager / chemical_manager / maintenance_supervisor）。
    fn production_extended_role_resources() -> RoleResourceSlice {
        // V15 P0-S18 新增：染色配方主管，含审批权限
        &[
            (
                "dye_recipe_master",
                &[
                    ("dye-recipes", "*"),
                    ("dye-recipes", "approve"),
                    ("dye-recipes", "audit"),
                    ("lab-dip", "*"),
                    ("dye-batches", "read"),
                    ("dye-batches", "update"),
                    ("production-recipes", "*"),
                    ("color-cards", "*"),
                    ("color-prices", "*"),
                    ("chemicals", "read"),
                    ("chemical-lots", "read"),
                    ("reports", "read"),
                ],
            ),
            (
                "greige_manager",
                &[
                    ("greige-fabrics", "*"),
                    ("outsourcing-orders", "*"),
                    ("outsourcing-receipts", "*"),
                    ("outsourcing-vouchers", "read"),
                    ("inventory", "read"),
                    ("stock", "read"),
                    ("suppliers", "read"),
                    ("purchase-orders", "read"),
                ],
            ),
            (
                "chemical_manager",
                &[
                    ("chemicals", "*"),
                    ("chemical-categories", "*"),
                    ("chemical-lots", "*"),
                    ("chemical-requisitions", "*"),
                    ("purchase-orders", "read"),
                    ("inventory", "read"),
                    ("stock", "read"),
                ],
            ),
            (
                "maintenance_supervisor",
                &[
                    ("equipment", "*"),
                    ("maintenance-records", "*"),
                    ("production-orders", "read"),
                    ("energy-meters", "read"),
                    ("energy-consumptions", "read"),
                ],
            ),
        ]
    }

    /// 质量域角色权限定义（qc_manager 质量经理 / quality_inspector 质检员 / fabric_inspector 面料检验员）。
    fn quality_role_resources() -> RoleResourceSlice {
        &[
            (
                "qc_manager",
                &[
                    ("quality-inspections", "*"),
                    ("quality-issues", "*"),
                    ("quality-standards", "*"),
                    ("fabric-inspections", "*"),
                    ("fabric-defects", "*"),
                    ("dye-batches", "read"),
                    ("production-orders", "read"),
                    ("reports", "read"),
                    ("business-trace", "read"),
                ],
            ),
            (
                "quality_inspector",
                &[
                    ("quality-inspections", "*"),
                    ("quality-issues", "read"),
                    ("quality-issues", "create"),
                    ("dye-batches", "read"),
                    ("fabric-inspections", "read"),
                    ("fabric-inspections", "create"),
                ],
            ),
            (
                "fabric_inspector",
                &[
                    ("fabric-inspections", "*"),
                    ("fabric-defects", "*"),
                    ("dye-batches", "read"),
                    ("products", "read"),
                    ("quality-standards", "read"),
                ],
            ),
        ]
    }

    /// 财务域角色权限定义（finance_manager / accountant / cashier / cost_accountant）。
    fn finance_role_resources() -> RoleResourceSlice {
        &[
            (
                "finance_manager",
                &[
                    ("vouchers", "*"),
                    ("subjects", "*"),
                    ("fixed-assets", "*"),
                    ("budgets", "*"),
                    ("cost-collections", "*"),
                    ("ar", "*"),
                    ("ap", "*"),
                    ("gl", "*"),
                    ("financial-analysis", "*"),
                    ("fund-management", "*"),
                    ("fund-transfers", "*"),
                    ("currencies", "*"),
                    ("exchange-rates", "*"),
                    ("ar-reconciliations", "*"),
                    ("accounting-periods", "*"),
                    ("wages", "read"),
                    ("reports", "read"),
                ],
            ),
            (
                "accountant",
                &[
                    ("vouchers", "*"),
                    ("subjects", "read"),
                    ("gl", "read"),
                    ("gl", "create"),
                    ("gl", "update"),
                    ("ar", "read"),
                    ("ar", "create"),
                    ("ap", "read"),
                    ("ap", "create"),
                    ("fixed-assets", "read"),
                    ("fixed-assets", "create"),
                    ("currencies", "read"),
                    ("exchange-rates", "read"),
                    ("ar-reconciliations", "read"),
                    ("ar-reconciliations", "create"),
                ],
            ),
            (
                "cashier",
                &[
                    ("fund-transfers", "read"),
                    ("fund-transfers", "create"),
                    ("fund-transfers", "update"),
                    ("fund-management", "read"),
                    ("fund-management", "create"),
                    ("ar", "read"),
                    ("ap", "read"),
                    ("vouchers", "read"),
                    ("vouchers", "create"),
                ],
            ),
            (
                "cost_accountant",
                &[
                    ("cost-collections", "*"),
                    ("vouchers", "read"),
                    ("vouchers", "create"),
                    ("dye-batches", "read"),
                    ("production-orders", "read"),
                    ("outsourcing-orders", "read"),
                    ("outsourcing-receipts", "read"),
                    ("wages", "read"),
                    ("energy-allocations", "read"),
                    ("gl", "read"),
                ],
            ),
        ]
    }

    /// CRM/物流/人力资源域角色权限定义（crm_manager / crm_rep / logistics_coordinator / customs_specialist / hr_manager / hr_specialist）。
    fn crm_logistics_hr_role_resources() -> RoleResourceSlice {
        &[
            (
                "crm_manager",
                &[
                    ("crm-leads", "*"),
                    ("crm-opportunities", "*"),
                    ("crm-customers", "*"),
                    ("customers", "*"),
                    ("customer-credits", "*"),
                    ("five-dimension", "*"),
                    ("sales-analysis", "*"),
                    ("reports", "read"),
                ],
            ),
            (
                "crm_rep",
                &[
                    ("crm-leads", "read"),
                    ("crm-leads", "create"),
                    ("crm-leads", "update"),
                    ("crm-opportunities", "read"),
                    ("crm-opportunities", "create"),
                    ("crm-opportunities", "update"),
                    ("crm-customers", "read"),
                    ("crm-customers", "create"),
                    ("customers", "read"),
                    ("customers", "create"),
                ],
            ),
            (
                "logistics_coordinator",
                &[
                    ("logistics", "*"),
                    ("ship-orders", "*"),
                    ("orders", "read"),
                    ("inventory", "read"),
                    ("incoterms", "read"),
                    ("reports", "read"),
                ],
            ),
            (
                "customs_specialist",
                &[
                    ("logistics", "read"),
                    ("logistics", "create"),
                    ("logistics", "update"),
                    ("incoterms", "read"),
                    ("incoterms", "create"),
                    ("orders", "read"),
                    ("ship-orders", "read"),
                ],
            ),
            (
                "hr_manager",
                &[
                    ("employees", "*"),
                    ("departments", "*"),
                    ("wages", "*"),
                    ("wage-rates", "*"),
                    ("wage-records", "*"),
                    ("users", "read"),
                    ("roles", "read"),
                    ("reports", "read"),
                ],
            ),
            (
                "hr_specialist",
                &[
                    ("employees", "read"),
                    ("employees", "create"),
                    ("employees", "update"),
                    ("departments", "read"),
                    ("wages", "read"),
                    ("wages", "create"),
                    ("wage-records", "read"),
                    ("wage-records", "create"),
                    ("users", "read"),
                ],
            ),
        ]
    }

    /// 其他域角色权限定义（safety_officer 安全员 / system_admin 系统管理员 / data_analyst 数据分析师 / admin_assistant 行政助理）。
    fn misc_role_resources() -> RoleResourceSlice {
        &[
            (
                "safety_officer",
                &[
                    ("safety-records", "*"),
                    ("environmental-records", "*"),
                    ("equipment", "read"),
                    ("maintenance-records", "read"),
                    ("chemicals", "read"),
                    ("chemical-lots", "read"),
                    ("reports", "read"),
                    ("audit-logs", "read"),
                ],
            ),
            (
                "system_admin",
                &[
                    ("users", "*"),
                    ("roles", "*"),
                    ("departments", "*"),
                    ("permissions", "*"),
                    ("field-permissions", "*"),
                    ("system-config", "*"),
                    ("audit-logs", "*"),
                    ("slow-queries", "*"),
                    ("print-templates", "*"),
                    ("data-import", "*"),
                    ("permissions-audit", "*"),
                    ("business-trace", "read"),
                ],
            ),
            (
                "data_analyst",
                &[
                    ("reports", "*"),
                    ("bi-analysis", "*"),
                    ("dashboard", "*"),
                    ("sales-analysis", "*"),
                    ("five-dimension", "read"),
                    ("orders", "read"),
                    ("customers", "read"),
                    ("inventory", "read"),
                    ("vouchers", "read"),
                    ("ar", "read"),
                    ("ap", "read"),
                ],
            ),
            (
                "admin_assistant",
                &[
                    ("users", "read"),
                    ("users", "create"),
                    ("users", "update"),
                    ("departments", "read"),
                    ("oa-announcements", "*"),
                    ("notifications", "read"),
                    ("notifications", "create"),
                ],
            ),
        ]
    }

    /// 初始化默认角色互斥规则（SoD 职责分离，幂等）
    pub(crate) async fn create_default_role_conflicts(&self) -> Result<(), InitError> {
        // 幂等检查：表已有记录则跳过
        let existing_count = role_conflict::Entity::find()
            .count(self.db.as_ref())
            .await
            .map_err(|e| InitError::DatabaseError(format!("查询 role_conflicts 失败: {}", e)))?;
        if existing_count > 0 {
            return Ok(());
        }

        // 默认互斥角色对（role_a_code < role_b_code 字典序约定）
        // 来源：经典 SoD 职责分离 + 面料行业业务场景
        let conflicts: &[(&str, &str, &str, &str)] = &[
            // (role_a_code, role_b_code, conflict_type, description)
            (
                "accounting_clerk",
                "financial_manager",
                "sod",
                "制单与审核分离：会计员不可兼任财务经理",
            ),
            (
                "purchase_clerk",
                "purchase_manager",
                "sod",
                "采购制单与采购审批分离",
            ),
            (
                "purchase_manager",
                "finance_manager",
                "sod",
                "采购审批与付款审批分离",
            ),
            (
                "sales_clerk",
                "sales_manager",
                "sod",
                "销售制单与销售审批分离",
            ),
            (
                "production_worker",
                "quality_inspector",
                "sod",
                "生产执行与质量检验分离",
            ),
            (
                "production_manager",
                "quality_manager",
                "sod",
                "生产管理与质量管理分离",
            ),
            (
                "warehouse_keeper",
                "purchase_clerk",
                "sod",
                "入库与采购分离，防止自采自收",
            ),
            (
                "warehouse_keeper",
                "sales_clerk",
                "sod",
                "出库与销售分离，防止自销自发",
            ),
            (
                "admin",
                "quality_inspector",
                "sod",
                "管理员与质检员互斥（管理员不应直接执行质检）",
            ),
        ];

        let now = chrono::Utc::now();
        let models: Vec<role_conflict::ActiveModel> = conflicts
            .iter()
            .map(|(a, b, ctype, desc)| role_conflict::ActiveModel {
                id: Default::default(),
                role_a_code: Set(a.to_string()),
                role_b_code: Set(b.to_string()),
                conflict_type: Set(ctype.to_string()),
                description: Set(Some(desc.to_string())),
                created_at: Set(now),
                updated_at: Set(now),
            })
            .collect();

        if let Err(e) = role_conflict::Entity::insert_many(models)
            .exec(self.db.as_ref())
            .await
        {
            warn!("批量创建角色互斥规则失败: {}, 可能部分已存在", e);
        }

        Ok(())
    }
}
