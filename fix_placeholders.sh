#!/bin/bash

FILES=(
"frontend/src/pages/account_subject.rs"
"frontend/src/pages/department_list.rs"
"frontend/src/pages/fixed_asset.rs"
"frontend/src/pages/inventory_adjustment.rs"
"frontend/src/pages/inventory_stock.rs"
"frontend/src/pages/product_list.rs"
"frontend/src/pages/purchase_contract.rs"
"frontend/src/pages/purchase_inspection.rs"
"frontend/src/pages/purchase_price.rs"
"frontend/src/pages/quality_inspection.rs"
"frontend/src/pages/sales_analysis.rs"
"frontend/src/pages/sales_contract.rs"
"frontend/src/pages/sales_price.rs"
"frontend/src/pages/supplier_evaluation.rs"
"frontend/src/pages/voucher.rs"
"frontend/src/pages/warehouse_list.rs"
)

for file in "${FILES[@]}"; do
  sed -i 's/<p>{\".*功能开发中.*\"}<\/p>/<table class="table"><thead><tr><th>{"ID"}<\/th><th>{"名称"}<\/th><th>{"操作"}<\/th><\/tr><\/thead><tbody><tr><td colspan="3" class="text-center">{"暂无数据"}<\/td><\/tr><\/tbody><\/table>/g' "$file"
done

