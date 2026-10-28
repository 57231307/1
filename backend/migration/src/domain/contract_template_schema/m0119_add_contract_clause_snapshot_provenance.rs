//! 合同条款快照来源指针补列迁移
//!
//! 功能 给 `contract_clauses` 追加一列 `template_clause_id`，记录固化当时引用的
//!   `contract_template_clauses` 行主键。合同详情返回条款数组时要带“必填标记”，而必填标记长在
//!   模板条款项行上；快照不存来源行指针就无法在条款码存在多适用条件变体时确定当时固化的是哪一行，
//!   只能靠 (模板, 条款码) 猜测，猜出来的必填标记是伪造值。本列把“当时签的是哪一版哪一条”
//!   的回答从“版加码可能多义”收紧为“版加码加行指针单义”。
//! 调用方 SeaORM migrator 链，经 `domain/contract_template_schema` 域注册，域内排在补
//!   两侧合同主表列之后执行，只依赖 m0117 建的快照表。
//! 入参 无，纯 DDL 迁移。
//! 传给谁 后端签约固化腿写入本列（新行必写），合同详情腿与打印腿按本列回查条款项取必填标记；
//!   历史快照行为空值，读侧对空值走 (模板, 条款码) 唯一行回落，多义时如实返回空值不伪造。
//! 存什么 一个可空整数列与列注释。不建外键：本表与模板条款项之间的引用是快照溯源指针，
//!   建外键等于让删除腿能顺着条款项反向阻塞快照行，与切片 A 对该表“不建外键不建 Relation”的
//!   建模决定一致。不建索引：读侧按快照自己的 (contract_kind, contract_id) 索引取行后，
//!   条款项主键点查即可，无按本列过滤的需求。
//! 存哪里 PostgreSQL `public.contract_clauses.template_clause_id`。
//!
//! 形态口径
//! - 新增列一律可空且无默认：已固化行没有可信的历史指针可回填，禁止用 UPDATE 造值圆场，
//!   回填伪造指针比空值更坏（假溯源比无溯源更难识破）。
//! - up 与 down 互逆：加一列撤一列，不触碰任何其他列与约束与数据行。
//! - 探测不适用：纯加可空无默认列不改变任何既有行的可读形态，无需先探测存量。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
ALTER TABLE "contract_clauses" ADD COLUMN IF NOT EXISTS "template_clause_id" INTEGER;
COMMENT ON COLUMN "contract_clauses"."template_clause_id" IS '固化当时引用的模板条款项行主键，历史快照行为空值，读侧按指针单义回查必填标记';
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
ALTER TABLE "contract_clauses" DROP COLUMN IF EXISTS "template_clause_id";
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
