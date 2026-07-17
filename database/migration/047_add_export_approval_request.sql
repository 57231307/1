-- Migration 047: V15 P0-S14 敏感数据导出二级审批机制
-- 创建 export_approval_request 表，用于敏感数据导出（财务报表/客户清单/染色配方/价格清单）的二级审批
-- 审批通过后生成临时下载 token（5 分钟有效），凭 token 下载导出文件
-- 设计依据：V15 审计报告 类十三 P0-S14

CREATE TABLE IF NOT EXISTS "export_approval_request" (
    "id" BIGSERIAL PRIMARY KEY,
    -- 申请人信息
    "applicant_user_id" INTEGER NOT NULL,
    "applicant_username" VARCHAR(100) NOT NULL,
    -- 审批人信息（二级审批时填充）
    "approver_user_id" INTEGER,
    "approver_username" VARCHAR(100),
    -- 导出资源信息
    "resource_type" VARCHAR(50) NOT NULL,          -- customer/supplier/dye_recipe/price_list/finance_report 等
    "export_params" JSONB,                        -- 导出参数（过滤条件/字段选择等）
    "estimated_rows" BIGINT,                     -- 预估导出行数
    "file_format" VARCHAR(20) DEFAULT 'xlsx',    -- xlsx/pdf/csv
    -- 审批状态
    "status" VARCHAR(20) NOT NULL DEFAULT 'pending', -- pending/approved/rejected/expired/cancelled
    "approval_level" INTEGER DEFAULT 1,           -- 当前审批层级（1=一级, 2=二级）
    "approver_comments" TEXT,
    "approved_at" TIMESTAMPTZ,
    "rejected_at" TIMESTAMPTZ,
    -- 临时下载 token（审批通过后生成，5 分钟有效）
    "download_token" VARCHAR(255),                -- 随机生成的下载令牌
    "token_expires_at" TIMESTAMPTZ,               -- token 过期时间（approved_at + 5min）
    "download_count" INTEGER DEFAULT 0,           -- 已下载次数
    "max_downloads" INTEGER DEFAULT 1,            -- 最大下载次数（防重放）
    -- 文件存储信息（审批通过后生成导出文件时填充）
    "file_path" VARCHAR(500),                    -- 导出文件临时存储路径
    "file_size_bytes" BIGINT,                     -- 文件大小
    "file_checksum" VARCHAR(64),                  -- 文件 SHA256 校验值
    -- 审计信息
    "applicant_ip" VARCHAR(50),
    "approver_ip" VARCHAR(50),
    "applicant_user_agent" TEXT,
    "risk_level" VARCHAR(20) DEFAULT 'medium',   -- low/medium/high/critical
    "context" JSONB,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "completed_at" TIMESTAMPTZ                    -- 流程终结时间（下载完成或 token 过期）
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_export_approval_applicant_user_id ON "export_approval_request"("applicant_user_id");
CREATE INDEX IF NOT EXISTS idx_export_approval_approver_user_id ON "export_approval_request"("approver_user_id");
CREATE INDEX IF NOT EXISTS idx_export_approval_status ON "export_approval_request"("status");
CREATE INDEX IF NOT EXISTS idx_export_approval_resource_type ON "export_approval_request"("resource_type");
CREATE INDEX IF NOT EXISTS idx_export_approval_download_token ON "export_approval_request"("download_token");
CREATE INDEX IF NOT EXISTS idx_export_approval_token_expires_at ON "export_approval_request"("token_expires_at");
CREATE INDEX IF NOT EXISTS idx_export_approval_created_at ON "export_approval_request"("created_at");

-- 表注释
COMMENT ON TABLE "export_approval_request" IS '敏感数据导出二级审批请求表（V15 P0-S14）';
COMMENT ON COLUMN "export_approval_request.applicant_user_id" IS '申请人用户 ID';
COMMENT ON COLUMN "export_approval_request.approver_user_id" IS '审批人用户 ID（二级审批时填充）';
COMMENT ON COLUMN "export_approval_request.resource_type" IS '导出资源类型：customer/supplier/dye_recipe/price_list/finance_report 等';
COMMENT ON COLUMN "export_approval_request.status" IS '审批状态：pending/approved/rejected/expired/cancelled';
COMMENT ON COLUMN "export_approval_request.approval_level" IS '当前审批层级：1=一级（直接上级），2=二级（部门经理或更高）';
COMMENT ON COLUMN "export_approval_request.download_token" IS '审批通过后生成的临时下载令牌（5 分钟有效）';
COMMENT ON COLUMN "export_approval_request.token_expires_at" IS '下载令牌过期时间（approved_at + 5min）';
COMMENT ON COLUMN "export_approval_request.max_downloads" IS '最大下载次数（默认 1，防重放攻击）';
COMMENT ON COLUMN "export_approval_request.file_checksum" IS '导出文件 SHA256 校验值（验证文件完整性）';
COMMENT ON COLUMN "export_approval_request.risk_level" IS '风险等级：low/medium/high/critical（基于导出行数和敏感度评估）';
