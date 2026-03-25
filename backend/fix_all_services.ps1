# Complete fix script - adds QuerySelect, PaginatorTrait and fixes type conversions

$serviceFiles = Get-ChildItem "src\services\*_service.rs"

foreach ($file in $serviceFiles) {
    $content = Get-Content $file.FullName -Raw -Encoding UTF8
    
    # Check if already fixed
    if ($content -match "use sea_orm::.*QuerySelect.*PaginatorTrait" -or $content -match "use sea_orm::.*PaginatorTrait.*QuerySelect") {
        Write-Host "OK: $($file.Name) already fixed" -ForegroundColor Green
        continue
    }
    
    # Add imports to sea_orm::{...} block
    if ($content -match "use sea_orm::\{") {
        # Extract existing imports
        $existingMatch = [regex]::Match($content, "use sea_orm::\{([^}]*)\}")
        if ($existingMatch.Success) {
            $existingImports = $existingMatch.Groups[1].Value
            $importsToAdd = @()
            
            if ($existingImports -notmatch "QuerySelect") { $importsToAdd += "QuerySelect" }
            if ($existingImports -notmatch "PaginatorTrait") { $importsToAdd += "PaginatorTrait" }
            if ($existingImports -notmatch "QueryFilter") { $importsToAdd += "QueryFilter" }
            if ($existingImports -notmatch "ActiveModelTrait") { $importsToAdd += "ActiveModelTrait" }
            if ($existingImports -notmatch "ColumnTrait") { $importsToAdd += "ColumnTrait" }
            
            if ($importsToAdd.Count -gt 0) {
                $newImports = $existingImports + ", " + ($importsToAdd -join ", ")
                $content = $content -replace "use sea_orm::\{[^}]*\}", "use sea_orm::{$newImports}"
                Write-Host "ADDED: $($file.Name)" -ForegroundColor Yellow
            } else {
                Write-Host "OK: $($file.Name) has all imports" -ForegroundColor Green
            }
        }
    }
    
    # Fix type conversions
    $content = $content -replace '\.offset\(params\.page \* params\.page_size\)', '.offset((params.page * params.page_size) as u64)'
    $content = $content -replace '\.limit\(params\.page_size\)', '.limit(params.page_size as u64)'
    $content = $content -replace '\.limit\(params\.limit\)', '.limit(params.limit as u64)'
    $content = $content -replace '\.limit\(limit\)', '.limit(limit as u64)'
    
    # Save
    $content | Set-Content $file.FullName -Encoding UTF8 -NoNewline
}

Write-Host "`nComplete!" -ForegroundColor Green
