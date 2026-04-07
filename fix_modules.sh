#!/bin/bash

MODULES=(
    "backend/src/services/financial_analysis_service.rs"
    "backend/src/services/metrics_service.rs"
    "backend/src/services/quality_inspection_service.rs"
    "backend/src/services/sales_return_service.rs"
    "backend/src/services/system_update_service.rs"
    "backend/src/services/operation_log_service.rs"
    "backend/src/services/voucher_service.rs"
    "backend/src/utils/fabric_five_dimension.rs"
    "backend/src/models/system_update.rs"
    "backend/src/utils/cache.rs"
    "frontend/src/models/sales_return.rs"
    "frontend/src/models/api_response.rs"
    "frontend/src/services/mod.rs"
    "frontend/src/components/mod.rs"
    "frontend/src/pages/mod.rs"
    "frontend/src/services/auth_service.rs"
    "frontend/src/services/department_service.rs"
    "frontend/src/services/inventory_service.rs"
    "frontend/src/services/product_service.rs"
    "frontend/src/services/purchase_order_service.rs"
    "backend/src/handlers/purchase_order_handler.rs"
    "backend/src/handlers/purchase_receipt_handler.rs"
)

for file in "${MODULES[@]}"; do
    if [ -f "$file" ]; then
        if ! grep -q "#!\[allow(dead_code" "$file"; then
            sed -i '1i #![allow(dead_code, unused_variables, unused_imports, unused_mut)]' "$file"
        fi
    fi
done
