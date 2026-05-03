import os
import re
import glob

services_dir = 'backend/src/services/'
files = glob.glob(os.path.join(services_dir, '*.rs'))

for filepath in files:
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # We messed up `use sea_orm::{ColumnTrait, QueryFilter, ` 
    # Let's revert that specific mistake.
    
    content = content.replace('use sea_orm::{ColumnTrait, QueryFilter, \n', 'use sea_orm::{')
    content = content.replace('use sea_orm::{ColumnTrait, QueryFilter, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};', 'use sea_orm::{DatabaseConnection, EntityTrait};')
    content = content.replace('use sea_orm::{ColumnTrait, QueryFilter, DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait', 'use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait')
    content = content.replace('use sea_orm::{ColumnTrait, QueryFilter, EntityTrait, Set, ActiveModelTrait, DbErr, Order, PaginatorTrait};', 'use sea_orm::{EntityTrait, Set, ActiveModelTrait, DbErr, Order, PaginatorTrait};')
    content = content.replace('use sea_orm::{ColumnTrait, QueryFilter, DatabaseConnection, EntityTrait, ActiveModelTrait, Set};', 'use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait, Set};')
    content = content.replace('use sea_orm::{QueryFilter, \n', 'use sea_orm::{')
    
    # Just to be safe, any `ColumnTrait, ` or `QueryFilter, ` right after `use sea_orm::{`
    content = re.sub(r'use sea_orm::\{\s*(ColumnTrait|QueryFilter),\s*', 'use sea_orm::{', content)
    content = re.sub(r'use sea_orm::\{\s*(ColumnTrait|QueryFilter),\s*', 'use sea_orm::{', content)
    content = re.sub(r'use sea_orm::\{\s*(ColumnTrait|QueryFilter),\s*', 'use sea_orm::{', content)
    content = re.sub(r'use sea_orm::\{\s*(ColumnTrait|QueryFilter),\s*', 'use sea_orm::{', content)

    # Now, if it needs QueryFilter and ColumnTrait, let's just put `use sea_orm::{QueryFilter, ColumnTrait};` at the top
    if 'Column::IsDeleted' in content:
        # Check if it already has `use sea_orm::QueryFilter;` at the top
        if 'use sea_orm::{QueryFilter, ColumnTrait};' not in content:
            content = "use sea_orm::{QueryFilter, ColumnTrait};\n" + content
    
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)
