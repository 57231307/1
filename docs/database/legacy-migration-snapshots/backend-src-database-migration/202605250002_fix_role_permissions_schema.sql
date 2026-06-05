-- Fix role_permissions table schema to match Rust model
-- The model expects: resource_type, resource_id, action, allowed, updated_at
-- But the database has: permission_code, permission_name

-- Step 1: Drop the old unique constraint
ALTER TABLE role_permissions DROP CONSTRAINT IF EXISTS role_permissions_role_id_permission_code_key;

-- Step 2: Drop old columns
ALTER TABLE role_permissions DROP COLUMN IF EXISTS permission_code;
ALTER TABLE role_permissions DROP COLUMN IF EXISTS permission_name;

-- Step 3: Add new columns
ALTER TABLE role_permissions ADD COLUMN IF NOT EXISTS resource_type VARCHAR(50) NOT NULL DEFAULT 'module';
ALTER TABLE role_permissions ADD COLUMN IF NOT EXISTS resource_id INTEGER;
ALTER TABLE role_permissions ADD COLUMN IF NOT EXISTS action VARCHAR(50) NOT NULL DEFAULT 'view';
ALTER TABLE role_permissions ADD COLUMN IF NOT EXISTS allowed BOOLEAN NOT NULL DEFAULT true;
ALTER TABLE role_permissions ADD COLUMN IF NOT EXISTS updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP;

-- Step 4: Create new unique constraint
ALTER TABLE role_permissions ADD CONSTRAINT role_permissions_role_resource_action_unique 
  UNIQUE (role_id, resource_type, action, resource_id);

-- Step 5: Create index for faster lookups
CREATE INDEX IF NOT EXISTS idx_role_permissions_resource ON role_permissions(resource_type, action);
