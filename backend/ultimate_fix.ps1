# Ultimate fix script - fixes all imports properly without double semicolons

$ErrorActionPreference = "Stop"
$serviceFiles = Get-ChildItem "src\services\*_service.rs"

Write-Host "Starting ultimate fix for $($serviceFiles.Count) service files..." -ForegroundColor Cyan

foreach ($file in $serviceFiles) {
    try {
        # Read file as UTF8
        $content = Get-Content $file.FullName -Raw -Encoding UTF8
        $original = $content
        
        # Define required imports
        $seaOrmImports = @("EntityTrait", "Set", "QueryFilter", "ColumnTrait", "ActiveModelTrait", "PaginatorTrait", "QuerySelect")
        
        # Fix sea_orm imports
        if ($content -match "use sea_orm::\{([^}]*)\}") {
            $existing = $matches[1] -split ',' | ForEach-Object { $_.Trim() } | Where-Object { $_ -ne '' }
            $allImports = $existing + $seaOrmImports
            $uniqueImports = ($allImports | Select-Object -Unique) -join ", "
            $newImport = "use sea_orm::{$uniqueImports};"
            $content = $content -replace "use sea_orm::\{[^}]*\}", $newImport
        }
        
        # Add chrono imports if file uses date/time
        if ($content -match '\b(Utc|NaiveDate|NaiveDateTime|DateTime)\b' -and $content -notmatch "use chrono::") {
            $chronoImport = "use chrono::{Utc, NaiveDate, NaiveDateTime};" + "`n"
            # Add after first use statement
            $content = $content -replace "^(use\s+\w+::)", "$chronoImport`$1"
        }
        
        # Fix type conversions
        $content = $content -replace '\.offset\(params\.page \* params\.page_size\)', '.offset((params.page * params.page_size) as u64)'
        $content = $content -replace '\.limit\(params\.page_size\)', '.limit(params.page_size as u64)'
        $content = $content -replace '\.limit\(params\.limit\)', '.limit(params.limit as u64)'
        $content = $content -replace '\.limit\(limit\)', '.limit(limit as u64)'
        
        # Remove any double semicolons
        $content = $content -replace ';;', ';'
        
        # Save if changed
        if ($content -ne $original) {
            $content | Set-Content $file.FullName -Encoding UTF8 -NoNewline
            Write-Host "  FIXED: $($file.Name)" -ForegroundColor Yellow
        } else {
            Write-Host "  OK: $($file.Name)" -ForegroundColor Green
        }
    } catch {
        Write-Host "  ERROR: $($file.Name) - $($_.Exception.Message)" -ForegroundColor Red
    }
}

Write-Host "`nUltimate fix completed!" -ForegroundColor Green
