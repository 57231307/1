use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create `roles` table
        manager
            .create_table(
                Table::create()
                    .table(Roles::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Roles::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Roles::Name).string_len(100).not_null())
                    .col(
                        ColumnDef::new(Roles::Code)
                            .string_len(50)
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Roles::Description).text())
                    .col(ColumnDef::new(Roles::Permissions).text())
                    .col(ColumnDef::new(Roles::IsSystem).boolean().default(false))
                    .col(
                        ColumnDef::new(Roles::IsDeleted)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Roles::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Roles::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Create `departments` table
        manager
            .create_table(
                Table::create()
                    .table(Departments::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Departments::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Departments::Name).string_len(100).not_null())
                    .col(
                        ColumnDef::new(Departments::Code)
                            .string_len(50)
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Departments::ParentId).integer())
                    .col(ColumnDef::new(Departments::ManagerId).integer())
                    .col(ColumnDef::new(Departments::Description).text())
                    .col(ColumnDef::new(Departments::SortOrder).integer().default(0))
                    .col(
                        ColumnDef::new(Departments::IsActive)
                            .boolean()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(Departments::IsDeleted)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Departments::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Departments::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Create `users` table
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Users::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Users::Username)
                            .string_len(100)
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Users::PasswordHash)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Users::Email).string_len(255))
                    .col(ColumnDef::new(Users::Phone).string_len(50))
                    .col(ColumnDef::new(Users::RoleId).integer())
                    .col(ColumnDef::new(Users::DepartmentId).integer())
                    .col(ColumnDef::new(Users::IsActive).boolean().default(true))
                    .col(ColumnDef::new(Users::TotpSecret).string_len(255))
                    .col(
                        ColumnDef::new(Users::IsTotpEnabled)
                            .boolean()
                            .default(false),
                    )
                    .col(ColumnDef::new(Users::LastLoginAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(Users::IsDeleted)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Users::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Users::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Indexes for `users`
        manager
            .create_index(
                Index::create()
                    .name("idx_users_username")
                    .table(Users::Table)
                    .col(Users::Username)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_users_role_id")
                    .table(Users::Table)
                    .col(Users::RoleId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_users_department_id")
                    .table(Users::Table)
                    .col(Users::DepartmentId)
                    .to_owned(),
            )
            .await?;

        // Indexes for `departments`
        manager
            .create_index(
                Index::create()
                    .name("idx_departments_parent_id")
                    .table(Departments::Table)
                    .col(Departments::ParentId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_departments_manager_id")
                    .table(Departments::Table)
                    .col(Departments::ManagerId)
                    .to_owned(),
            )
            .await?;

        let sql = include_str!("../../migrations/20260323000001_initial_schema/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260323000001_initial_schema/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }

        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Departments::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Roles::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
    Username,
    PasswordHash,
    Email,
    Phone,
    RoleId,
    DepartmentId,
    IsActive,
    TotpSecret,
    IsTotpEnabled,
    LastLoginAt,
    IsDeleted,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Roles {
    Table,
    Id,
    Name,
    Code,
    Description,
    Permissions,
    IsSystem,
    IsDeleted,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Departments {
    Table,
    Id,
    Name,
    Code,
    ParentId,
    ManagerId,
    Description,
    SortOrder,
    IsActive,
    IsDeleted,
    CreatedAt,
    UpdatedAt,
}
