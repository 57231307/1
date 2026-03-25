# Final comprehensive cleanup - remove all unused imports

$serviceFiles = Get-ChildItem "src\services\*_service.rs"
$handlerFiles = Get-ChildItem "src\handlers\*_handler.rs"

$allFiles = $serviceFiles + $handlerFiles

Write-Host "Cleaning $($allFiles.Count) files..." -ForegroundColor Cyan

foreach ($file in $allFiles) {
    $content = Get-Content $file.FullName -Raw -Encoding UTF8
    $original = $content
    
    # Remove completely unused chrono import
    if ($content -match "^use chrono::\{[^}]*\};`r?`n") {
        $chronoLine = $matches[0]
        if (-not ($content -match '\b(Utc|NaiveDate|NaiveDateTime|DateTime|Duration)\b')) {
            $content = $content -replace [regex]::Escape($chronoLine), ""
        }
    }
    
    # Remove unused sea_orm individual imports
    $ormPatterns = @{
        'PaginatorTrait' = '\bPaginatorTrait\b'
        'QuerySelect' = '\bQuerySelect\b'
        'Set' = '\bSet\s*\('
        'ActiveModelTrait' = '\.update\(|\.insert\(|\.delete\('
        'QueryFilter' = '\.filter\('
        'ColumnTrait' = 'Column::'
        'EntityTrait' = '::Entity::'
        'TransactionTrait' = '\.transaction\('
    }
    
    if ($content -match "use sea_orm::\{([^}]*)\}") {
        $imports = $matches[1] -split ',' | ForEach-Object { $_.Trim() }
        $toRemove = @()
        
        foreach ($import in $imports) {
            if ($ormPatterns.ContainsKey($import)) {
                if ($content -notmatch $ormPatterns[$import]) {
                    $toRemove += $import
                }
            }
        }
        
        if ($toRemove.Count -gt 0) {
            $remaining = $imports | Where-Object { $toRemove -notcontains $_ }
            if ($remaining.Count -gt 0) {
                $newImport = "use sea_orm::{" + ($remaining -join ", ") + "};"
                $content = $content -replace "use sea_orm::\{[^}]*\}", $newImport
            } else {
                $content = $content -replace "use sea_orm::\{[^}]*\};`r?`n", ""
            }
        }
    }
    
    # Remove unused Serialize
    if ($content -match "use serde::Serialize;" -and $content -notmatch "#\[derive\([^)]*Serialize") {
        $content = $content -replace "use serde::Serialize;`r?`n", ""
    }
    
    # Remove unused tracing imports
    if ($content -match "use tracing::\{([^}]*)\}") {
        $tracingImports = $matches[1] -split ',' | ForEach-Object { $_.Trim() }
        $toRemove = @()
        
        foreach ($import in $tracingImports) {
            if ($content -notmatch "\b$import\b") {
                $toRemove += $import
            }
        }
        
        if ($toRemove.Count -gt 0) {
            $remaining = $tracingImports | Where-Object { $toRemove -notcontains $_ }
            if ($remaining.Count -gt 0) {
                $newImport = "use tracing::{" + ($remaining -join ", ") + "};"
                $content = $content -replace "use tracing::\{[^}]*\}", $newImport
            } else {
                $content = $content -replace "use tracing::\{[^}]*\};`r?`n", ""
            }
        }
    }
    
    # Save if changed
    if ($content -ne $original) {
        $content | Set-Content $file.FullName -Encoding UTF8 -NoNewline
        Write-Host "  CLEANED: $($file.Name)" -ForegroundColor Yellow
    }
}

Write-Host "`nFinal cleanup completed!" -ForegroundColor Green
