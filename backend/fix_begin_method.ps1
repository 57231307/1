# 批量修复 self.db.begin() 问题
$files = @(
    "src\services\ap_invoice_service.rs",
    "src\services\ap_payment_request_service.rs",
    "src\services\ap_payment_service.rs",
    "src\services\ap_reconciliation_service.rs",
    "src\services\ap_verification_service.rs",
    "src\services\inventory_count_service.rs",
    "src\services\inventory_transfer_service.rs",
    "src\services\sales_service.rs",
    "src\services\inventory_adjustment_service.rs",
    "src\services\fixed_asset_service.rs",
    "src\services\purchase_receipt_service.rs"
)

foreach ($file in $files) {
    $path = Join-Path $PSScriptRoot $file
    if (Test-Path $path) {
        $content = Get-Content $path -Raw -Encoding UTF8
        # 替换 self.db.begin() 为 (&*self.db).begin()
        $content = $content -replace 'self\.db\.begin\(\)', '(&*self.db).begin()'
        Set-Content $path -Value $content -Encoding UTF8 -NoNewline
        Write-Host "Fixed: $file"
    }
}

Write-Host "`nBegin method fix completed!" -ForegroundColor Green
