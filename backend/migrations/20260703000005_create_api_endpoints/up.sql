-- API 端点管理表（批次 91 P0-1）
-- 管理 API 网关暴露的端点元数据，支持 CRUD 操作
CREATE TABLE IF NOT EXISTS "api_endpoints" (
    "id" SERIAL PRIMARY KEY,
    "path" VARCHAR(255) NOT NULL,
    "method" VARCHAR(10) NOT NULL,
    "description" VARCHAR(500),
    "module" VARCHAR(100),
    "status" VARCHAR(20) NOT NULL DEFAULT 'active',
    "rate_limit" INTEGER NOT NULL DEFAULT 0,
    "timeout" INTEGER NOT NULL DEFAULT 30000,
    "authentication" BOOLEAN NOT NULL DEFAULT TRUE,
    "authorization" JSONB,
    "request_schema" JSONB,
    "response_schema" JSONB,
    "version" VARCHAR(20) DEFAULT 'v1',
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 按路径+方法唯一索引，防止重复注册同一端点
CREATE UNIQUE INDEX IF NOT EXISTS "uk_api_endpoints_path_method"
    ON "api_endpoints"("path", "method");

-- 按模块索引，便于按模块分组查询
CREATE INDEX IF NOT EXISTS "idx_api_endpoints_module" ON "api_endpoints"("module");

-- 按状态索引，便于筛选启用的端点
CREATE INDEX IF NOT EXISTS "idx_api_endpoints_status" ON "api_endpoints"("status");
