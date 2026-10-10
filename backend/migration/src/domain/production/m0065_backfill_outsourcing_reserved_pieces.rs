//! 委外发料匹状态占用闭环上线的存量回填
//!
//! 背景：发料链路此前从不写 `inventory_piece.status`（占用闭环上线前，被
//! issued/processing 委外单引用的胚布匹永远停在 AVAILABLE，同一实物可被重复发料）。
//! 本迁移把「非 draft、未删除、且未取消的委外订单明细所引用的、当前仍 AVAILABLE
//! 的匹」补置 RESERVED，使词表语义（RESERVED=已为订单预留，
//! src/models/status/purchase_inventory.rs::inventory_piece）与存量事实对齐。
//!
//! 口径说明（词表语义推导，非自创规则）：cancelled 订单不纳入占用面——
//! 取消即释放是本次闭环的定义行为（order.rs::cancel 走 CAS RESERVED→AVAILABLE），
//! 若把已取消订单的匹回填成 RESERVED，将造成永久冻结、对所有后续发料不可用，
//! 与词表语义直接矛盾。draft 未发料、从未占用，同样排除。
//! received/settled/closed 单的匹按定案统一回填 RESERVED：其原胚布匹至少
//! 保证不再 AVAILABLE（杜绝重复发料这一核心缺陷）；把它们推断成 SHIPPED 需要
//! 假设历史收回事实，属超出本迁移证据的猜测式改写，不做。
//!
//! 状态字面量说明：migration crate 不依赖主 crate（循环依赖），SQL 内只能写
//! 词表原文；'AVAILABLE'/'RESERVED' 逐字符取自
//! src/models/status/purchase_inventory.rs::inventory_piece，
//! 'draft'/'cancelled' 取自 src/models/status/wage_energy_chemical_business.rs::
//! outsourcing_order_status（小写值）。比较点与写入值不得偏离。
//!
//! 存量异常处置（fail-visible，照 m0063 范式，禁止静默跳过）：
//! A) 同一匹号被多张不同订单交叉引用——占用闭环上线前重复发料的直接证据，
//!    RESERVED 无法归因到唯一订单，回填会替人工决定「哪张单占着这匹」；
//! B) 引用匹号在 inventory_piece 命中多行（m0051 部分唯一索引仅约束 greige，
//!    染色匹同串号/索引生效前的遗留行都会让条件更新批量写多行）；
//! C) 引用匹号在 inventory_piece 无行（悬空引用，UPDATE 静默 0 行会把完整性
//!    异常吞成"回填成功"）。
//! 任一命中即 RAISE EXCEPTION 中止迁移（消息含计数与最多 10 个样例匹号），
//! 由人工对照业务处置存量数据后重跑。
//!
//! 域内注册位置：目标表 outsourcing_order（v15/mod.rs:3227）与
//! outsourcing_order_item（v15/mod.rs:671）均在 v15 域内建表，而 production 域
//! 早于 v15 执行（lib.rs:34 vs :36）；直接注册本域 up() 会因
//! "relation ... does not exist" 中断迁移链（先例教训，见 m0058/m0063
//! 文件头）。故照 m0063 先例：迁移文件留在 production 域（m00NN 命名序列），
//! up/down 由 domain/v15/mod.rs 在全部建表完成后调用。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
-- A) 交叉引用检测：同一匹号被 >=2 张不同订单引用（占用归属歧义）
DO $$
DECLARE
    cross_ref_count INTEGER;
    sample_pieces TEXT;
BEGIN
    SELECT COUNT(*) INTO cross_ref_count
      FROM (
        SELECT i."piece_no"
          FROM "outsourcing_order_item" i
          JOIN "outsourcing_order" o ON o."id" = i."outsourcing_order_id"
         WHERE o."status" NOT IN ('draft', 'cancelled')
           AND o."is_deleted" = FALSE
           AND i."piece_no" IS NOT NULL AND i."piece_no" <> ''
         GROUP BY i."piece_no"
        HAVING COUNT(DISTINCT i."outsourcing_order_id") > 1
      ) d;
    IF cross_ref_count > 0 THEN
        SELECT string_agg("piece_no", ', ' ORDER BY "piece_no")
          INTO sample_pieces
          FROM (
            SELECT i."piece_no"
              FROM "outsourcing_order_item" i
              JOIN "outsourcing_order" o ON o."id" = i."outsourcing_order_id"
             WHERE o."status" NOT IN ('draft', 'cancelled')
               AND o."is_deleted" = FALSE
               AND i."piece_no" IS NOT NULL AND i."piece_no" <> ''
             GROUP BY i."piece_no"
            HAVING COUNT(DISTINCT i."outsourcing_order_id") > 1
            LIMIT 10
          ) s;
        RAISE EXCEPTION '生产匹 % 个匹号同时被多张非 draft/非 cancelled 委外订单明细引用（样例：%）。这是占用闭环上线前同一实物被重复发料的存量证据，回填无法把 RESERVED 归因到唯一订单：本迁移拒绝替人工决定归属，请逐匹处置（取消多余订单或更正明细匹号）后重跑。',
            cross_ref_count, sample_pieces;
    END IF;
END
$$;

-- B) 匹号命中多行检测：同一匹号在 inventory_piece 存在多行（写入对象歧义）
DO $$
DECLARE
    multi_row_count INTEGER;
    sample_pieces TEXT;
BEGIN
    SELECT COUNT(*) INTO multi_row_count
      FROM (
        SELECT i."piece_no"
          FROM "outsourcing_order_item" i
          JOIN "outsourcing_order" o ON o."id" = i."outsourcing_order_id"
          JOIN "inventory_piece" p ON p."piece_no" = i."piece_no"
         WHERE o."status" NOT IN ('draft', 'cancelled')
           AND o."is_deleted" = FALSE
           AND i."piece_no" IS NOT NULL AND i."piece_no" <> ''
         GROUP BY i."piece_no"
        HAVING COUNT(DISTINCT p."id") > 1
      ) d;
    IF multi_row_count > 0 THEN
        SELECT string_agg("piece_no", ', ' ORDER BY "piece_no")
          INTO sample_pieces
          FROM (
            SELECT i."piece_no"
              FROM "outsourcing_order_item" i
              JOIN "outsourcing_order" o ON o."id" = i."outsourcing_order_id"
              JOIN "inventory_piece" p ON p."piece_no" = i."piece_no"
             WHERE o."status" NOT IN ('draft', 'cancelled')
               AND o."is_deleted" = FALSE
               AND i."piece_no" IS NOT NULL AND i."piece_no" <> ''
             GROUP BY i."piece_no"
            HAVING COUNT(DISTINCT p."id") > 1
            LIMIT 10
          ) s;
        RAISE EXCEPTION '委外明细引用的 % 个匹号在 inventory_piece 命中多行（样例：%）。匹号写入目标歧义（同串号染色匹或 m0051 唯一索引生效前的遗留行），条件更新会批量改写多行状态：请人工核实匹档案唯一性后重跑。',
            multi_row_count, sample_pieces;
    END IF;
END
$$;

-- C) 悬空引用检测：明细引用的匹号在 inventory_piece 无行（回填 no-op 会吞异常）
DO $$
DECLARE
    dangling_count INTEGER;
    sample_pieces TEXT;
BEGIN
    SELECT COUNT(*) INTO dangling_count
      FROM (
        SELECT DISTINCT i."piece_no"
          FROM "outsourcing_order_item" i
          JOIN "outsourcing_order" o ON o."id" = i."outsourcing_order_id"
         WHERE o."status" NOT IN ('draft', 'cancelled')
           AND o."is_deleted" = FALSE
           AND i."piece_no" IS NOT NULL AND i."piece_no" <> ''
           AND NOT EXISTS (
               SELECT 1 FROM "inventory_piece" p WHERE p."piece_no" = i."piece_no"
           )
      ) d;
    IF dangling_count > 0 THEN
        SELECT string_agg("piece_no", ', ' ORDER BY "piece_no")
          INTO sample_pieces
          FROM (
            SELECT DISTINCT i."piece_no"
              FROM "outsourcing_order_item" i
              JOIN "outsourcing_order" o ON o."id" = i."outsourcing_order_id"
             WHERE o."status" NOT IN ('draft', 'cancelled')
               AND o."is_deleted" = FALSE
               AND i."piece_no" IS NOT NULL AND i."piece_no" <> ''
               AND NOT EXISTS (
                   SELECT 1 FROM "inventory_piece" p WHERE p."piece_no" = i."piece_no"
               )
             LIMIT 10
          ) s;
        RAISE EXCEPTION '非 draft/非 cancelled 委外订单明细引用了 % 个在 inventory_piece 不存在的匹号（样例：%）。发料门控「匹不存在」上线前的悬空存量，回填对其静默 0 行等于吞掉账实断裂：请人工补建匹档案或更正明细后重跑。',
            dangling_count, sample_pieces;
    END IF;
END
$$;

-- 回填：占用面（非 draft、非 cancelled、未删除订单的明细匹号）内仍 AVAILABLE 的匹
-- 置 RESERVED。updated_at 与运行时 CAS（piece_domain_service::cas_piece_status）
-- 同口径刷新；updated_by 不写——回填无操作者主体，伪造用户 ID 属制造数据。
UPDATE "inventory_piece" p
   SET "status" = 'RESERVED',
       "updated_at" = now()
 WHERE p."status" = 'AVAILABLE'
   AND EXISTS (
        SELECT 1
          FROM "outsourcing_order_item" i
          JOIN "outsourcing_order" o ON o."id" = i."outsourcing_order_id"
         WHERE i."piece_no" = p."piece_no"
           AND i."piece_no" IS NOT NULL AND i."piece_no" <> ''
           AND o."status" NOT IN ('draft', 'cancelled')
           AND o."is_deleted" = FALSE
   );
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // 回填后无法区分「本次补置的 RESERVED」与「闭环上线后发料事务写入的
        // RESERVED」，反向 UPDATE 会把在用占用一起放开成 AVAILABLE，等价于
        // 人为制造重复发料窗口。回滚显式不做任何写操作（与 m0056/m0057
        // 数据归一迁移同策略），需要回退时按业务人工处置。
        Ok(())
    }
}
