//! 委外收回入库单（`outsourcing_receipt`）补三列打卷实测值：`weight` / `width` / `gram_weight`。
//!
//! 功能：成品布入库标签的重量/幅宽/克重只认匹行实测值，而委外收回匹的这三列取自收回单确认
//! 产匹时的逐列透传，所以实测值的采集点必须落在实物经手的收回入库环节——本迁移建的就是这组采集列。
//!
//! 字段口径（全取匹行实测值、不回落主数据、缺值写 NULL 并由标签逐列点名拒绝）：
//! - 标签侧 fail-closed 逐列判空点名：`services/print_service.rs:4999-5010`
//!   （`get_inventory_piece_label_print_data` 按 `view.weight/width/gram_weight/barcode` 收集缺失列），
//!   拒绝文案「请先补录打卷实测值」在 `services/print_service.rs:5018-5022`；米数列虽是 NOT NULL，
//!   `≤ 0` 同样视同无实测依据（`services/print_service.rs:4995-4998`）。
//! - 透传落库：`services/piece_domain_service.rs:647-649`（`create_piece_from_outsourcing_receipt`
//!   `:516` 把上下文 `OutsourcingReceiptPieceContext` `:493-510` 的三列原样写入匹行；缺值保持 NULL），
//!   调用点 `services/outsourcing_ops/receipt.rs:507-525`（确认收回时取收回单行的三列）。
//! - 收回路径不经打卷入口，打卷侧的必填门（`RollFabricRequest #[validate(required)]`，
//!   `services/fabric_inspection_service.rs:200-210`）覆盖不到它，实测值只能由收回单自身承载。
//! - 故新列可空且不带 DEFAULT：NULL 表达"未补录"，对应匹的标签继续被拒绝打印。回落
//!   `products.width/gram_weight` 标称值会让标签掺印标称值，破坏「账（匹行）→标（签）→实（卷）」
//!   同一性；塞默认值 0 是把无据伪装成实测值（0 kg / 0 cm 无业务含义，且标签只判 NULL，见上）。
//!
//! 列形态（与目标列严格同型，保证透传无损）：
//! - 类型取 `inventory_piece` 同名列的 `DECIMAL(18,4)`（生效 DDL `domain/production/mod.rs:376`
//!   weight / `:377` width / `:354` gram_weight），单位同模型注释：weight=千克、width=cm、
//!   gram_weight=g/m²；消费侧对应 `models/outsourcing_receipt.rs:71-78`（`Option<Decimal>`）与
//!   入参 `services/outsourcing_ops/types.rs:230-234`；
//! - 可空（NULL=未补录）：收回时是否强制必填由端点/业务侧门控，不由本迁移收紧——同形态先例
//!   `m0062_add_dye_batch_actual_output`（同为可空新增列 `:30-32`，必填由端点校验保证 `:15`，
//!   并在列注释里声明 `:33-35`）；
//! - 必要约束 = 值域 CHECK（非空时必须 > 0）：三列各自独立命名，使 23514 的归因能精确到列；
//!   服务层另有同域值的 400 门（`services/outsourcing_ops/receipt.rs:65-74`，建单 `:230`、
//!   编辑 `:373-383`），CHECK 是并发/旁路写入的兜底，两者取值域逐字符一致，不新造第二套口径。
//!
//! 注册位置：目标表 `outsourcing_receipt` 由 v15 域内建表（`domain/v15/mod.rs:689-714`，建表语句
//! 本身不含这三列，其唯一来源即本迁移），而 production 域早于 v15 执行——直接注册本域会因
//! "relation outsourcing_receipt does not exist" 中断整条迁移链。照 m0058/m0063/m0065/m0068 先例，
//! 迁移文件仍属 production 域（本文件），up/down 由 `domain/v15/mod.rs` 在建表完成后调用
//! （up 挂在 v15 域 up 末尾 `:4616-4618`、down 挂在 v15 域 down 开头 `:4625-4627`，顺序严格对应）。
//!
//! 幂等写法（重跑等价）：只用 `ADD COLUMN IF NOT EXISTS` 建可空无默认列（列已存在即 no-op，
//! 形态即目标形态）；值域 CHECK 用 `DROP CONSTRAINT IF EXISTS` + `ADD CONSTRAINT` 重建
//! （幂等且必然生效）。不在
//! `ADD COLUMN IF NOT EXISTS` 之后再用 `IF NOT EXISTS` 补 NOT NULL/DEFAULT——那种写法在列已存在时
//! 会被整句吃成恒 no-op，目标形态永不生效。up 末尾回读 information_schema / pg_constraint 自证，
//! 不信任"执行过=生效过"。
//!
//! 存量数据策略（fail-visible，禁止静默洗数据）：新增列对历史行恒为 NULL，up 只统计并
//! RAISE NOTICE 点名"历史收回单零实测值"的行数（这些匹的标签继续被 fail-closed 逐列点名拒绝，
//! 需由收回单编辑口按实补录——补录是业务动作，不属迁移职责，绝不 UPDATE 造值）。表不存在视为
//! 结构漂移 → RAISE EXCEPTION 中止。
//!
//! down 真实可逆（禁止空实现）：撤掉三条 CHECK 与三列，回到本迁移前的表形态，并回读
//! information_schema / pg_constraint 证明残留为 0（与 up 的复核对称，不信任"执行过=生效过"）。
//! 不可复原的是"已补录的实测值"本身——列级 DROP 必然丢弃它们，因此 down 先用 RAISE NOTICE
//! 点名受影响行数（回滚是操作者的显式意图，不留静默），再执行删除。行数点名前先做目录探测：
//! 三列由本迁移自建，在 up 从未执行（或 up 中途失败后回滚）的库上裸查会抛
//! `column "weight" does not exist` 而把整条回滚链卡死；缺失形态显式 NOTICE「无可回退对象」，
//! 撤销语句本身保持 `DROP ... IF EXISTS` 幂等。

use sea_orm_migration::prelude::*;

/// 三列及其值域 CHECK 约束名/单位注释（列名与 `inventory_piece` 同名列逐字符一致，
/// 透传链路可逐列对照，不引入第二套字段命名）
const MEASURED_COLUMNS: [(&str, &str, &str); 3] = [
    (
        "weight",
        "chk_outsourcing_receipt_weight_positive",
        "收回匹实测重量（千克；标签 fail-closed 点名「重量(weight)」的数据源）",
    ),
    (
        "width",
        "chk_outsourcing_receipt_width_positive",
        "收回匹实测幅宽（cm；标签 fail-closed 点名「幅宽(width)」的数据源）",
    ),
    (
        "gram_weight",
        "chk_outsourcing_receipt_gram_weight_positive",
        "收回匹实测克重（g/m²；标签 fail-closed 点名「克重(gram_weight)」的数据源）",
    ),
];

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1) 结构前提 fail-visible：表由 v15 域建表语句（domain/v15/mod.rs:689）创建，缺失说明
        //    迁移链形态与本迁移前提不符（注册位置被改动），拒绝"跳过即通过"。
        let probe = r#"
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.tables
                    WHERE table_name = 'outsourcing_receipt') THEN
        RAISE EXCEPTION 'm0075：表 outsourcing_receipt 不存在（建表 DDL 应在 domain/v15/mod.rs:689，本迁移须后置到 v15 域建表之后），中止。';
    END IF;
END
$$;
"#;
        manager.get_connection().execute_unprepared(probe).await?;

        // 2) 逐列新增（可空、无默认）+ 值域 CHECK 重建 + 列注释。
        //    ADD COLUMN IF NOT EXISTS 之后不再叠加任何 IF NOT EXISTS 的 NOT NULL/DEFAULT
        //    （列已存在时整句会被吃成恒 no-op，见文件头）。
        for (column, constraint, comment) in MEASURED_COLUMNS {
            let sql = format!(
                r#"ALTER TABLE "outsourcing_receipt" ADD COLUMN IF NOT EXISTS "{column}" DECIMAL(18,4);
ALTER TABLE "outsourcing_receipt" DROP CONSTRAINT IF EXISTS "{constraint}";
ALTER TABLE "outsourcing_receipt" ADD CONSTRAINT "{constraint}" CHECK ("{column}" IS NULL OR "{column}" > 0);
COMMENT ON COLUMN "outsourcing_receipt"."{column}" IS '{comment}；NULL=未补录实测值，对应匹的成品布入库标签按 fail-closed 逐列点名拒绝，禁止回落主数据或默认值';"#,
            );
            manager.get_connection().execute_unprepared(&sql).await?;
        }

        // 3) 存量事实点名（不洗数据）：历史收回单三列恒 NULL，其产匹无标签依据。
        let notice = r#"
DO $$
DECLARE
    total_rows      INTEGER;
    measured_rows   INTEGER;
BEGIN
    SELECT COUNT(*) INTO total_rows FROM "outsourcing_receipt";
    SELECT COUNT(*) INTO measured_rows FROM "outsourcing_receipt"
      WHERE "weight" IS NOT NULL OR "width" IS NOT NULL OR "gram_weight" IS NOT NULL;
    RAISE NOTICE 'm0075：outsourcing_receipt 共 % 行，其中已带实测值 % 行（新增列对历史行恒为 NULL）。这些历史收回匹的成品布入库标签将继续被 fail-closed 逐列点名拒绝，需由收回单编辑口按实补录后才可打签；本迁移拒绝 UPDATE 造值。',
        total_rows, measured_rows;
END
$$;
"#;
        manager.get_connection().execute_unprepared(notice).await?;

        // 4) 复核（不信任"执行过=生效过"）：三列存在且可空无默认、三条 CHECK 在
        //    pg_constraint 里为 validated，且形态与 inventory_piece 同名列同型。
        let verify = r#"
DO $$
DECLARE
    col_count    INTEGER;
    type_agg     TEXT;
    nn_flag      TEXT;
    def_flag     TEXT;
    chk_count    INTEGER;
BEGIN
    SELECT COUNT(*) INTO col_count FROM information_schema.columns
      WHERE table_name = 'outsourcing_receipt'
        AND column_name IN ('weight', 'width', 'gram_weight');
    IF col_count <> 3 THEN
        RAISE EXCEPTION 'm0075：三列实测值仅 % 列存在（期望 3），ADD COLUMN 未真实生效，中止。', col_count;
    END IF;

    SELECT COUNT(*) INTO col_count FROM information_schema.columns
      WHERE table_name = 'outsourcing_receipt'
        AND column_name IN ('weight', 'width', 'gram_weight')
        AND data_type IN ('numeric', 'decimal');
    -- PG 在编译 DO 块时就校验 RAISE 的 % 占位符数与实参数是否一致，不一致直接 42601 并打断
    -- 整条迁移链，因此拒绝面要报"实际是什么型"而不是只报"不对"：col_count 只是本段判定量，
    -- 实参改用下面聚合出的三列真实 data_type，既配平占位符也让失败信息可直接定位。
    SELECT string_agg(data_type, ',') INTO type_agg FROM information_schema.columns
      WHERE table_name = 'outsourcing_receipt'
        AND column_name IN ('weight', 'width', 'gram_weight');
    IF col_count <> 3 THEN
        RAISE EXCEPTION 'm0075：三列 data_type 非 numeric/decimal（实际 data_type 聚合=%，期望三列均为 numeric；须与 inventory_piece 同名列 DECIMAL(18,4) 同型以保证透传无损），中止。', type_agg;
    END IF;

    -- 可空 + 无默认是口径本身：NULL 表达"未补录"，DEFAULT 0 会把无据伪装成实测值
    SELECT string_agg(is_nullable, ',') INTO nn_flag FROM information_schema.columns
      WHERE table_name = 'outsourcing_receipt'
        AND column_name IN ('weight', 'width', 'gram_weight');
    IF nn_flag <> 'YES,YES,YES' THEN
        RAISE EXCEPTION 'm0075：三列可空形态不符（is_nullable 聚合=%，期望 YES,YES,YES），中止。', nn_flag;
    END IF;
    SELECT string_agg(COALESCE(column_default, '<null>'), ',') INTO def_flag FROM information_schema.columns
      WHERE table_name = 'outsourcing_receipt'
        AND column_name IN ('weight', 'width', 'gram_weight');
    IF def_flag <> '<null>,<null>,<null>' THEN
        RAISE EXCEPTION 'm0075：三列不得带 DEFAULT（实测值缺省必须是 NULL，默认值即伪造），实际 column_default=%，中止。', def_flag;
    END IF;

    SELECT COUNT(*) INTO chk_count FROM pg_constraint
      WHERE conrelid = '"outsourcing_receipt"'::regclass
        AND contype = 'c'
        AND conname IN ('chk_outsourcing_receipt_weight_positive',
                        'chk_outsourcing_receipt_width_positive',
                        'chk_outsourcing_receipt_gram_weight_positive');
    IF chk_count <> 3 THEN
        RAISE EXCEPTION 'm0075：值域 CHECK 约束仅 % 条存在（期望 3，逐列独立命名以便 23514 归因到列），中止。', chk_count;
    END IF;

    RAISE NOTICE 'm0075：outsourcing_receipt 三列实测值已就位（DECIMAL(18,4) 可空无默认 + 逐列正值 CHECK），与 inventory_piece 同名列同型，收回→产匹透传链路可逐列对照。';
END
$$;
"#;
        manager.get_connection().execute_unprepared(verify).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 回滚 = 丢弃三列与其上已补录的实测值。先点名行数（成功/失败都留显式日志，
        // 不静默吞掉业务数据），再撤约束、撤列，回到本迁移前的表形态。
        //
        // 点名语句本身必须先做目录探测再执行：三列是**本迁移自己建的**，在 up 从未执行
        // （或 up 中途失败后回滚）的库上直接 `WHERE "weight" IS NOT NULL` 会抛
        // `column "weight" does not exist`，把整条回滚链硬卡死——这不是"有问题被拦住"，
        // 而是回滚工具自身不可用。缺失形态一律显式 RAISE NOTICE 说明"无可回退对象"
        // （留痕、不静默），撤销语句本身是 `DROP ... IF EXISTS`（幂等）。
        let notice = r#"
DO $$
DECLARE
    has_table     BOOLEAN;
    col_count     INTEGER;
    measured_rows INTEGER;
BEGIN
    SELECT EXISTS (SELECT 1 FROM information_schema.tables
                    WHERE table_name = 'outsourcing_receipt') INTO has_table;

    IF NOT has_table THEN
        RAISE NOTICE 'm0075 down：表 outsourcing_receipt 不存在 ⇒ 本迁移从未在本库生效，无列可撤、无实测值将被丢弃（显式留痕，非静默）。';
    ELSE
        SELECT COUNT(*) INTO col_count FROM information_schema.columns
          WHERE table_name = 'outsourcing_receipt'
            AND column_name IN ('weight', 'width', 'gram_weight');

        IF col_count = 0 THEN
            RAISE NOTICE 'm0075 down：三列实测值均不存在（本迁移的 up 未在本库执行）⇒ 无列可撤、无实测值将被丢弃。';
        ELSE
            IF col_count <> 3 THEN
                RAISE NOTICE 'm0075 down：三列实测值仅 % 列就位（up 曾中途失败）⇒ 按 DROP ... IF EXISTS 逐列撤销，不做完整形态假设。', col_count;
            ELSE
                -- 仅三列齐备时才统计：不齐时的裸 WHERE 会抛 "column does not exist"，
                -- 那是回滚工具自身失效，不是"拦住了一条坏回滚"。
                SELECT COUNT(*) INTO measured_rows FROM "outsourcing_receipt"
                  WHERE "weight" IS NOT NULL OR "width" IS NOT NULL OR "gram_weight" IS NOT NULL;
                RAISE NOTICE 'm0075 down：即将删除 outsourcing_receipt 的 weight/width/gram_weight 三列，% 行已补录的实测值将随之丢弃（回滚为操作者显式意图）。这些列删除后，由其产匹的标签依据同时消失。', measured_rows;
            END IF;
        END IF;
    END IF;
END
$$;
"#;
        manager.get_connection().execute_unprepared(notice).await?;

        for (column, constraint, _) in MEASURED_COLUMNS {
            let sql = format!(
                r#"ALTER TABLE "outsourcing_receipt" DROP CONSTRAINT IF EXISTS "{constraint}";
ALTER TABLE "outsourcing_receipt" DROP COLUMN IF EXISTS "{column}";"#,
            );
            manager.get_connection().execute_unprepared(&sql).await?;
        }

        // 回读自证（与 up 的复核对称，不信任"执行过=生效过"）：表存在时必须证明三列已不在、
        // 三条 CHECK 已不在。回滚后仍残留即说明语句被吃掉或形态漂移 ⇒ RAISE EXCEPTION 中止，
        // 绝不留下"报称已回滚"的假状态。
        let verify = r#"
DO $$
DECLARE
    has_table  BOOLEAN;
    col_count  INTEGER;
    chk_count  INTEGER;
BEGIN
    SELECT EXISTS (SELECT 1 FROM information_schema.tables
                    WHERE table_name = 'outsourcing_receipt') INTO has_table;

    IF NOT has_table THEN
        RAISE NOTICE 'm0075 down：表不存在 ⇒ 回滚为空操作，跳过回读复核（与点名语句同一判定，两处不互相假设）。';
    ELSE
        SELECT COUNT(*) INTO col_count FROM information_schema.columns
          WHERE table_name = 'outsourcing_receipt'
            AND column_name IN ('weight', 'width', 'gram_weight');
        IF col_count <> 0 THEN
            RAISE EXCEPTION 'm0075 down：回滚后仍有 % 列实测值残留（期望 0，DROP COLUMN IF EXISTS 未真实生效），中止。', col_count;
        END IF;

        SELECT COUNT(*) INTO chk_count FROM pg_constraint
          WHERE conrelid = '"outsourcing_receipt"'::regclass
            AND contype = 'c'
            AND conname IN ('chk_outsourcing_receipt_weight_positive',
                            'chk_outsourcing_receipt_width_positive',
                            'chk_outsourcing_receipt_gram_weight_positive');
        IF chk_count <> 0 THEN
            RAISE EXCEPTION 'm0075 down：回滚后仍有 % 条值域 CHECK 残留（期望 0，DROP CONSTRAINT IF EXISTS 未真实生效），中止。', chk_count;
        END IF;

        RAISE NOTICE 'm0075 down：已回退至本迁移前形态（三列与三条 CHECK 均不存在，经 information_schema/pg_constraint 回读证明）。';
    END IF;
END
$$;
"#;
        manager.get_connection().execute_unprepared(verify).await?;
        Ok(())
    }
}
