//! 价格域外键约束迁移（sales_prices / purchase_prices → products / customers / suppliers）
//!
//! 背景：`sales_prices` 与 `purchase_prices` 两张表自建表（business 域
//! m0009_add_purchase_extensions.rs / m0011_add_sales_and_logistics_extensions.rs）
//! 起从未声明任何外键约束（全量 grep backend/migration/src 的
//! `REFERENCES|ADD CONSTRAINT|foreign key` 证实：两表唯一命中只有 price_vocab_check
//! 的状态 CHECK）。产品/客户/供应商被删除后，价目行成为孤儿引用，而孤儿行会被
//! 当作"生效价"读取，属数据完整性缺口。本迁移补齐 4 条外键：
//! - `sales_prices.product_id`   INTEGER NOT NULL  → products.id  (SERIAL/INTEGER)
//! - `sales_prices.customer_id`  INTEGER NULL      → customers.id (SERIAL/INTEGER)
//!   （标准价行 customer_id 为 NULL，FK 天然允许 NULL，无需回填）
//! - `purchase_prices.product_id`  INTEGER NOT NULL → products.id  (SERIAL/INTEGER)
//! - `purchase_prices.supplier_id` INTEGER NOT NULL → suppliers.id (SERIAL/INTEGER)
//!
//! 列型逐型对齐核查（本仓有 i32/INTEGER vs i64/BIGINT 宽度漂移前科，任务板 #68/#237，
//! 宽度不一致时 FK 根本建不起来）：
//! - 参照表主键：system/m0001_initial_schema.rs 中 products / suppliers / customers
//!   三表均为 `"id" SERIAL PRIMARY KEY`（即 INTEGER）；
//! - 价目列：business m0009（purchase_prices）与 m0011（sales_prices）建表列
//!   product_id/customer_id/supplier_id 均为 INTEGER；
//! - 全仓 grep `ALTER COLUMN "(product_id|customer_id|supplier_id|id)" TYPE` 仅命中
//!   custom_orders（BIGINT 加宽），未触及上述任何列；m0044 fix_fk_types 只改写
//!   m0044 内嵌 SQL 的建表列声明，且其 INTEGER_ID_TABLES 白名单（products/customers/
//!   suppliers 均在列）方向是 BIGINT→INTEGER，与本批列无关。
//! 结论：四组列/主键同为 INTEGER，无宽度冲突，本迁移不设"暂缓列"。
//! SeaORM 实体侧同为 i32（models/product.rs、customer.rs、supplier.rs 与
//! sales_price.rs、purchase_price.rs 的对应字段），与 DDL 同源。
//!
//! 为什么用 DO 守卫而不是直接 ADD CONSTRAINT：PostgreSQL 在存在孤儿行时整条
//! `ADD CONSTRAINT ... FOREIGN KEY` 会失败，但报错只到 "key is not present in table"
//! 级别、不点名是哪张表多少行孤儿，运维无法据此定位脏数据；且 ADD 失败会使迁移
//! 中断在语句层，报错信息不可读。DO 守卫先逐条统计孤儿行数，非 0 即 RAISE EXCEPTION
//! 点名「表名.列名 / 孤儿行数 / 最多 10 条样例行 id」，fail-visible（口径同
//! price_vocab_check 的词表守卫与 production/m0076 的只读点名守卫）。
//!
//! 为什么不在迁移里洗数据：孤儿行只能靠业务判断"是补建主数据还是删价目行"，
//! 迁移里 UPDATE/DELETE 猜洗等于伪造引用关系（本仓 fail-closed 铁律）。摸出脏数据
//! 交业务侧核实更正后重跑本迁移。
//!
//! 幂等：up 先对四条约束 `DROP CONSTRAINT IF EXISTS` 再加守卫与 ADD，可安全重跑；
//! 守卫通过后 ADD 前约束必不存在，不会撞重名。注册在迁移链尾（由主编排在
//! migration/src/domain/mod.rs 与 lib.rs 完成），晚于全部建表迁移（system m0001、
//! business m0009/m0011），全新迁移库上链跑到此时四组引用关系均已定型。
//!
//! 删除行为说明：不加 ON DELETE 子句即 PostgreSQL 默认 NO ACTION（等价 RESTRICT，
//! 与仓内既有 FK 如 fk_purchase_receipt_supplier / fk_sales_delivery_customer 同形），
//! 被引用的产品/客户/供应商行将被拒绝删除——这正是补齐 FK 的目的；若未来业务需要
//! 级联清理或软删联动，属独立决策，不在本迁移内夹带。
//!
//! 存量库上线前先做只读摸底（守卫按设计中止，摸出孤儿交业务定性，不在迁移里猜洗）：
//!
//! ```text
//! SELECT 'sales_prices.product_id -> products.id' AS pair, COUNT(*) AS orphans
//!   FROM sales_prices sp
//!  WHERE NOT EXISTS (SELECT 1 FROM products p WHERE p.id = sp.product_id)
//! UNION ALL
//! SELECT 'sales_prices.customer_id -> customers.id', COUNT(*)
//!   FROM sales_prices sp
//!  WHERE sp.customer_id IS NOT NULL
//!    AND NOT EXISTS (SELECT 1 FROM customers c WHERE c.id = sp.customer_id)
//! UNION ALL
//! SELECT 'purchase_prices.product_id -> products.id', COUNT(*)
//!   FROM purchase_prices pp
//!  WHERE NOT EXISTS (SELECT 1 FROM products p WHERE p.id = pp.product_id)
//! UNION ALL
//! SELECT 'purchase_prices.supplier_id -> suppliers.id', COUNT(*)
//!   FROM purchase_prices pp
//!  WHERE NOT EXISTS (SELECT 1 FROM suppliers s WHERE s.id = pp.supplier_id);
//! ```
//!
//! 任一行为 > 0 即需人工逐行定性（补建被删主数据，或删除/更正价目行），更正后
//! 重跑本迁移；建议先在结构等价的存量快照库预跑一次，把中止暴露在预发而非生产。

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_price_fk"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1) 幂等前置：删旧约束（若上一轮已加或历史遗留同名），保证后面 ADD 不撞重名，
        //    本段之外的一切存量判定都交给 DO 守卫，此处不动任何数据。
        let setup = r#"
ALTER TABLE "sales_prices"    DROP CONSTRAINT IF EXISTS "fk_sales_prices_product";
ALTER TABLE "sales_prices"    DROP CONSTRAINT IF EXISTS "fk_sales_prices_customer";
ALTER TABLE "purchase_prices" DROP CONSTRAINT IF EXISTS "fk_purchase_prices_product";
ALTER TABLE "purchase_prices" DROP CONSTRAINT IF EXISTS "fk_purchase_prices_supplier";
"#;
        manager.get_connection().execute_unprepared(setup).await?;

        // 2) 存量孤儿守卫（fail-visible，严禁 UPDATE/DELETE 猜洗数据）：逐列统计
        //    孤儿行数并采样例行 id；非 0 即点名拒绝，本迁移不加 FK 直接中止。
        let guard = r#"
DO $$
DECLARE
    bad_rows INTEGER;
    samples  TEXT;
BEGIN
    SELECT COUNT(*) INTO bad_rows FROM "sales_prices" sp
      WHERE NOT EXISTS (SELECT 1 FROM "products" p WHERE p."id" = sp."product_id");
    IF bad_rows = 0 THEN
        RAISE NOTICE 'm_price_fk：sales_prices.product_id 无孤儿引用行，可安全施加 FK fk_sales_prices_product。';
    ELSE
        SELECT string_agg(sample, ', ') INTO samples
          FROM (SELECT sp."id"::text AS sample
                  FROM "sales_prices" sp
                 WHERE NOT EXISTS (SELECT 1 FROM "products" p WHERE p."id" = sp."product_id")
                 LIMIT 10) s;
        RAISE EXCEPTION 'm_price_fk：sales_prices.product_id 存在 % 行孤儿引用（指向 products.id 不存在，样例行 id：%），拒绝添加 FK fk_sales_prices_product。', bad_rows, samples
            USING HINT = '本迁移拒绝 UPDATE/DELETE 猜洗数据：孤儿行只能由业务定性（补建被删产品或删除/更正价目行），先跑文件头只读摸底 SQL 逐行核实，更正后重跑迁移。';
    END IF;

    SELECT COUNT(*) INTO bad_rows FROM "sales_prices" sp
      WHERE sp."customer_id" IS NOT NULL
        AND NOT EXISTS (SELECT 1 FROM "customers" c WHERE c."id" = sp."customer_id");
    IF bad_rows = 0 THEN
        RAISE NOTICE 'm_price_fk：sales_prices.customer_id 无孤儿引用行（NULL 标准价行不违反 FK），可安全施加 FK fk_sales_prices_customer。';
    ELSE
        SELECT string_agg(sample, ', ') INTO samples
          FROM (SELECT sp."id"::text AS sample
                  FROM "sales_prices" sp
                 WHERE sp."customer_id" IS NOT NULL
                   AND NOT EXISTS (SELECT 1 FROM "customers" c WHERE c."id" = sp."customer_id")
                 LIMIT 10) s;
        RAISE EXCEPTION 'm_price_fk：sales_prices.customer_id 存在 % 行孤儿引用（指向 customers.id 不存在，样例行 id：%），拒绝添加 FK fk_sales_prices_customer。', bad_rows, samples
            USING HINT = '本迁移拒绝 UPDATE/DELETE 猜洗数据：孤儿行只能由业务定性（补建被删客户或把该行改挂有效客户/置 NULL 为标准价），先跑文件头只读摸底 SQL 逐行核实，更正后重跑迁移。';
    END IF;

    SELECT COUNT(*) INTO bad_rows FROM "purchase_prices" pp
      WHERE NOT EXISTS (SELECT 1 FROM "products" p WHERE p."id" = pp."product_id");
    IF bad_rows = 0 THEN
        RAISE NOTICE 'm_price_fk：purchase_prices.product_id 无孤儿引用行，可安全施加 FK fk_purchase_prices_product。';
    ELSE
        SELECT string_agg(sample, ', ') INTO samples
          FROM (SELECT pp."id"::text AS sample
                  FROM "purchase_prices" pp
                 WHERE NOT EXISTS (SELECT 1 FROM "products" p WHERE p."id" = pp."product_id")
                 LIMIT 10) s;
        RAISE EXCEPTION 'm_price_fk：purchase_prices.product_id 存在 % 行孤儿引用（指向 products.id 不存在，样例行 id：%），拒绝添加 FK fk_purchase_prices_product。', bad_rows, samples
            USING HINT = '本迁移拒绝 UPDATE/DELETE 猜洗数据：孤儿行只能由业务定性（补建被删产品或删除/更正价目行），先跑文件头只读摸底 SQL 逐行核实，更正后重跑迁移。';
    END IF;

    SELECT COUNT(*) INTO bad_rows FROM "purchase_prices" pp
      WHERE NOT EXISTS (SELECT 1 FROM "suppliers" s WHERE s."id" = pp."supplier_id");
    IF bad_rows = 0 THEN
        RAISE NOTICE 'm_price_fk：purchase_prices.supplier_id 无孤儿引用行，可安全施加 FK fk_purchase_prices_supplier。';
    ELSE
        SELECT string_agg(sample, ', ') INTO samples
          FROM (SELECT pp."id"::text AS sample
                  FROM "purchase_prices" pp
                 WHERE NOT EXISTS (SELECT 1 FROM "suppliers" s WHERE s."id" = pp."supplier_id")
                 LIMIT 10) s;
        RAISE EXCEPTION 'm_price_fk：purchase_prices.supplier_id 存在 % 行孤儿引用（指向 suppliers.id 不存在，样例行 id：%），拒绝添加 FK fk_purchase_prices_supplier。', bad_rows, samples
            USING HINT = '本迁移拒绝 UPDATE/DELETE 猜洗数据：孤儿行只能由业务定性（补建被删供应商或删除/更正价目行），先跑文件头只读摸底 SQL 逐行核实，更正后重跑迁移。';
    END IF;
END;
$$;
"#;
        manager.get_connection().execute_unprepared(guard).await?;

        // 3) 守卫全部通过后施加 4 条 FK。列型与主键同为 INTEGER（见文件头核查），
        //    不带 ON DELETE 子句 = 默认 NO ACTION，与仓内既有 FK 同形。
        let apply = r#"
ALTER TABLE "sales_prices"    ADD CONSTRAINT "fk_sales_prices_product"   FOREIGN KEY ("product_id")  REFERENCES "products"  ("id");
ALTER TABLE "sales_prices"    ADD CONSTRAINT "fk_sales_prices_customer"  FOREIGN KEY ("customer_id") REFERENCES "customers" ("id");
ALTER TABLE "purchase_prices" ADD CONSTRAINT "fk_purchase_prices_product"  FOREIGN KEY ("product_id")  REFERENCES "products"  ("id");
ALTER TABLE "purchase_prices" ADD CONSTRAINT "fk_purchase_prices_supplier" FOREIGN KEY ("supplier_id") REFERENCES "suppliers" ("id");
"#;
        manager.get_connection().execute_unprepared(apply).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 回滚仅对称移除 4 条 FK，回到"可旁路写入"状态，不回退、不删除任何数据
        // （口径同 price_vocab_check 的 down；不留空实现，空 down 会被判缺陷，
        // 前科见任务板 #214 rls_dept）。
        let sql = r#"
ALTER TABLE "sales_prices"    DROP CONSTRAINT IF EXISTS "fk_sales_prices_product";
ALTER TABLE "sales_prices"    DROP CONSTRAINT IF EXISTS "fk_sales_prices_customer";
ALTER TABLE "purchase_prices" DROP CONSTRAINT IF EXISTS "fk_purchase_prices_product";
ALTER TABLE "purchase_prices" DROP CONSTRAINT IF EXISTS "fk_purchase_prices_supplier";
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
