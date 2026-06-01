-- 修复 role_permissions 表结构
-- 代码期望的列：resource_type, resource_id, action, allowed
-- 数据库实际的列：permission_code, permission_name

-- 添加缺失的列
DO $$
BEGIN
    -- 添加 resource_type 列
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'role_permissions' AND column_name = 'resource_type') THEN
        ALTER TABLE role_permissions ADD COLUMN resource_type VARCHAR(100);
        -- 从 permission_code 迁移数据
        UPDATE role_permissions SET resource_type = SPLIT_PART(permission_code, '.', 1) WHERE resource_type IS NULL;
        ALTER TABLE role_permissions ALTER COLUMN resource_type SET NOT NULL;
    END IF;

    -- 添加 action 列
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'role_permissions' AND column_name = 'action') THEN
        ALTER TABLE role_permissions ADD COLUMN action VARCHAR(50);
        -- 从 permission_code 迁移数据
        UPDATE role_permissions SET action = SPLIT_PART(permission_code, '.', 2) WHERE action IS NULL;
        ALTER TABLE role_permissions ALTER COLUMN action SET NOT NULL;
    END IF;

    -- 添加 resource_id 列
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'role_permissions' AND column_name = 'resource_id') THEN
        ALTER TABLE role_permissions ADD COLUMN resource_id INTEGER;
    END IF;

    -- 添加 allowed 列
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'role_permissions' AND column_name = 'allowed') THEN
        ALTER TABLE role_permissions ADD COLUMN allowed BOOLEAN NOT NULL DEFAULT true;
        COMMENT ON COLUMN role_permissions.allowed IS '是否允许';
    END IF;

    -- 添加 updated_at 列
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'role_permissions' AND column_name = 'updated_at') THEN
        ALTER TABLE role_permissions ADD COLUMN updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP;
    END IF;
END $$;

-- 添加索引
CREATE INDEX IF NOT EXISTS idx_role_permissions_resource ON role_permissions(resource_type, action);

COMMENT ON TABLE role_permissions IS '角色权限关联表 - 已修复列结构';
