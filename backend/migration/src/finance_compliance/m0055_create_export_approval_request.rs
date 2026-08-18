use sea_orm_migration::prelude::*;

// Batch 473 P0-S14 修复：敏感数据导出二级审批表
//
// 背景：Batch 461 PR #643 已实现 service/model/handler 3 层完整逻辑，
// 但 migration m0047 被占用于 webhooks（last_payload 字段），
// export_approval_request 表的 migration 完全缺失，导致运行时表不存在。
// 复审重新打开此任务，本批次补齐缺失的 migration。
//
// 表结构：按 backend/src/models/export_approval_request.rs Model 定义（29 字段）
// 索引：6 个（status / applicant_user_id / approver_user_id / resource_type / download_token / risk_level）
//
// 设计依据：V15 审计报告 类十三 P0-S14
// 关联文件：models/export_approval_request.rs / services/export_approval_service.rs / handlers/export_approval_handler.rs

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE IF NOT EXISTS export_approval_request (
                    id BIGSERIAL PRIMARY KEY,
                    -- 申请人用户 ID
                    applicant_user_id INTEGER NOT NULL,
                    -- 申请人用户名
                    applicant_username VARCHAR(100) NOT NULL,
                    -- 审批人用户 ID（二级审批时填充）
                    approver_user_id INTEGER,
                    -- 审批人用户名
                    approver_username VARCHAR(100),
                    -- 导出资源类型：customer/supplier/dye_recipe/price_list/finance_report 等
                    resource_type VARCHAR(100) NOT NULL,
                    -- 导出参数 JSON（过滤条件/字段选择等）
                    export_params JSONB,
                    -- 预估导出行数
                    estimated_rows BIGINT,
                    -- 文件格式：xlsx/pdf/csv
                    file_format VARCHAR(20) NOT NULL,
                    -- 审批状态：pending/approved/rejected/expired/cancelled
                    status VARCHAR(20) NOT NULL,
                    -- 当前审批层级：1=一级，2=二级
                    approval_level INTEGER NOT NULL,
                    -- 审批人备注
                    approver_comments TEXT,
                    -- 审批通过时间
                    approved_at TIMESTAMPTZ,
                    -- 审批拒绝时间
                    rejected_at TIMESTAMPTZ,
                    -- 临时下载令牌（审批通过后生成，5 分钟有效，防重放攻击）
                    download_token VARCHAR(100),
                    -- token 过期时间（approved_at + 5min）
                    token_expires_at TIMESTAMPTZ,
                    -- 已下载次数
                    download_count INTEGER NOT NULL DEFAULT 0,
                    -- 最大下载次数（默认 1，防重放攻击）
                    max_downloads INTEGER NOT NULL DEFAULT 1,
                    -- 导出文件临时存储路径
                    file_path VARCHAR(500),
                    -- 文件大小（字节）
                    file_size_bytes BIGINT,
                    -- 文件 SHA256 校验值
                    file_checksum VARCHAR(64),
                    -- 申请人 IP
                    applicant_ip VARCHAR(45),
                    -- 审批人 IP
                    approver_ip VARCHAR(45),
                    -- 申请人 User-Agent
                    applicant_user_agent VARCHAR(500),
                    -- 风险等级：low/medium/high/critical
                    risk_level VARCHAR(20) NOT NULL,
                    -- 审批上下文（JSON）
                    context JSONB,
                    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    -- 流程终结时间（下载完成或 token 过期）
                    completed_at TIMESTAMPTZ
                );

                -- 索引：按状态查询待审批/已通过列表
                CREATE INDEX idx_ear_status ON export_approval_request (status);
                -- 索引：按申请人查询（我的申请列表）
                CREATE INDEX idx_ear_applicant ON export_approval_request (applicant_user_id);
                -- 索引：按审批人查询（待我审批列表）
                CREATE INDEX idx_ear_approver ON export_approval_request (approver_user_id);
                -- 索引：按资源类型查询（资源审批历史）
                CREATE INDEX idx_ear_resource_type ON export_approval_request (resource_type);
                -- 索引：按 download_token 查询（下载校验高频查询）
                CREATE UNIQUE INDEX idx_ear_download_token ON export_approval_request (download_token) WHERE download_token IS NOT NULL;
                -- 索引：按风险等级查询（高风险导出监控）
                CREATE INDEX idx_ear_risk_level ON export_approval_request (risk_level);
                "#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(r#"DROP TABLE IF EXISTS export_approval_request;"#)
            .await?;
        Ok(())
    }
}
