# Add rust_decimal imports where needed

$files = Get-ChildItem "src\**\*.rs" -Recurse

foreach ($file in $files) {
    try {
        $content = Get-Content $file.FullName -Raw -Encoding UTF8
        
        # Check if file uses Decimal but doesn't have import
        if (($content -match '\bDecimal\b') -and 
            ($content -notmatch '^use rust_decimal::') -and
            ($content -notmatch '^use sea_orm::rust_decimal::')) {
            
            # Add rust_decimal import
            if ($content -match "^use\s+\w+::") {
                $decimalImport = "use rust_decimal::Decimal;`n"
                $content = $content -replace "^(use\s+\w+::)", "$decimalImport`$1"
                $content = $content -replace ';;', ';'
                $content | Set-Content $file.FullName -Encoding UTF8 -NoNewline
                Write-Host "ADDED Decimal: $($file.Name)" -ForegroundColor Yellow
            }
        }
    } catch {
        Write-Host "ERROR: $($file.Name)" -ForegroundColor Red
    }
}

Write-Host "`nDecimal imports completed!" -ForegroundColor Green
