#!/usr/bin/env pwsh
# 修复 DateTime<Utc> 到 NaiveDateTime 的转换错误

$files = @(
    "src\services\ap_reconciliation_service.rs",
    "src\services\ap_payment_request_service.rs",
    "src\services\ap_verification_service.rs",
    "src\services\ap_payment_service.rs",
    "src\services\bpm_service.rs",
    "src\services\operation_log_service.rs",
    "src\services\code_conversion_service.rs",
    "src\services\supplier_service.rs"
)

foreach ($file in $files) {
    $path = Join-Path $PSScriptRoot $file
    if (Test-Path $path) {
        $content = Get-Content $path -Raw -Encoding UTF8
        $original = $content
        
        # 修复 Utc::now().into() -> Utc::now().naive_utc()
        $content = $content -replace 'Utc::now\(\)\.into\(\)', 'Utc::now().naive_utc()'
        
        # 修复 dt.into() -> dt.naive_utc()  
        $content = $content -replace '\.map\(\|dt\| dt\.into\(\)\)', '.map(|dt| dt.naive_utc())'
        
        if ($content -ne $original) {
            $content | Set-Content $path -Encoding UTF8 -NoNewline
            Write-Host "FIXED: $file" -ForegroundColor Yellow
        } else {
            Write-Host "OK: $file" -ForegroundColor Green
        }
    } else {
        Write-Host "NOT FOUND: $path" -ForegroundColor Red
    }
}

Write-Host "`nDateTime conversion fixes completed!" -ForegroundColor Green
