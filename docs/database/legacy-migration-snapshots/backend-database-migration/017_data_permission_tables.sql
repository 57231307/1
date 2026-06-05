-- ============================================
-- 迁移 017: 数据权限细化表结构
-- ============================================

-- 数据权限规则表
CREATE TABLE IF NOT EXISTS data_permissions (
    id SERIAL PRIMARY KEY,
    role_id INTEGER NOT NULL,
    resource_type VARCHAR(50) NOT NULL,
    scope_type VARCHAR(20) NOT NULL DEFAULT 'ALL',
    custom_condition JSONB,
    allowed_fields JSONB,
    hidden_fields JSONB,
    is_enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_data_permissions_role FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE,
    CONSTRAINT uk_data_permissions_role_resource UNIQUE (role_id, resource_type)
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_data_permissions_role_id ON data_permissions(role_id);
CREATE INDEX IF NOT EXISTS idx_data_permissions_resource_type ON data_permissions(resource_type);
CREATE INDEX IF NOT EXISTS idx_data_permissions_enabled ON data_permissions(is_enabled);

-- 添加注释
COMMENT ON TABLE data_permissions IS '数据权限规则表，定义角色的数据访问范围和字段权限';
COMMENT ON COLUMN data_permissions.scope_type IS '数据范围类型：ALL-全部, DEPT-本部门, DEPT_AND_BELOW-本部门及以下, SELF-仅本人, CUSTOM-自定义';
COMMENT ON COLUMN data_permissions.custom_condition IS '自定义数据范围条件，JSON格式';
COMMENT ON COLUMN data_permissions.allowed_fields IS '允许的字段列表，JSON数组';
COMMENT ON COLUMN data_permissions.hidden_fields IS '隐藏的字段列表，JSON数组';
