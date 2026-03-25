# Batch add PaginatorTrait import and fix type conversions - UTF8 safe version

$serviceFiles = Get-ChildItem "src\services\*_service.rs"

foreach ($file in $serviceFiles) {
    # Read file as UTF8
    $content = Get-Content $file.FullName -Raw -Encoding UTF8
    
    # Check if PaginatorTrait import already exists
    if ($content -match "use sea_orm::.*PaginatorTrait") {
        Write-Host "OK: $($file.Name) already has PaginatorTrait" -ForegroundColor Green
        continue
    }
    
    # Check if there is a sea_orm import with QuerySelect
    if ($content -match "use sea_orm::\{[^}]*QuerySelect") {
        # Add PaginatorTrait to existing sea_orm import
        $newContent = $content -replace "use sea_orm::\{([^}]*)QuerySelect", "use sea_orm::{$1QuerySelect, PaginatorTrait"
        $newContent | Set-Content $file.FullName -Encoding UTF8 -NoNewline
        Write-Host "ADDED PaginatorTrait: $($file.Name)" -ForegroundColor Yellow
    } elseif ($content -match "use sea_orm::\{") {
        # Add PaginatorTrait to existing sea_orm import
        $newContent = $content -replace "use sea_orm::\{", "use sea_orm::{PaginatorTrait, "
        $newContent | Set-Content $file.FullName -Encoding UTF8 -NoNewline
        Write-Host "ADDED PaginatorTrait: $($file.Name)" -ForegroundColor Yellow
    }
}

Write-Host "`nPaginatorTrait import completed!" -ForegroundColor Green
