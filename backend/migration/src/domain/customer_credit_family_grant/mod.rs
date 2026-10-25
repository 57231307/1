//! 客户信用族（customer-credits）授权面补授（finance_manager 与 accountant）聚合迁移域。
//!
//! 本域只落一条数据迁移台账，登记为新的顶层域而非并入既有域：并入已应用的域在复用库上
//! 会因台账已记账而被整体跳过，导致财务两岗的客户信用入口在存量库里永远缺位（本仓有先例）。
//! 链尾新域名在复用库上属 pending，必然执行。
//! - m0001_grant_customer_credit_family_keys：按 roles.code 反查 id，为两岗补授通道二
//!   （与 init_service_ops/permission.rs 的 finance_manager 与 accountant 分组同源的
//!   customer-credits 动作键），冲突目标用 role_permissions 的真实唯一索引列集合，
//!   逐键打印实写行数与跳过成因，同键显式拒绝行只点名不覆盖。
//!
//! 依赖关系：roles（system 域）与 role_permissions（system 域）建表、以及
//! uq_role_permissions_role_resource_action 唯一索引（domain/v15）均早于本域，故本域可
//! 安全使用 ON CONFLICT；资源名 customer-credits 在 init_service.rs 的 PERMISSION_RESOURCES
//! 已在册（客户信用族端点挂在 /api/v1/erp/crm/customer-credits 下，第四段即权威资源名，
//! 未被 utils/path_utils.rs 的消歧映射改写），本域不新增资源键、不动通道一 矩阵。
//!
//! 授予面按财务岗位职责切分（键名口径与通道一 逐字符相同）：finance_manager 五枚（read、
//! create、update、delete、release），accountant 三枚（read、create、update）；处置面（停用与
//! 额度释放）按内置的财务制单与审核互斥留在管理岗。print 本轮不授（前端零调用方）。其余岗位
//! 一律不授，尤其不用动作段通配符，通配会经前端 actionEquivalent 与域别名映射把别的域变成
//! 新增可达页并触发基线重签。
//!
//! down 按 角色码 与 本域逐枚 (resource_type, action) 配对 收窄回收，且只删 allowed=true
//! 的本域授予行（人工显式拒绝行不得被回退抹掉），可独立回退，不触碰其它角色、其它动作与
//! crm_manager 名下的通配授权行。

mod m0001_grant_customer_credit_family_keys;

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_customer_credit_family_grant"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0001_grant_customer_credit_family_keys::Migration
            .up(manager)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0001_grant_customer_credit_family_keys::Migration
            .down(manager)
            .await?;
        Ok(())
    }
}
