-- 修复缺失的数据库表和列名不匹配问题

-- 1. 创建 email_templates 表
CREATE TABLE IF NOT EXISTS email_templates (
    id SERIAL PRIMARY KEY,
    tenant_id INTEGER NOT NULL DEFAULT 1,
    name VARCHAR(200) NOT NULL,
    code VARCHAR(100) NOT NULL UNIQUE,
    subject_template VARCHAR(500) NOT NULL,
    body_template TEXT NOT NULL,
    template_type VARCHAR(50) NOT NULL DEFAULT 'NOTIFICATION',
    variables JSONB,
    description TEXT,
    is_active BOOLEAN NOT NULL DEFAULT true,
    status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE',
    created_by INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);
COMMENT ON TABLE email_templates IS '邮件模板表';

-- 2. 创建 email_logs 表
CREATE TABLE IF NOT EXISTS email_logs (
    id SERIAL PRIMARY KEY,
    tenant_id INTEGER NOT NULL DEFAULT 1,
    user_id INTEGER,
    recipients TEXT NOT NULL,
    cc TEXT,
    bcc TEXT,
    subject VARCHAR(500) NOT NULL,
    body TEXT,
    template_id INTEGER,
    status VARCHAR(20) NOT NULL DEFAULT 'PENDING',
    error_message TEXT,
    external_message_id VARCHAR(200),
    sent_at TIMESTAMP WITH TIME ZONE,
    retry_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);
COMMENT ON TABLE email_logs IS '邮件发送记录表';

-- 3. 创建 tenant_subscriptions 表
CREATE TABLE IF NOT EXISTS tenant_subscriptions (
    id SERIAL PRIMARY KEY,
    tenant_id INTEGER NOT NULL,
    plan_id INTEGER NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE',
    billing_cycle VARCHAR(20) NOT NULL DEFAULT 'MONTHLY',
    start_date TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    end_date TIMESTAMP WITH TIME ZONE,
    auto_renew BOOLEAN NOT NULL DEFAULT true,
    current_price DECIMAL(12,2) NOT NULL DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);
COMMENT ON TABLE tenant_subscriptions IS '租户订阅表';

-- 4. 创建 report_subscriptions 表（如果不存在）
CREATE TABLE IF NOT EXISTS report_subscriptions (
    id SERIAL PRIMARY KEY,
    tenant_id INTEGER NOT NULL DEFAULT 1,
    name VARCHAR(200) NOT NULL,
    template_id INTEGER NOT NULL,
    frequency VARCHAR(20) NOT NULL DEFAULT 'DAILY',
    recipients JSONB NOT NULL DEFAULT '[]',
    export_format VARCHAR(20) NOT NULL DEFAULT 'pdf',
    is_enabled BOOLEAN NOT NULL DEFAULT true,
    status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE',
    next_run_at TIMESTAMP WITH TIME ZONE,
    last_run_at TIMESTAMP WITH TIME ZONE,
    last_run_status VARCHAR(20),
    last_run_error TEXT,
    run_count INTEGER NOT NULL DEFAULT 0,
    created_by INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);
COMMENT ON TABLE report_subscriptions IS '报表订阅表';

-- 5. 修复 log_login 表列名（如果需要）
DO $$ 
BEGIN
    -- 将 login_status 改为 status
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'log_login' AND column_name = 'login_status') THEN
        ALTER TABLE log_login RENAME COLUMN login_status TO status;
    END IF;
    
    -- 将 failure_reason 改为 fail_reason
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'log_login' AND column_name = 'failure_reason') THEN
        ALTER TABLE log_login RENAME COLUMN failure_reason TO fail_reason;
    END IF;
END $$;

-- 6. 创建索引
CREATE INDEX IF NOT EXISTS idx_email_templates_tenant ON email_templates(tenant_id);
CREATE INDEX IF NOT EXISTS idx_email_templates_code ON email_templates(code);
CREATE INDEX IF NOT EXISTS idx_email_logs_tenant ON email_logs(tenant_id);
CREATE INDEX IF NOT EXISTS idx_email_logs_status ON email_logs(status);
CREATE INDEX IF NOT EXISTS idx_email_logs_sent_at ON email_logs(sent_at);
CREATE INDEX IF NOT EXISTS idx_tenant_subscriptions_tenant ON tenant_subscriptions(tenant_id);
CREATE INDEX IF NOT EXISTS idx_report_subscriptions_tenant ON report_subscriptions(tenant_id);
CREATE INDEX IF NOT EXISTS idx_report_subscriptions_template ON report_subscriptions(template_id);

-- 7. 插入默认数据
INSERT INTO email_templates (tenant_id, name, code, subject_template, body_template, template_type, description, is_active, status, created_by)
VALUES 
    (1, '系统通知', 'SYSTEM_NOTIFICATION', '系统通知: {{title}}', '<h1>{{title}}</h1><p>{{content}}</p>', 'NOTIFICATION', '系统默认通知模板', true, 'ACTIVE', 1),
    (1, '审批通知', 'APPROVAL_NOTIFICATION', '审批通知: {{title}}', '<h1>{{title}}</h1><p>您有一个新的审批任务</p>', 'WORKFLOW', '审批流程通知模板', true, 'ACTIVE', 1),
    (1, '库存预警', 'STOCK_ALERT', '库存预警: {{product_name}}', '<h1>库存预警</h1><p>产品 {{product_name}} 库存不足</p>', 'ALERT', '库存预警通知模板', true, 'ACTIVE', 1)
ON CONFLICT (code) DO NOTHING;
