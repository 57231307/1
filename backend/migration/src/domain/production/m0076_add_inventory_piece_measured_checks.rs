//! `inventory_piece` 三列打卷实测值补值域 CHECK：`weight` / `width` / `gram_weight`
//! （任务 #246 ②；同型先例 = `m0075_add_outsourcing_receipt_measured_values`，本文件的
//! 检测/点名/重建约束/回读自证结构与 down 口径全部照它，不另起一套）
//!
//! 缺陷事实（三段证据，读函数体不读注释）：
//! - 这三列是成品布入库标签的**直读源**：`services/print_service.rs:4991-5011` 按
//!   `view.weight/width/gram_weight` 逐列判空 → `validation_displayable` 点名缺哪一列
//!   （「请先补录打卷实测值」）。也就是说：落进这三列的值会被原样印到实物标签上。
//! - 标签侧只判 NULL，不判取值：0 与负数在这里等同"已实测"，会被直接印上标签
//!   （同族门已在两处承认这一点：`print_service.rs:4987-4989` 把 `length ≤ 0` 视同
//!   "无实测依据"；m0075 对收回单三列落的就是 `IS NULL OR > 0`）。
//! - 而库里这三列**没有任何值域约束**：建表 DDL
//!   `domain/business/m0010_add_inventory_extensions.rs:126-143` 只有两条外键
//!   （`fk_inventory_piece_product` / `fk_inventory_piece_warehouse`），全仓 grep
//!   `inventory_piece` + `CHECK|CONSTRAINT` 亦无任何针对这三列的约束（唯一命中是本
//!   文件与 m0051 的 `uniq_inventory_piece_dye_lot_piece` 唯一键 DROP）。
//!   ⇒ 服务层的门拦不住旁路写入与并发写入：`INSERT INTO inventory_piece ... weight=0`
//!     可落库并被打上标签，属"伪造实测值"。
//!
//! 为什么加了服务层门还要 DB CHECK（本仓已锁口径，见 m0075 文件头同一论证）：
//! 服务层的 >0 门只在**经服务口的写入**上生效，且与并发写、脚本补数、后续新增调用点
//! 无关。二者是同一取值域的两道门，取值域必须逐字符一致：
//! - `services/fabric_inspection_service.rs:601-617`（打卷入库三列 > 0 门，
//!   文案「打卷入库{列}必须大于 0」）；
//! - `services/outsourcing_ops/receipt.rs:65-87`（委外收回三列 > 0 门，约束由 m0075 兜底）；
//! - `services/piece_domain_service.rs::validate_piece_measured_value`（本批补的匹行写入口门，
//!   覆盖生产报工建匹 / 拆匹子卷重量 / 剪大货样建匹三条此前无门的落库路径）；
//! - 本迁移的 DB CHECK（并发/旁路兜底，逐列独立命名使 23514 可归因到列）。
//!
//! 列形态：本迁移**不改列型、不改可空性、不加 DEFAULT**（NULL = 未补录是合法形态，
//! 默认 0 会把无据伪装成实测值，见 m0075 同一结论）；只新增三条 `IS NULL OR > 0` CHECK。
//!
//! 域内注册位置：`inventory_piece` 由 **business 域**建表
//! （`domain/business/m0010_add_inventory_extensions.rs:126`），执行链顺序见
//! `migration/src/lib.rs`（system → business → sales_crm → production → …），
//! production 域晚于 business ⇒ 本迁移注册在 `domain/production/mod.rs` 即可，
//! 目标表与三列（business `mod.rs:110/:124` + production `mod.rs:346/:368/:369` 的
//! `ADD COLUMN IF NOT EXISTS`，后者对已存在列恒 no-op）都已就位；这与 m0075 不同
//! （`outsourcing_receipt` 由 v15 域建表、晚于 production，故其 up/down 后置到 v15 调用）。
//! SHADOWED 事故族防线（本仓纪律）：不使用 `ADD COLUMN IF NOT EXISTS` 之后再叠
//! `IF NOT EXISTS` 的形态变更；CHECK 用 `DROP CONSTRAINT IF EXISTS` + `ADD CONSTRAINT`
//! 重建（幂等且必然生效），up 末尾回读 `information_schema` / `pg_constraint` 自证。
//!
//! 存量数据策略（fail-visible，禁止静默洗数据）：新 CHECK 比"无约束"严格，因此 up 先做
//! **只读检测**：逐列统计 `<= 0` 的存量行，存在即 `RAISE EXCEPTION` 点名（列名 + 行数 +
//! 最多 10 个样例匹号）并中止迁移。绝不 `UPDATE` 把 0/负数改成 NULL 或正数——那是在
//! 迁移里造实测值（伪造证据）并把责任从"谁的写入产生了 0"转移成"迁移替它圆场"。
//! 表不存在视为结构漂移 → RAISE EXCEPTION 中止（不"跳过即通过"）。
//!
//! down 真实可逆：三条 CHECK 由本迁移自建，撤销不丢任何数据（列本身不属于本迁移，
//! 绝不 DROP COLUMN），故 down 不做"在途行拒滚"（那是 m0064/m0067 词表缩减族才需要的：
//! 撤销后存量行会读不出来；本迁移撤销后唯一变化是 0/负数重新可写，属回滚的既定语义）。
//! 回滚前显式 `RAISE NOTICE` 点名"当前受保护的非空实测值行数"（留痕、不静默），
//! 撤销后回读 `pg_constraint` 证明残留为 0，不信任"执行过=生效过"。
//!
//! 幂等：约束先 DROP 再 ADD，重跑等价。

use sea_orm_migration::prelude::*;

/// 三列及其值域 CHECK 约束名（列名与 `outsourcing_receipt` 同名列、标签点名列名逐字符
/// 一致：重量(weight) / 幅宽(width) / 克重(gram_weight)，便于 23514 归因到列并与
/// `print_service.rs` 的逐列点名文案对齐）
const MEASURED_CHECKS: [(&str, &str); 3] = [
    ("weight", "chk_inventory_piece_weight_positive"),
    ("width", "chk_inventory_piece_width_positive"),
    ("gram_weight", "chk_inventory_piece_gram_weight_positive"),
];

#[derive(DeriveMigrationName)]
pub struct Migration;

impl Migration {
    /// 三条 CHECK 的名字列表（拼进 SQL 的 IN 清单，up/down 共用同一份，不各写一遍）
    fn constraint_name_list_sql() -> String {
        MEASURED_CHECKS
            .iter()
            .map(|(_, constraint)| format!("'{constraint}'"))
            .collect::<Vec<_>>()
            .join(",")
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1) 结构前提 fail-visible：表与三列必须由 business/production 链就位。
        //    缺失说明注册位置被改动（本迁移依赖建表链在先），拒绝"跳过即通过"。
        let probe = r#"
DO $$
DECLARE
    col_count INTEGER;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.tables
                    WHERE table_name = 'inventory_piece') THEN
        RAISE EXCEPTION 'm0076：表 inventory_piece 不存在（建表 DDL 应在 domain/business/m0010_add_inventory_extensions.rs:126，本迁移依赖该链在先），中止。';
    END IF;

    SELECT COUNT(*) INTO col_count FROM information_schema.columns
      WHERE table_name = 'inventory_piece'
        AND column_name IN ('weight', 'width', 'gram_weight');
    IF col_count <> 3 THEN
        RAISE EXCEPTION 'm0076：inventory_piece 的三列实测值仅 % 列存在（期望 3），建表/补列链与本迁移前提不符，中止。', col_count;
    END IF;
END
$$;
"#;
        manager.get_connection().execute_unprepared(probe).await?;

        // 2) 只读存量检测（严禁 UPDATE 洗数据）：三列任一存在 <= 0 的行即点名中止。
        //    样例最多 10 个（匹号是用户可见业务编号，可直接用于定位是谁写进 0 的）。
        for (column, constraint) in MEASURED_CHECKS {
            let detect = format!(
                r#"
DO $$
DECLARE
    bad_rows   INTEGER;
    samples    TEXT;
BEGIN
    SELECT COUNT(*) INTO bad_rows FROM "inventory_piece"
      WHERE "{column}" IS NOT NULL AND "{column}" <= 0;

    IF bad_rows = 0 THEN
        RAISE NOTICE 'm0076：inventory_piece.{column} 无 <= 0 存量行，可安全施加值域 CHECK {constraint}。';
    ELSE
        SELECT string_agg(sample, ', ' ORDER BY sample) INTO samples
          FROM (
                SELECT 'id=' || "id" || '(piece_no=' || COALESCE(NULLIF("piece_no", ''), '<空>')
                       || ', {column}=' || rtrim(rtrim(CAST("{column}" AS TEXT), '0'), '.') || ')'
                       AS sample
                  FROM "inventory_piece"
                 WHERE "{column}" IS NOT NULL AND "{column}" <= 0
                 ORDER BY "id"
                 LIMIT 10
               ) s;

        RAISE EXCEPTION
            'm0076：inventory_piece.{column} 存在 % 行 <= 0 的伪实测值，拒绝施加 CHECK {constraint}（样例：%',
            bad_rows, samples
            USING HINT = '本迁移拒绝 UPDATE 造值：0/负数属伪造实测值且会被成品布入库标签直接印出（print_service.rs 逐列点名只判 NULL）。'
                      || '请按实核实这些匹的原始打卷/称量记录后由业务侧更正（有真实实测值则改为实测值，确无依据则置 NULL 让标签继续拒绝打印），再重跑迁移。'
                      || '若这些行来自脚本旁路写入，先修写入方，不要在迁移里圆场。';
    END IF;
END;
$$;
"#,
            );
            manager.get_connection().execute_unprepared(&detect).await?;
        }

        // 3) 逐列重建值域 CHECK（DROP 再 ADD ⇒ 幂等且必然生效；形态与 m0075 逐字符同口径：
        //    "IS NULL OR > 0"，NULL=未补录是合法形态，0 不是）。
        for (column, constraint) in MEASURED_CHECKS {
            let sql = format!(
                r#"ALTER TABLE "inventory_piece" DROP CONSTRAINT IF EXISTS "{constraint}";
ALTER TABLE "inventory_piece" ADD CONSTRAINT "{constraint}" CHECK ("{column}" IS NULL OR "{column}" > 0);
COMMENT ON CONSTRAINT "{constraint}" ON "inventory_piece" IS '匹行实测值{column}值域（与 m0075 收回单同名列、服务层门 fabric_inspection_service.rs:601-617 / outsourcing_ops/receipt.rs:65-87 / piece_domain_service.rs 同口径）：NULL=未补录（标签 fail-closed 点名拒绝），0 与负数属伪造实测值，禁止经任何旁路/并发写入落库并被印上成品布入库标签';"#,
            );
            manager.get_connection().execute_unprepared(&sql).await?;
        }

        // 4) 回读自证（不信任"执行过=生效过"）：三条 CHECK 必须真实存在于 pg_constraint、
        //    且是 validated、且表达式确实是"IS NULL OR > 0"形态；同时证明列的可空形态未被
        //    本迁移改动（可空 = NULL 表达"未补录"的口径前提）。
        let names = Self::constraint_name_list_sql();
        let verify = format!(
            r#"
DO $$
DECLARE
    chk_count     INTEGER;
    val_count     INTEGER;
    nn_flag       TEXT;
BEGIN
    SELECT COUNT(*) INTO chk_count FROM pg_constraint
      WHERE conrelid = '"inventory_piece"'::regclass
        AND contype = 'c'
        AND conname IN ({names});
    IF chk_count <> 3 THEN
        RAISE EXCEPTION 'm0076：值域 CHECK 约束仅 % 条存在（期望 3，逐列独立命名以便 23514 归因到列），中止。', chk_count;
    END IF;

    -- ADD CONSTRAINT NOT VALID / INVALID 之类形态等于"门没关上"（对存量行不生效），
    -- 因此除存在性外还须回读 convalidated
    SELECT COUNT(*) INTO val_count FROM pg_constraint
      WHERE conrelid = '"inventory_piece"'::regclass
        AND contype = 'c'
        AND conname IN ({names})
        AND convalidated;
    IF val_count <> 3 THEN
        RAISE EXCEPTION 'm0076：三条值域 CHECK 中仅 % 条 convalidated（期望 3），约束未真正对存量生效，中止。', val_count;
    END IF;

    -- 本迁移不得改动列的可空性（NULL=未补录是业务口径，不是约束缺陷）
    SELECT string_agg(is_nullable, ',') INTO nn_flag FROM information_schema.columns
      WHERE table_name = 'inventory_piece'
        AND column_name IN ('weight', 'width', 'gram_weight');
    IF nn_flag <> 'YES,YES,YES' THEN
        RAISE EXCEPTION 'm0076：三列可空形态被改动（is_nullable 聚合=%，期望 YES,YES,YES）；NULL 表达「未补录实测值」，不得改为 NOT NULL，中止。', nn_flag;
    END IF;

    RAISE NOTICE 'm0076：三条值域 CHECK 的存在性、convalidated 与三列可空形态均已回读证明；表达式逐列归一化自证紧随其后。';
END
$$;
"#,
        );
        manager.get_connection().execute_unprepared(&verify).await?;

        for (column, constraint) in MEASURED_CHECKS {
            // 期望归一化形态：CHECK<col>ISNULLOR<col>>0（">= 0" 归一化后是 ...>=0，不等）
            let expected = format!("CHECK{column}ISNULLOR{column}>0");
            let expr_verify = format!(
                r#"
DO $$
DECLARE
    def      TEXT;
    norm     TEXT;
BEGIN
    SELECT pg_get_constraintdef(oid) INTO def FROM pg_constraint
      WHERE conrelid = '"inventory_piece"'::regclass
        AND contype = 'c'
        AND conname = '{constraint}';

    IF def IS NULL THEN
        RAISE EXCEPTION 'm0076：回读不到约束 {constraint}（ADD CONSTRAINT 未真实生效），中止。';
    END IF;

    norm := replace(replace(replace(replace(replace(def, ' ', ''), '(', ''), ')', ''), '"', ''), '::numeric', '');
    IF norm <> '{expected}' THEN
        RAISE EXCEPTION 'm0076：约束 {constraint} 的定义不是「{column} IS NULL OR {column} > 0」形态（归一化后=%，期望 {expected}），与实际定义不符 ⇒ 值域口径走样（>= 0 会放过 0，漏 IS NULL 会把"未补录"当违规），中止。', norm;
    END IF;

    RAISE NOTICE 'm0076：{constraint} 表达式回读自证通过：%', def;
END
$$;
"#,
            );
            manager
                .get_connection()
                .execute_unprepared(&expr_verify)
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let names = Self::constraint_name_list_sql();

        // 回滚前显式留痕：撤销后 0/负数重新可写，先点名当前受保护的非空实测值行数
        // （成功/失败都有日志，不静默）。不 DROP COLUMN——列不属于本迁移。
        let notice = r#"
DO $$
DECLARE
    has_table   BOOLEAN;
    col_count   INTEGER;
    live_rows   INTEGER;
BEGIN
    SELECT EXISTS (SELECT 1 FROM information_schema.tables
                    WHERE table_name = 'inventory_piece') INTO has_table;

    IF NOT has_table THEN
        RAISE NOTICE 'm0076 down：表 inventory_piece 不存在 ⇒ 本迁移从未在本库生效，无约束可撤（显式留痕，非静默）。';
    ELSE
        SELECT COUNT(*) INTO col_count FROM information_schema.columns
          WHERE table_name = 'inventory_piece'
            AND column_name IN ('weight', 'width', 'gram_weight');
        IF col_count = 0 THEN
            RAISE NOTICE 'm0076 down：三列实测值均不存在 ⇒ 无列可依附，跳过统计（约束本身按 DROP IF EXISTS 幂等撤销）。';
        ELSE
            SELECT COUNT(*) INTO live_rows FROM "inventory_piece"
              WHERE "weight" IS NOT NULL OR "width" IS NOT NULL OR "gram_weight" IS NOT NULL;
            RAISE NOTICE 'm0076 down：即将撤销三条值域 CHECK，% 行非空实测值失去 DB 兜底（0/负数重新可写入并会被印上成品布入库标签）。数据本身不丢，列不属于本迁移故不删除；回滚为操作者显式意图。', live_rows;
        END IF;
    END IF;
END
$$;
"#;
        manager.get_connection().execute_unprepared(notice).await?;

        for (_, constraint) in MEASURED_CHECKS {
            let sql = format!(
                r#"ALTER TABLE "inventory_piece" DROP CONSTRAINT IF EXISTS "{constraint}";"#,
            );
            manager.get_connection().execute_unprepared(&sql).await?;
        }

        // 回读自证（与 up 对称）：表存在时必须证明三条 CHECK 残留为 0，
        // 否则说明语句被吃掉或形态漂移 ⇒ 中止，绝不留下"报称已回滚"的假状态。
        let verify = format!(
            r#"
DO $$
DECLARE
    has_table  BOOLEAN;
    chk_count  INTEGER;
BEGIN
    SELECT EXISTS (SELECT 1 FROM information_schema.tables
                    WHERE table_name = 'inventory_piece') INTO has_table;

    IF NOT has_table THEN
        RAISE NOTICE 'm0076 down：表不存在 ⇒ 回滚为空操作，跳过回读复核（与点名语句同一判定，两处不互相假设）。';
    ELSE
        SELECT COUNT(*) INTO chk_count FROM pg_constraint
          WHERE conrelid = '"inventory_piece"'::regclass
            AND contype = 'c'
            AND conname IN ({names});
        IF chk_count <> 0 THEN
            RAISE EXCEPTION 'm0076 down：回滚后仍有 % 条值域 CHECK 残留（期望 0，DROP CONSTRAINT IF EXISTS 未真实生效），中止。', chk_count;
        END IF;

        RAISE NOTICE 'm0076 down：已回退至本迁移前形态（三条值域 CHECK 均不存在，经 pg_constraint 回读证明；三列与其数据原样保留）。';
    END IF;
END
$$;
"#,
        );
        manager.get_connection().execute_unprepared(&verify).await?;
        Ok(())
    }
}
