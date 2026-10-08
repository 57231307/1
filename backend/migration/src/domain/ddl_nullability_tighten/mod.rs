//! 列形态收紧域（模型非 Option、DDL 却可空的落差归位）
//!
//! 本域只做一件事：把「写入方恒给值、模型侧声明为非 Option，而数据库列仍可空（部分连
//! DEFAULT 都没落地）」的列收紧成与语义一致的 `NOT NULL + DEFAULT`。方向选择依据仓内既有
//! 同型裁定——收紧 DDL 而非把模型翻成 `Option`（后者会让既有非空假设的读取点全线动摇）。
//!
//! - m0092_tighten_order_amount_and_color_columns: 销售订单七条金额列 + 采购订单总额列
//!   + 销售明细/发货两条色号列。
//! - m0093_tighten_flag_and_status_columns: 角色内置标志、用户 TOTP 标志、三条主数据状态列。
//! - m0094_tighten_priority_and_webhook_columns: 客户专属价排序权重、通知出站开关
//!   （这两列的建表整句 `NOT NULL DEFAULT` 曾被更早的裸列定义吞成 no-op，默认一并丢失，
//!   故必须先补默认再收紧，并在收尾自证里复核默认确已生效）。
//!
//! 注册位置：lib.rs 全链尾。被触及的表分别由 system / business / sales_crm / production /
//! price 等域建表，收紧必须晚于建表；同域内三条按 up 顺序串行、down 逆序，
//! 任一列存量不合预期即整条中止（单事务，不留半态）。
//!
//! 每条迁移都保留「fail-visible 点名存量 NULL」守卫且绝不做 UPDATE 造值归一：
//! 收紧的正当性来自"存量本就无 NULL"，若真有 NULL，那是业务数据待定性，不在迁移里圆场。

mod m0092_tighten_order_amount_and_color_columns;
mod m0093_tighten_flag_and_status_columns;
mod m0094_tighten_priority_and_webhook_columns;

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_ddl_nullability_tighten"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0092_tighten_order_amount_and_color_columns::Migration
            .up(manager)
            .await?;
        m0093_tighten_flag_and_status_columns::Migration
            .up(manager)
            .await?;
        m0094_tighten_priority_and_webhook_columns::Migration
            .up(manager)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0094_tighten_priority_and_webhook_columns::Migration
            .down(manager)
            .await?;
        m0093_tighten_flag_and_status_columns::Migration
            .down(manager)
            .await?;
        m0092_tighten_order_amount_and_color_columns::Migration
            .down(manager)
            .await?;
        Ok(())
    }
}
