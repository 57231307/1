-- Create field_permissions table
CREATE TABLE IF NOT EXISTS field_permissions (
    id SERIAL PRIMARY KEY,
    role_id INTEGER NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    resource_type VARCHAR(50) NOT NULL,
    field_name VARCHAR(100) NOT NULL,
    can_read BOOLEAN NOT NULL DEFAULT true,
    can_write BOOLEAN NOT NULL DEFAULT true,
    mask_strategy VARCHAR(20) NOT NULL DEFAULT 'NONE',
    is_enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(role_id, resource_type, field_name)
);

-- Create indexes
CREATE INDEX IF NOT EXISTS idx_field_permissions_role ON field_permissions(role_id);
CREATE INDEX IF NOT EXISTS idx_field_permissions_resource ON field_permissions(resource_type);
