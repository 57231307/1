# Complete fix for all service files - UTF8 safe and PowerShell 5 compatible

$ErrorActionPreference = "Stop"
$serviceFiles = Get-ChildItem "src\services\*_service.rs"

Write-Host "Starting comprehensive fix for $($serviceFiles.Count) files..." -ForegroundColor Cyan

foreach ($file in $serviceFiles) {
    try {
        # Read file as UTF8
        $content = Get-Content $file.FullName -Raw -Encoding UTF8
        $original = $content
        
        # Required sea_orm imports
        $requiredImports = @("EntityTrait", "Set", "QueryFilter", "ColumnTrait", "ActiveModelTrait", "PaginatorTrait", "QuerySelect")
        
        # Check if sea_orm import exists
        if ($content -match "use sea_orm::\{([^}]*)\};?") {
            $existingImports = $matches[1]
            $importsToAdd = $requiredImports | Where-Object { $existingImports -notmatch [regex]::Escape($_) }
            
            if ($importsToAdd.Count -gt 0) {
                # Combine existing and new imports
                $allImports = ($existingImports -split ',' | ForEach-Object { $_.Trim() } | Where-Object { $_ -ne '' }) + $importsToAdd
                $uniqueImports = ($allImports | Select-Object -Unique) -join ", "
                $newImport = "use sea_orm::{$uniqueImports};"
                $content = $content -replace "use sea_orm::\{[^}]*\};?", $newImport
            }
        } elseif ($content -match "use sea_orm;") {
            $newImport = "use sea_orm::{" + ($requiredImports -join ", ") + "};"
            $content = $content -replace "use sea_orm;", $newImport
        }
        
        # Add chrono imports if not present
        if ($content -notmatch "use chrono::") {
            $content = "use chrono::{Utc, NaiveDate, NaiveDateTime};`n" + $content
        }
        
        # Fix type conversions
        $content = $content -replace '\.offset\(params\.page \* params\.page_size\)', '.offset((params.page * params.page_size) as u64)'
        $content = $content -replace '\.limit\(params\.page_size\)', '.limit(params.page_size as u64)'
        $content = $content -replace '\.limit\(params\.limit\)', '.limit(params.limit as u64)'
        $content = $content -replace '\.limit\(limit\)', '.limit(limit as u64)'
        
        # Save only if changed
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

Write-Host "`nComprehensive fix completed!" -ForegroundColor Green
