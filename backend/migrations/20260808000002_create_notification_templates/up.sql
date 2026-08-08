-- batch-16 P2-3：创建通知模板表

CREATE TABLE IF NOT EXISTS notification_templates (
    id SERIAL PRIMARY KEY,
    code VARCHAR(100) NOT NULL UNIQUE,
    name VARCHAR(200) NOT NULL,
    template_type VARCHAR(20) NOT NULL DEFAULT 'system',
    title_template TEXT NOT NULL,
    content_template TEXT NOT NULL,
    language VARCHAR(10) NOT NULL DEFAULT 'zh-CN',
    is_active BOOLEAN NOT NULL DEFAULT true,
    remarks TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 索引
CREATE INDEX idx_notification_templates_code ON notification_templates(code);
CREATE INDEX idx_notification_templates_type ON notification_templates(template_type);
CREATE INDEX idx_notification_templates_language ON notification_templates(language);

-- 插入默认模板
INSERT INTO notification_templates (code, name, template_type, title_template, content_template, language) VALUES
('ORDER_CREATED', '订单创建通知', 'system', '新订单已创建', '订单 {{order_no}} 已创建，金额 {{amount}} 元', 'zh-CN'),
('ORDER_APPROVED', '订单审批通过', 'system', '订单已审批', '订单 {{order_no}} 已通过审批', 'zh-CN'),
('STOCK_LOW', '库存不足预警', 'system', '库存不足', '产品 {{product_name}} 库存不足，当前库存 {{current_stock}}', 'zh-CN'),
('DELIVERY_REMIND', '发货提醒', 'system', '发货提醒', '订单 {{order_no}} 需在 {{delivery_date}} 前发货', 'zh-CN');
