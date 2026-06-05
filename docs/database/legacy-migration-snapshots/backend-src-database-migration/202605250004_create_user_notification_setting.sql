-- Create user_notification_setting table
CREATE TABLE IF NOT EXISTS user_notification_setting (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    email_enabled BOOLEAN NOT NULL DEFAULT true,
    internal_enabled BOOLEAN NOT NULL DEFAULT true,
    order_notification_type VARCHAR(20) NOT NULL DEFAULT 'both',
    approval_notification_type VARCHAR(20) NOT NULL DEFAULT 'both',
    inventory_notification_type VARCHAR(20) NOT NULL DEFAULT 'internal',
    purchase_notification_type VARCHAR(20) NOT NULL DEFAULT 'both',
    finance_notification_type VARCHAR(20) NOT NULL DEFAULT 'both',
    system_notification_type VARCHAR(20) NOT NULL DEFAULT 'internal',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create index
CREATE INDEX IF NOT EXISTS idx_user_notification_setting_user ON user_notification_setting(user_id);
