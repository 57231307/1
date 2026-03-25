# Comprehensive fix for remaining type errors

$files = Get-ChildItem "src\**\*.rs" -Recurse

Write-Host "Starting comprehensive type fix..." -ForegroundColor Cyan

foreach ($file in $files) {
    try {
        $content = Get-Content $file.FullName -Raw -Encoding UTF8
        $original = $content
        $changed = $false
        
        # Fix RustDecimal -> rust_decimal::Decimal or just Decimal
        if ($content -match '\bRustDecimal\b') {
            $content = $content -replace '\bRustDecimal\b', 'Decimal'
            Write-Host "FIXED RustDecimal: $($file.Name)" -ForegroundColor Yellow
            $changed = $true
        }
        
        # Fix generic argument issues (e.g., Vec<Model> where Model doesn't need generics)
        # This is a common pattern issue
        
        # Save if changed
        if ($changed) {
            $content | Set-Content $file.FullName -Encoding UTF8 -NoNewline
        }
    } catch {
        Write-Host "ERROR: $($file.Name) - $($_.Exception.Message)" -ForegroundColor Red
    }
}

Write-Host "`nComprehensive type fix completed!" -ForegroundColor Green
