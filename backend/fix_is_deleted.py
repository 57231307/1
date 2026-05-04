import re

with open('database/migration/001_consolidated_schema.sql', 'r', encoding='utf-8') as f:
    content = f.read()

# Add is_deleted to warehouses
content = content.replace(
    "status VARCHAR(20) NOT NULL DEFAULT 'active',\n    created_at TIMESTAMP",
    "status VARCHAR(20) NOT NULL DEFAULT 'active',\n    is_deleted BOOLEAN NOT NULL DEFAULT false,\n    created_at TIMESTAMP"
)

# Add is_deleted to inventory_stocks
content = content.replace(
    "status VARCHAR(20) NOT NULL DEFAULT 'active',\n    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,",
    "status VARCHAR(20) NOT NULL DEFAULT 'active',\n    is_deleted BOOLEAN NOT NULL DEFAULT false,\n    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,"
)

# Add is_deleted to products
content = content.replace(
    "status VARCHAR(20) NOT NULL DEFAULT 'active',\n    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,\n    updated_at TIMESTAMP",
    "status VARCHAR(20) NOT NULL DEFAULT 'active',\n    is_deleted BOOLEAN NOT NULL DEFAULT false,\n    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,\n    updated_at TIMESTAMP"
)

with open('database/migration/001_consolidated_schema.sql', 'w', encoding='utf-8') as f:
    f.write(content)
