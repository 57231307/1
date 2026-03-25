# Fix missing chrono imports in service files

$serviceFiles = Get-ChildItem "src\services\*_service.rs"

foreach ($file in $serviceFiles) {
    $content = Get-Content $file.FullName -Raw -Encoding UTF8
    
    # Check if chrono is already imported
    if ($content -match "use chrono::") {
        Write-Host "OK: $($file.Name) already has chrono" -ForegroundColor Green
        continue
    }
    
    # Add chrono imports at the beginning
    $chronoImport = "use chrono::{Utc, NaiveDate, NaiveDateTime};`n"
    $newContent = $chronoImport + $content
    $newContent | Set-Content $file.FullName -Encoding UTF8 -NoNewline
    Write-Host "ADDED chrono: $($file.Name)" -ForegroundColor Yellow
}

Write-Host "`nChrono import completed!" -ForegroundColor Green
