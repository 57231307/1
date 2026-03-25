# 批量修复 order_by_desc 问题
$files = @(
    "src\services\purchase_price_service.rs",
    "src\services\supplier_evaluation_service.rs",
    "src\services\purchase_receipt_service.rs",
    "src\services\purchase_order_service.rs",
    "src\services\purchase_return_service.rs",
    "src\services\purchase_inspection_service.rs",
    "src\services\ap_payment_service.rs",
    "src\services\ap_reconciliation_service.rs",
    "src\services\ap_verification_service.rs",
    "src\services\ap_payment_request_service.rs",
    "src\services\ap_invoice_service.rs",
    "src\services\cost_collection_service.rs",
    "src\services\voucher_service.rs",
    "src\services\ar_invoice_service.rs",
    "src\services\supplier_service.rs",
    "src\services\inventory_count_service.rs",
    "src\services\role_permission_service.rs",
    "src\services\finance_invoice_service.rs",
    "src\services\inventory_transfer_service.rs",
    "src\services\customer_credit_service.rs",
    "src\services\sales_contract_service.rs",
    "src\services\purchase_contract_service.rs",
    "src\services\fixed_asset_service.rs",
    "src\services\budget_management_service.rs",
    "src\services\quality_standard_service.rs",
    "src\services\fund_management_service.rs",
    "src\services\quality_inspection_service.rs",
    "src\services\sales_analysis_service.rs",
    "src\services\sales_price_service.rs",
    "src\services\financial_analysis_service.rs",
    "src\services\customer_service.rs",
    "src\services\test_fixes.rs",
    "src\services\log_service.rs",
    "src\services\bpm_service.rs"
)

foreach ($file in $files) {
    $path = Join-Path $PSScriptRoot $file
    if (Test-Path $path) {
        $content = Get-Content $path -Raw -Encoding UTF8
        # 替换 .order_by_desc(X) 为 .order_by(X, Order::Desc)
        $content = $content -replace '\.order_by_desc\(([^)]+)\)', '.order_by($1, Order::Desc)'
        Set-Content $path -Value $content -Encoding UTF8 -NoNewline
        Write-Host "Fixed: $file"
    }
}

Write-Host "Done!"
