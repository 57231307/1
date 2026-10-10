//! RLS dept 扩展:`users.department_id` 运行期变更时,同事务重算 5 张 RLS 表的冗余
//! `department_id`(m_rls_dept_domain 的缺口补齐)。
//!
//! ## 要修的缺口(实证,非推测)
//! - m_rls_dept_domain 只锚定"归属列变更"(`rls_dept/mod.rs:111-114` 触发时机
//!   `BEFORE INSERT OR UPDATE OF owner_id`),而归属人所在的 `users.department_id`
//!   可在运行期被改(`src/services/user_service.rs:364`),`users` 表上没有任何触发器
//!   (全库 `CREATE TRIGGER` 无 `ON users` 命中)。后果:换部门后其名下 5 表行的
//!   `department_id` 滞留旧部门 → 旧部门经理越界可见、新部门该看看不到
//!   (行级判定:`src/utils/data_scope.rs:141`、`:149-171`、`:229-236`,
//!   RLS 策略 `department_id = ANY(app_dept_ids())`,`rls_dept/mod.rs:152`)。
//! - 本迁移补 `trg_users_dept_sync`(`AFTER UPDATE OF department_id ON users`),
//!   对 5 表各一条批量 UPDATE(同事务,不允许应用层逐行查询),并做一次存量回填。
//!
//! ## 发布性判定(为何新增而非改写 rls_dept)
//! `backend/migration/src/domain/rls_dept/` 已进入 main(25ae95e1 落地,8b15f468 为其
//! 收口笔,8b15f468 = 本分支与 main 的 merge-base)。已发布迁移**不可改写**,只做新增。
//!
//! ## 勘误(如实记录,不改写已发布文件)
//! `rls_dept/mod.rs:232-237` 的 `down()` 注释声称"回滚:恢复 finance 域原策略",
//! 但函数体是空实现 `Ok(())`——既不会恢复策略,也不会撤销列/触发器:**注释与代码不符,
//! 且 rls_dept 自身不可回滚(仅前进)**。本迁移不重写该文件(已发布),纠正方式:
//! 1) 以本注释固定事实,后续读到 rls_dept 的人以此为准;2) 生产回滚 rls_dept 只能走
//! "前向修复"(重新执行 finance 域策略段或手动 DROP/CREATE POLICY),不得依赖其 down()。
//! 本迁移自身的 `down()` 为真实可逆实现(见下)。
//!
//! ## 设计
//! - **可回退性取舍**:回填选 m0007(`domain/system/m0007_normalize_account_subject_
//!   balance_direction.rs`)的备份表精确回退路线,而非"回填单向不可逆"声明。理由:
//!   本仓迁移红线要求 down 真可执行;备份仅覆盖"将被改写的行"(WHERE 与回填同谓词),
//!   down 逐行还原后删除备份表,回滚粒度精确且不触碰数据本身。
//! - **幂等/可重入**:备份表 CREATE IF NOT EXISTS;备份 INSERT 带 NOT EXISTS 守卫;
//!   回填 UPDATE 带 `IS DISTINCT FROM` 谓词(第二次起 0 行);函数 CREATE OR REPLACE;
//!   触发器先 DROP IF EXISTS 再 CREATE。整支迁移可安全重放。
//! - **触发时机 AFTER**(论证见 sql.rs::SYNC_TRIGGER_SQL 注释):与 5 表既有 BEFORE
//!   触发器(反查 users)在同事务内读到同一部门值,无覆写窗口。
//! - **性能**:回填与触发器重算均为 UPDATE ... FROM / WHERE 归属列=NEW.id 的单条批量;
//!   顺带补 sales_orders.created_by 缺失的索引(sqlite 无此顾虑,PG 上是顺序扫描源)。
//!
//! ## 已知口径差(只报事实,不在本迁移拍板)
//! 本触发器与 rls_dept 的 5 表触发器同源,均只取 `users.department_id`(**主部门**单值,
//! `rls_dept/mod.rs:78-85`);而应用层可见部门集合是"主 + 兼职 + 子部门"
//! (`data_scope.rs:61-69`,`dept_ids` 多值)。行只挂一个部门列 vs 可见集多部门,两口径
//! 何时统一属产品/架构决策,已列入交付报告"待人工决策",本批不擅改。
//!
//! SQL 常量在 `sql.rs`(唯一来源,活体测试经 `#[path]` 直引,防两处漂移)。
//!
//! 部署顺序:必须排在 `m_rls_dept_domain` 之后(依赖其 5 表冗余列与 5 表触发器),
//! 置于 `m_crm_vocab_check` 之前(本迁移不触碰状态列,与 CHECK 约束无交互)。

use sea_orm_migration::prelude::*;

pub mod sql;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_rls_dept_user_sync"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        // 1. 备份表 + 备份将被改写的存量行(必须在回填前,否则原值丢失不可回退)
        conn.execute_unprepared(sql::CREATE_BACKUP_TABLE_SQL)
            .await?;
        conn.execute_unprepared(sql::BACKUP_BEFORE_ALIGN_SQL)
            .await?;
        // 2. 存量回填:5 表对齐当前 users.department_id(单条 UPDATE...FROM,幂等)
        conn.execute_unprepared(sql::BACKFILL_ALIGN_SQL).await?;
        // 3. 触发器重算所需的索引(sales_orders.created_by 原缺失)
        conn.execute_unprepared(sql::CREATED_BY_INDEX_SQL).await?;
        // 4. 函数 + 触发器:此后 users 换部门即时重算 5 表冗余列
        conn.execute_unprepared(sql::SYNC_FUNCTION_SQL).await?;
        conn.execute_unprepared(sql::SYNC_TRIGGER_SQL).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        // 回滚顺序与 up 对称:先摘触发器/函数(还原期间不得再重算),
        // 再按备份逐行还原回填前原值,最后弃备份表。
        conn.execute_unprepared(sql::DROP_TRIGGER_SQL).await?;
        conn.execute_unprepared(sql::DROP_FUNCTION_SQL).await?;
        conn.execute_unprepared(sql::RESTORE_BACKUP_SQL).await?;
        conn.execute_unprepared(sql::DROP_BACKUP_TABLE_SQL).await?;
        Ok(())
    }
}
