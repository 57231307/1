#!/bin/bash

# Define files to add #[allow(dead_code)]
FILES=(
    "backend/src/services/financial_analysis_service.rs"
    "backend/src/services/metrics_service.rs"
    "backend/src/services/quality_inspection_service.rs"
    "backend/src/services/sales_return_service.rs"
    "backend/src/services/system_update_service.rs"
    "backend/src/services/operation_log_service.rs"
    "backend/src/services/voucher_service.rs"
    "backend/src/utils/app_state.rs"
    "backend/src/utils/cache.rs"
    "backend/src/utils/fabric_five_dimension.rs"
    "backend/src/models/system_update.rs"
    "frontend/src/models/sales_return.rs"
    "frontend/src/models/api_response.rs"
)

for file in "${FILES[@]}"; do
    if [ -f "$file" ]; then
        # Check if #![allow(dead_code)] is already there
        if ! grep -q "#!\[allow(dead_code)\]" "$file"; then
            sed -i '1i #![allow(dead_code)]\n#![allow(unused_variables)]\n#![allow(unused_imports)]' "$file"
        fi
    fi
done

# Frontend specific fixes for unused
sed -i '1i #![allow(dead_code, unused_variables, unused_imports)]' frontend/src/services/department_service.rs
sed -i '1i #![allow(dead_code, unused_variables, unused_imports)]' frontend/src/services/inventory_service.rs
sed -i '1i #![allow(dead_code, unused_variables, unused_imports)]' frontend/src/services/product_service.rs
sed -i '1i #![allow(dead_code, unused_variables, unused_imports)]' frontend/src/services/purchase_order_service.rs
sed -i '1i #![allow(dead_code, unused_variables, unused_imports)]' frontend/src/services/auth_service.rs
sed -i '1i #![allow(dead_code, unused_variables, unused_imports)]' frontend/src/components/mod.rs
sed -i '1i #![allow(dead_code, unused_variables, unused_imports)]' frontend/src/pages/mod.rs

