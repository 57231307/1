//! 销售合同挂销售订单溯源链。为 sales_contracts 补可空列 source_order_id，外键指向
//! sales_orders 主键并随订单删除置空；为 sales_contract_items 补可空列
//! source_order_item_id，外键指向 sales_order_items 主键并随订单行删除置空，同时补行级
//! 金额口径四列 discount_percent、discount_amount、tax_percent、tax_amount 与行级色号列
//! color_no，两表各补一条检索索引。
//!
//! ## 用途
//! 销售订单与销售合同此前零通路，合同只能整表手填，订单明细的数量、单价、折扣、税率、
//! 色号要靠人工重抄进合同，既慢又容易抄错金额口径。补链后：
//! - 合同表头记录它源自哪张销售订单，合同明细行记录它对应订单的哪一行。从订单跳合同、
//!   从合同跳回订单都有唯一判据，重复转单的判定只看这两列，不靠备注文本比对；
//! - 合同明细行补齐折扣与税额四列，使合同行金额口径与订单行金额口径逐项可比，订单行的
//!   折扣率与税率能原值落入合同行，不需要挪用文本备注承载数值；
//! - 合同明细行补 color_no，纺织订单的行级色号不再在转单时丢失，空串语义与订单行一致，
//!   即白坯布。
//!
//! ## 精度与列宽依据
//! 新增四列的列宽逐列取订单侧同名列的真实列宽，保证带入过程只可能等价、不可能截断。
//! sales_order_items 的 discount_percent、discount_amount、tax_percent、tax_amount 建列
//! 即为 numeric 18 位小数 4 位，本域同用 numeric 18 位小数 4 位；sales_order_items 的
//! color_no 为 varchar 255，本域同用 varchar 255。既有 sales_contract_items.quantity 为
//! numeric 15 位小数 2 位，与订单行 quantity 的小数位同为 2 位；unit_price 为 numeric
//! 15 位小数 4 位，宽于订单行 unit_price 的 12 位小数 2 位。两者带入都不丢精度，故本域
//! 不改动这两列。
//!
//! ## 语义与幂等
//! - 全部新列可空且不回填任何存量行。存量合同 source_order_id 为 NULL 的语义是手工建档、
//!   无来源订单，与已转单可严格区分；手工建单链路不写这些列，保持原行为。
//! - ON DELETE SET NULL。订单被删除时合同与其明细行都不连带删除，只失去溯源指向。已签署
//!   合同是法律凭证，不能因来源单据被清理而消失。
//! - ADD COLUMN IF NOT EXISTS、CREATE INDEX IF NOT EXISTS、DROP IF EXISTS 全部幂等可重放，
//!   无备份表即可精确回退。
//! - 重复转单的互斥判据不落在本域的部分唯一索引上。已作废或已驳回的合同允许重新转，若在
//!   索引里写死状态集合，就会与合同状态词表 draft、active、cancelled、rejected 形成两套
//!   口径并随词表演化而漂移。互斥由转单服务在同一事务内对来源订单行加排他锁后按词表判定，
//!   本域索引只承担检索加速。
//!
//! ## 注册位置
//! 作为链尾新顶层域而非并入既有域。并入已应用的域在复用库上会因台账已记账而被整体跳过，
//! 导致列永不补上；链尾新域名在复用库上属 pending，必然执行。依赖 sales_contracts 与
//! sales_contract_items（business 与 v15 域建表）以及 sales_orders 与 sales_order_items
//! （system 域建表），四者均早于本域完成。

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_sales_contract_source_order"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();

        // 合同表头溯源列。调用方为转单服务建单路径与合同列表的按来源订单筛选，存的是
        // 该合同由哪张销售订单生成
        conn.execute_unprepared(
            r#"ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "source_order_id" INTEGER REFERENCES "sales_orders"("id") ON DELETE SET NULL;"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"COMMENT ON COLUMN "sales_contracts"."source_order_id" IS '来源销售订单（可空）。NULL 表示手工建档合同，非空表示由该订单转单生成的合同。外键指向 sales_orders 主键，订单删除时本列置空而合同行保留，已签署合同是凭证不随来源单据清理消失。写入方是销售合同转单服务，读取方是合同列表按来源订单筛选与订单转合同入口的重复判定';"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"CREATE INDEX IF NOT EXISTS "idx_sales_contracts_source_order_id" ON "sales_contracts" ("source_order_id");"#,
        )
        .await?;

        // 合同明细行溯源列。存的是该行合同明细对应哪一行销售订单明细
        conn.execute_unprepared(
            r#"ALTER TABLE "sales_contract_items" ADD COLUMN IF NOT EXISTS "source_order_item_id" INTEGER REFERENCES "sales_order_items"("id") ON DELETE SET NULL;"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"COMMENT ON COLUMN "sales_contract_items"."source_order_item_id" IS '来源销售订单明细行（可空）。NULL 表示手工录入的合同行，非空表示由该订单行带入的合同行。转单时服务端按本行回读订单行的折扣率与税率并重算金额，客户端不申报税率，因此合同与订单的金额算法只有一个来源。写入方是销售合同转单服务';"#,
        )
        .await?;

        // 合同明细行金额口径四列，列宽与订单同名列一致，带入不截断
        conn.execute_unprepared(
            r#"ALTER TABLE "sales_contract_items" ADD COLUMN IF NOT EXISTS "discount_percent" NUMERIC(18,4);"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE "sales_contract_items" ADD COLUMN IF NOT EXISTS "discount_amount" NUMERIC(18,4);"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE "sales_contract_items" ADD COLUMN IF NOT EXISTS "tax_percent" NUMERIC(18,4);"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE "sales_contract_items" ADD COLUMN IF NOT EXISTS "tax_amount" NUMERIC(18,4);"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE "sales_contract_items" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255);"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"COMMENT ON COLUMN "sales_contract_items"."discount_percent" IS '折扣率百分比（可空）。手工录入行为 NULL 表示未采集，转单行按来源订单行原值带入。列宽与 sales_order_items.discount_percent 一致，numeric 18 位小数 4 位';"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"COMMENT ON COLUMN "sales_contract_items"."discount_amount" IS '折扣金额（可空）。由服务端按数量乘单价再乘折扣率算出，不采信客户端送来的值。列宽与 sales_order_items.discount_amount 一致';"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"COMMENT ON COLUMN "sales_contract_items"."tax_percent" IS '税率百分比（可空）。手工录入行为 NULL 表示未采集，转单行按来源订单行原值带入，入参边界校验 0 到 100。列宽与 sales_order_items.tax_percent 一致';"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"COMMENT ON COLUMN "sales_contract_items"."tax_amount" IS '税额（可空）。由服务端按折扣后金额乘税率算出，与订单行共用同一算法模块。列宽与 sales_order_items.tax_amount 一致';"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"COMMENT ON COLUMN "sales_contract_items"."color_no" IS '色号（可空）。NULL 表示手工录入行未采集，空串表示白坯布，非空表示染色布，与 sales_order_items.color_no 同一语义。转单时按来源订单行原值带入';"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"CREATE INDEX IF NOT EXISTS "idx_sales_contract_items_source_order_item_id" ON "sales_contract_items" ("source_order_item_id");"#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();

        conn.execute_unprepared(
            r#"DROP INDEX IF EXISTS "idx_sales_contract_items_source_order_item_id";"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE "sales_contract_items" DROP COLUMN IF EXISTS "color_no";"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE "sales_contract_items" DROP COLUMN IF EXISTS "tax_amount";"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE "sales_contract_items" DROP COLUMN IF EXISTS "tax_percent";"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE "sales_contract_items" DROP COLUMN IF EXISTS "discount_amount";"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE "sales_contract_items" DROP COLUMN IF EXISTS "discount_percent";"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE "sales_contract_items" DROP COLUMN IF EXISTS "source_order_item_id";"#,
        )
        .await?;

        conn.execute_unprepared(
            r#"DROP INDEX IF EXISTS "idx_sales_contracts_source_order_id";"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE "sales_contracts" DROP COLUMN IF EXISTS "source_order_id";"#,
        )
        .await?;

        Ok(())
    }
}
