//! 应收族（AR）授权面补授（finance_manager 与 accountant）聚合迁移域。
//!
//! 本域只落一条数据迁移台账，登记为新的顶层域而非并入既有域：并入已应用的域在复用库上
//! 会因台账已记账而被整体跳过，导致应收族入口在存量库里永远缺位（本仓有先例，见
//! contract_clause_snapshot_provenance 域头同一记述）。链尾新域名在复用库上属 pending，
//! 必然执行。
//! - m0125_grant_ar_family_keys 按 roles.code 反查 id，先探测本域八枚资源名在存量库的
//!   既有授权行并逐枚点名缺失项，再为两岗补授通道①
//!   （init_service_ops/permission.rs 的 finance_manager 与 accountant 分组）同源的应收族
//!   键，冲突目标用 role_permissions 的真实唯一索引列集合，逐键打印实写行数与跳过成因，
//!   同键显式拒绝行只点名不覆盖。
//!
//! 依赖关系：roles 与 role_permissions 建表（system 域）以及
//! uq_role_permissions_role_resource_action 唯一索引（m_v15_domain）均早于本域，故本域可安全
//! 使用 ON CONFLICT。八枚资源名（ar-invoices、ar-payments、ar-verifications、ar-collections、
//! ar-aging、ar-daily、ar-monthly、ar-statistics）在 utils/path_utils.rs 的消歧映射与
//! init_service.rs 的 PERMISSION_RESOURCES 均已在册（实测两处逐枚命中），本域不新增资源键、
//! 不动通道① 的在册清单，只补角色配对。
//!
//! 与应付族的一处形态差需要点名：应付族授予面里有 ap-payment-requests 与 ap-reconciliations
//! 两枚，应收族的对账面走另一枚资源名 ar-reconciliations（挂在根级第四段，不经 ar 前缀消歧），
//! 该资源通道① 早已在册（finance_manager 全权、accountant 读与建单），故本域不重复授它，
//! 只补 ar 前缀下这八枚从未落过角色行的细键。
//!
//! 出纳（cashier）刻意不在本域角色集内：口径是应收族的审核面与报表面归管理岗、制单记账面归
//! 会计，出纳不持任何 ar-* 键，不在本域擅自补授。
//!
//! down 按「角色码 乘 本域逐枚 (resource_type, action) 配对」收窄回收，可独立回退，
//! 不触碰其它角色、其它动作与通配授予。

mod m0125_grant_ar_family_keys;

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_ar_family_grant"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0125_grant_ar_family_keys::Migration.up(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0125_grant_ar_family_keys::Migration.down(manager).await?;
        Ok(())
    }
}
