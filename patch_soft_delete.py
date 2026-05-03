import os
import re
import glob

# We want to replace:
# ModelEntity::delete_by_id(id).exec(&txn).await
# With:
# ModelEntity::update_many().col_expr(ModelColumn::IsDeleted, Expr::value(true)).filter(ModelColumn::Id.eq(id)).exec(&txn).await

# This is highly pattern dependent.
# Let's search for `.delete` in backend/src/services/*.rs

services_dir = 'backend/src/services/'
files = glob.glob(os.path.join(services_dir, '*.rs'))

for filepath in files:
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    if 'delete_by_id' in content:
        # Replace:
        # XxxEntity::delete_by_id(id)
        # With:
        # XxxEntity::update_many().col_expr(xxx::Column::IsDeleted, sea_orm::sea_query::Expr::value(true)).filter(xxx::Column::Id.eq(id))
        
        # It's tricky because `xxx::Column` needs the correct module prefix.
        pass

print("Soft delete patch script ready.")
