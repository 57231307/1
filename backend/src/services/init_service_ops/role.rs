//! 默认角色创建子模块（init_service_ops/role）
//!
//! 从原 `init_service.rs` 迁移 10 个角色创建方法：
//! - create_default_roles / find_existing_admin_role / create_admin_role / batch_insert_roles
//! - build_manager_role / build_operator_role / build_business_roles
//! - build_front_business_roles / build_back_business_roles / build_role_active_model

use crate::models::role;
use crate::services::init_service::{InitError, InitService};
use crate::utils::admin_checker::ADMIN_ROLE_CODE;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tracing::warn;

impl InitService {
    pub(crate) async fn create_default_roles(&self) -> Result<role::Model, InitError> {
        // 批次 24 v6 P0-1 修复：使用 ADMIN_ROLE_CODE 常量替代硬编码 "admin"，
        // 与 admin_checker.rs 保持单一真相源，避免角色编码变更时多处不同步。
        if let Some(admin_role) = self.find_existing_admin_role().await? {
            return Ok(admin_role);
        }

        // 如果不存在，则创建角色
        let admin_role = self.create_admin_role().await?;

        // 创建其他角色
        let mut all_new_roles = vec![Self::build_manager_role(), Self::build_operator_role()];
        all_new_roles.extend(Self::build_business_roles());
        self.batch_insert_roles(all_new_roles).await;

        Ok(admin_role)
    }

    async fn find_existing_admin_role(&self) -> Result<Option<role::Model>, InitError> {
        role::Entity::find()
            .filter(role::Column::Code.eq(ADMIN_ROLE_CODE))
            .one(self.db.as_ref())
            .await
            .map_err(|e| {
                let err_msg = format!("{}", e);
                if err_msg.contains("does not exist") || err_msg.contains("relation") {
                    InitError::DatabaseError("数据库表不存在，需要先初始化数据库".to_string())
                } else {
                    InitError::DatabaseError(format!("查询角色失败: {}", e))
                }
            })
    }

    async fn create_admin_role(&self) -> Result<role::Model, InitError> {
        let admin_role = role::ActiveModel {
            id: Default::default(),
            name: Set("管理员".to_string()),
            code: Set(ADMIN_ROLE_CODE.to_string()),
            description: Set(Some("系统管理员".to_string())),
            permissions: Set(Some("[\"*\"]".to_string())),
            is_system: Set(true),
            // V15 P0-S01：admin 角色数据范围为 all（全部数据）
            data_scope: Set("all".to_string()),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        };
        admin_role
            .insert(self.db.as_ref())
            .await
            .map_err(|e| InitError::DatabaseError(format!("创建管理员角色失败: {}", e)))
    }

    async fn batch_insert_roles(&self, roles: Vec<role::ActiveModel>) {
        if let Err(e) = role::Entity::insert_many(roles)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(role::Column::Code)
                    .do_nothing()
                    .to_owned(),
            )
            .exec(self.db.as_ref())
            .await
        {
            warn!("批量创建角色失败: {}, 可能部分已存在", e);
        }
    }

    fn build_manager_role() -> role::ActiveModel {
        role::ActiveModel {
            id: Default::default(),
            name: Set("部门经理".to_string()),
            code: Set("manager".to_string()),
            description: Set(Some("部门经理".to_string())),
            permissions: Set(Some(
                "[\"user:view\", \"product:*\", \"inventory:*\", \"sales:*\"]".to_string(),
            )),
            is_system: Set(true),
            // V15 P0-S01：manager 角色数据范围为 dept（本部门数据）
            data_scope: Set("dept".to_string()),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        }
    }

    fn build_operator_role() -> role::ActiveModel {
        role::ActiveModel {
            id: Default::default(),
            name: Set("操作员".to_string()),
            code: Set("operator".to_string()),
            description: Set(Some("操作员".to_string())),
            permissions: Set(Some(
                "[\"product:view\", \"inventory:view\", \"sales:view\"]".to_string(),
            )),
            is_system: Set(true),
            // V15 P0-S01：operator 角色数据范围为 self（仅本人数据）
            data_scope: Set("self".to_string()),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        }
    }

    fn build_business_roles() -> Vec<role::ActiveModel> {
        // V15 P0-S04：补齐业务角色（仅创建角色记录，权限在 create_default_role_permissions 中配置）
        // 覆盖面料行业 ERP 全业务场景：管理/销售/采购/库存/生产/质量/财务/CRM/物流/人力/安全/IT
        // V15 P0-S01：每个角色配置 data_scope（all/dept/self）
        // 元组第四项为 data_scope：all（管理层）/ dept（经理）/ self（执行角色）
        let now = chrono::Utc::now();
        let mut roles = Self::build_front_business_roles(now);
        roles.extend(Self::build_back_business_roles(now));
        roles
    }

    fn build_front_business_roles(now: chrono::DateTime<chrono::Utc>) -> Vec<role::ActiveModel> {
        let tuples: &[(&str, &str, &str, &str)] = &[
            // 管理层 → all
            ("总经理", "gm", "全公司最高管理权限", "all"),
            ("副总经理", "deputy_gm", "分管副总管理权限", "all"),
            // 销售域
            ("销售经理", "sales_manager", "销售业务管理与审批", "dept"),
            ("销售代表", "sales_rep", "销售订单录入与客户跟进", "self"),
            // 采购域
            ("采购经理", "purchase_manager", "采购业务管理与审批", "dept"),
            ("采购员", "purchase_clerk", "采购订单录入与供应商对接", "self"),
            ("寻源专员", "sourcing_specialist", "供应商开发与寻源询价", "self"),
            // 库存仓储域
            ("库存经理", "inventory_manager", "库存管理与调拨审批", "dept"),
            ("仓库管理员", "warehouse_keeper", "仓库收发货与库存操作", "self"),
            // 生产域（面料行业深化）
            ("生产经理", "production_manager", "生产计划与排程管理", "dept"),
            ("染色主管", "dyeing_master", "染色生产与配方管理", "self"),
            ("后整理主管", "finishing_master", "定型预缩柔软等后整理工序", "self"),
            ("化验室技术员", "lab_technician", "打样配方开发与颜色管理", "self"),
            ("染色配方主管", "dye_recipe_master", "染色配方管理与审批发布", "self"),
            ("胚布管理员", "greige_manager", "胚布采购库存与委托加工", "self"),
            ("染化料管理员", "chemical_manager", "染料助剂化学品采购存储领用", "self"),
            ("设备维护主管", "maintenance_supervisor", "设备管理与维修计划", "self"),
        ];
        tuples
            .iter()
            .map(|(name, code, desc, scope)| {
                Self::build_role_active_model(name, code, desc, scope, now)
            })
            .collect()
    }

    fn build_back_business_roles(now: chrono::DateTime<chrono::Utc>) -> Vec<role::ActiveModel> {
        let tuples: &[(&str, &str, &str, &str)] = &[
            // 质量域
            ("质量管理经理", "qc_manager", "质量体系管理与8D流程协调", "dept"),
            ("质检员", "quality_inspector", "质量检验与异常处理", "self"),
            ("验布员", "fabric_inspector", "面料检验与十项物理指标检测", "self"),
            // 财务域
            ("财务经理", "finance_manager", "财务管理与凭证审批", "dept"),
            ("会计", "accountant", "会计凭证录入与对账", "self"),
            ("出纳", "cashier", "资金收付与银行对账", "self"),
            ("成本会计", "cost_accountant", "成本核算与成本分析", "self"),
            // CRM 域
            ("CRM经理", "crm_manager", "CRM管理与客户分配", "dept"),
            ("CRM专员", "crm_rep", "线索跟进与商机管理", "self"),
            // 物流域
            ("物流协调员", "logistics_coordinator", "运单管理与物流跟踪", "self"),
            ("报关专员", "customs_specialist", "进出口报关与贸易合规", "self"),
            // 人力资源域
            ("人事经理", "hr_manager", "员工管理与考勤工资审批", "dept"),
            ("人事专员", "hr_specialist", "员工档案与考勤录入", "self"),
            // 安全环保域
            ("安全环保专员", "safety_officer", "安全生产与环保合规", "self"),
            // IT/数据域
            ("系统管理员", "system_admin", "系统配置与用户管理", "all"),
            ("数据分析师", "data_analyst", "BI报表与数据分析", "all"),
            // 行政
            ("行政助理", "admin_assistant", "用户管理与公告发布", "self"),
        ];
        tuples
            .iter()
            .map(|(name, code, desc, scope)| {
                Self::build_role_active_model(name, code, desc, scope, now)
            })
            .collect()
    }

    fn build_role_active_model(
        name: &str,
        code: &str,
        desc: &str,
        scope: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> role::ActiveModel {
        role::ActiveModel {
            id: Default::default(),
            name: Set(name.to_string()),
            code: Set(code.to_string()),
            description: Set(Some(desc.to_string())),
            permissions: Set(None),
            is_system: Set(true),
            // V15 P0-S01：配置数据范围（all/dept/self）
            data_scope: Set(scope.to_string()),
            created_at: Set(now),
            updated_at: Set(now),
        }
    }
}
