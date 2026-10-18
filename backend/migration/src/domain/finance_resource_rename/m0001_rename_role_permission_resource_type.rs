//! 财务域权限资源键改名波（乙波 finance）的数据迁移。
//!
//! ## 背景（RBAC 按 URL 段推导权限键，非风格问题）
//! 运行时资源键由 `middleware/permission.rs::extract_resource_info` 从 `/api/v1/erp/{seg3}/...`
//! 派生，授权行仅当 `role_permissions.resource_type` 与派生键逐字符相同才命中
//! （`matches_permission` 精确串匹配）。本波把 ap/ar/finance 三域下的同名 URL 段随域改名
//! （`invoices` → `ap-invoices`/`ar-invoices`/`finance-invoices` 等），改名映射的唯一真相源是
//! `utils/path_utils.rs::RESOURCE_NAME_MAP`；注册表、代码消歧、本迁移三处必须同批落地，
//! 否则旧授权行与派生键漂移、非超管当场 403 且旧行成死码。
//!
//! ## 语义与幂等
//! - `resource_type` 是无 CHECK 的 VARCHAR(100)，但受唯一索引
//!   `uq_role_permissions_role_resource_action`（role_id+resource_type+action）约束：改名前
//!   必须预检「同 (role_id, action) 是否已存在目标行」，命中即 RAISE EXCEPTION 中止，
//!   绝不静默覆盖或忽略。
//! - 每一对逐 `UPDATE` 后用 `GET DIAGNOSTICS ROW_COUNT` 打印影响行数（含 0 行）到 NOTICE，
//!   不静默：0 行也是显式可核对的结果。
//! - UPDATE 幂等可重放（二次执行源值已不存在 ⇒ 0 行）。
//!
//! ## 一对一改名对（旧值可唯一还原为新值）
//! `reconciliations`/`reconciliation`(单数并入复数) → `ap-reconciliations`、
//! `payment-requests` → `ap-payment-requests`、`collections` → `ar-collections`、
//! `audit` → `finance-audit`、`balance-sheet`/`cash-flow`/`general-ledger`/`income-statement`/
//! `subsidiary-ledger`/`trial-balance`/`drill-down` → 各自加 `finance-` 前缀。
//!
//! ## 一对多旧段（单值无法判定归属域，本迁移不臆造改名）
//! `invoices`/`payments`/`verifications`/`aging`/`daily`/`monthly`/`statistics` 按挂载 seg3
//! 分别落到 ap-/ar-/finance- 前缀，`role_permissions.resource_type` 这个裸值不携带 seg3，
//! 无法逐行还原归属；且这些段此前从未登记进 `PERMISSION_RESOURCES`（目录勾不到 ⇒ 任何角色
//! 都授予不了 ⇒ 不可能有落库授权行）。故本迁移对其设**失败可见闸门**：一旦检出存量行即
//! RAISE EXCEPTION 点名行数并中止，交由人工按域裁定，不静默丢弃、不擅自选一个目标域。
//!
//! ## down（可独立回退）
//! 仅对上方一对一改名逐对反向 UPDATE（新值 → 原 URL 段名）。一对多段在 up 中未改名，
//! 故 down 无需处理；闸门检出的异常态在 up 阶段即中止，不会进入 down。

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m0001_rename_role_permission_resource_type"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();

        // 闸门：一对多旧段若真有存量授权行，无法从裸 resource_type 判定归属域，中止待裁。
        conn.execute_unprepared(
            r#"DO $$
DECLARE
    v_cnt INTEGER;
BEGIN
    SELECT count(*) INTO v_cnt FROM role_permissions
        WHERE resource_type IN
            ('invoices', 'payments', 'verifications', 'aging', 'daily', 'monthly', 'statistics');
    IF v_cnt > 0 THEN
        RAISE EXCEPTION '存量 role_permissions 含无法按裸值判定归属域的一对多旧段共 % 行（invoices/payments/verifications/aging/daily/monthly/statistics），单值无法还原 ap-/ar-/finance- 前缀，中止迁移等待人工按域裁定', v_cnt;
    END IF;
END $$;"#,
        )
        .await?;

        // 一对一改名：逐对预检唯一索引冲突后 UPDATE，并打印影响行数（含 0 行）。
        conn.execute_unprepared(
            r#"DO $$
DECLARE
    r RECORD;
    v_conflict INTEGER;
    v_rows INTEGER;
BEGIN
    FOR r IN
        SELECT * FROM (VALUES
            ('reconciliation',     'ap-reconciliations'),
            ('reconciliations',    'ap-reconciliations'),
            ('payment-requests',   'ap-payment-requests'),
            ('collections',        'ar-collections'),
            ('audit',              'finance-audit'),
            ('balance-sheet',      'finance-balance-sheet'),
            ('cash-flow',          'finance-cash-flow'),
            ('general-ledger',     'finance-general-ledger'),
            ('income-statement',   'finance-income-statement'),
            ('subsidiary-ledger',  'finance-subsidiary-ledger'),
            ('trial-balance',      'finance-trial-balance'),
            ('drill-down',         'finance-drill-down')
        ) AS m(old_type, new_type)
    LOOP
        SELECT count(*) INTO v_conflict FROM role_permissions t
            WHERE t.resource_type = r.new_type
              AND EXISTS (
                  SELECT 1 FROM role_permissions s
                      WHERE s.resource_type = r.old_type
                        AND s.role_id = t.role_id
                        AND s.action = t.action
              );
        IF v_conflict > 0 THEN
            RAISE EXCEPTION '改名冲突：源段 % 存在与目标段 % 同 (role_id, action) 的授权行共 % 条，命中唯一索引 uq_role_permissions_role_resource_action，中止不覆盖', r.old_type, r.new_type, v_conflict;
        END IF;
        UPDATE role_permissions SET resource_type = r.new_type WHERE resource_type = r.old_type;
        GET DIAGNOSTICS v_rows = ROW_COUNT;
        RAISE NOTICE '财务域改名 % -> % 影响行数 %', r.old_type, r.new_type, v_rows;
    END LOOP;
END $$;"#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();

        // 反向逐对：目标名 → 原 URL 段名。ap-reconciliations 的打印面源单复数并存，回退统一还原为复数
        // reconciliations（单数 /ap/reconciliation 端点本波已并入复数挂载，回退到复数与旧词表一致）。
        conn.execute_unprepared(
            r#"DO $$
DECLARE
    r RECORD;
    v_rows INTEGER;
BEGIN
    FOR r IN
        SELECT * FROM (VALUES
            ('ap-reconciliations',       'reconciliations'),
            ('ap-payment-requests',      'payment-requests'),
            ('ar-collections',           'collections'),
            ('finance-audit',            'audit'),
            ('finance-balance-sheet',    'balance-sheet'),
            ('finance-cash-flow',        'cash-flow'),
            ('finance-general-ledger',   'general-ledger'),
            ('finance-income-statement', 'income-statement'),
            ('finance-subsidiary-ledger','subsidiary-ledger'),
            ('finance-trial-balance',    'trial-balance'),
            ('finance-drill-down',       'drill-down')
        ) AS m(old_type, new_type)
    LOOP
        UPDATE role_permissions SET resource_type = r.new_type WHERE resource_type = r.old_type;
        GET DIAGNOSTICS v_rows = ROW_COUNT;
        RAISE NOTICE '财务域改名回退 % -> % 影响行数 %', r.old_type, r.new_type, v_rows;
    END LOOP;
END $$;"#,
        )
        .await?;

        Ok(())
    }
}
