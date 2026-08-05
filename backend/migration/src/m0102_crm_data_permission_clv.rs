use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_query::Table;

/// V15 P2 18.4-D5/D6 + 18.5-D3/D4/D5: CRM 数据权限+数据流转增强
///
/// - 18.4-D5: 客户字段权限
/// - 18.4-D6: 客户操作日志
/// - 18.5-D3: 转化数据双向同步
/// - 18.5-D4: 客户主数据关系
/// - 18.5-D5: 客户全生命周期价值（CLV）
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 18.4-D5: 客户字段权限配置表
        manager
            .create_table(
                Table::create()
                    .table(CustomerFieldPermission::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(CustomerFieldPermission::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(CustomerFieldPermission::RoleId).integer().not_null().comment("角色ID"))
                    .col(ColumnDef::new(CustomerFieldPermission::FieldName).string().not_null().comment("字段名称"))
                    .col(ColumnDef::new(CustomerFieldPermission::Permission).string().not_null().comment("权限：visible/hidden/masked"))
                    .col(ColumnDef::new(CustomerFieldPermission::MaskPattern).string().null().comment("脱敏模式"))
                    .col(ColumnDef::new(CustomerFieldPermission::CreatedAt).timestamp_with_time_zone().default(Expr::current_timestamp()))
                    .col(ColumnDef::new(CustomerFieldPermission::UpdatedAt).timestamp_with_time_zone().default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await?;

        // 18.4-D6: 客户操作日志表
        manager
            .create_table(
                Table::create()
                    .table(CustomerAuditLog::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(CustomerAuditLog::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(CustomerAuditLog::CustomerId).integer().not_null().comment("客户ID"))
                    .col(ColumnDef::new(CustomerAuditLog::Operation).string().not_null().comment("操作类型：create/update/delete/view/export"))
                    .col(ColumnDef::new(CustomerAuditLog::FieldName).string().null().comment("变更字段"))
                    .col(ColumnDef::new(CustomerAuditLog::OldValue).text().null().comment("旧值"))
                    .col(ColumnDef::new(CustomerAuditLog::NewValue).text().null().comment("新值"))
                    .col(ColumnDef::new(CustomerAuditLog::UserId).integer().not_null().comment("操作人ID"))
                    .col(ColumnDef::new(CustomerAuditLog::UserName).string().not_null().comment("操作人姓名"))
                    .col(ColumnDef::new(CustomerAuditLog::IpAddress).string().null().comment("IP地址"))
                    .col(ColumnDef::new(CustomerAuditLog::UserAgent).string().null().comment("用户代理"))
                    .col(ColumnDef::new(CustomerAuditLog::CreatedAt).timestamp_with_time_zone().default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await?;

        // 18.5-D5: 客户全生命周期价值（CLV）表
        manager
            .create_table(
                Table::create()
                    .table(CustomerLifetimeValue::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(CustomerLifetimeValue::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(CustomerLifetimeValue::CustomerId).integer().not_null().comment("客户ID"))
                    .col(ColumnDef::new(CustomerLifetimeValue::TotalOrders).integer().default(0).comment("总订单数"))
                    .col(ColumnDef::new(CustomerLifetimeValue::TotalRevenue).decimal_len(15, 2).default(0).comment("总收入"))
                    .col(ColumnDef::new(CustomerLifetimeValue::AvgOrderValue).decimal_len(15, 2).default(0).comment("平均订单金额"))
                    .col(ColumnDef::new(CustomerLifetimeValue::FirstOrderDate).date().null().comment("首次订单日期"))
                    .col(ColumnDef::new(CustomerLifetimeValue::LastOrderDate).date().null().comment("最近订单日期"))
                    .col(ColumnDef::new(CustomerLifetimeValue::CustomerLifespanDays).integer().default(0).comment("客户生命周期天数"))
                    .col(ColumnDef::new(CustomerLifetimeValue::PurchaseFrequency).decimal_len(10, 2).default(0).comment("购买频率（订单数/年）"))
                    .col(ColumnDef::new(CustomerLifetimeValue::ClvScore).decimal_len(15, 2).default(0).comment("CLV评分"))
                    .col(ColumnDef::new(CustomerLifetimeValue::Segment).string().null().comment("客户分层：champion/loyal/potential/at_risk/lost"))
                    .col(ColumnDef::new(CustomerLifetimeValue::CalculatedAt).timestamp_with_time_zone().default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await?;

        // 创建索引
        manager
            .create_index(
                Index::create()
                    .name("idx_customer_audit_log_customer_id")
                    .table(CustomerAuditLog::Table)
                    .col(CustomerAuditLog::CustomerId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_customer_field_permission_role_field")
                    .table(CustomerFieldPermission::Table)
                    .col(CustomerFieldPermission::RoleId)
                    .col(CustomerFieldPermission::FieldName)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_customer_clv_customer_id")
                    .table(CustomerLifetimeValue::Table)
                    .col(CustomerLifetimeValue::CustomerId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(CustomerLifetimeValue::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(CustomerAuditLog::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(CustomerFieldPermission::Table).to_owned()).await?;
        Ok(())
    }
}

#[derive(Iden)]
enum CustomerFieldPermission {
    Table,
    Id,
    RoleId,
    FieldName,
    Permission,
    MaskPattern,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum CustomerAuditLog {
    Table,
    Id,
    CustomerId,
    Operation,
    FieldName,
    OldValue,
    NewValue,
    UserId,
    UserName,
    IpAddress,
    UserAgent,
    CreatedAt,
}

#[derive(Iden)]
enum CustomerLifetimeValue {
    Table,
    Id,
    CustomerId,
    TotalOrders,
    TotalRevenue,
    AvgOrderValue,
    FirstOrderDate,
    LastOrderDate,
    CustomerLifespanDays,
    PurchaseFrequency,
    ClvScore,
    Segment,
    CalculatedAt,
}
