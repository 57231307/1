# Batch add QuerySelect import to all service files - UTF8 safe version

$serviceFiles = Get-ChildItem "src\services\*_service.rs"

foreach ($file in $serviceFiles) {
    # Read file as UTF8
    $content = Get-Content $file.FullName -Raw -Encoding UTF8
    
    # Check if QuerySelect import already exists
    if ($content -match "use sea_orm::.*QuerySelect") {
        Write-Host "OK: $($file.Name) already has QuerySelect" -ForegroundColor Green
        continue
    }
    
    # Check if there is a sea_orm import
    if ($content -match "use sea_orm::\{") {
        # Add QuerySelect to existing sea_orm import
        $newContent = $content -replace "use sea_orm::\{", "use sea_orm::{QuerySelect, "
        # Write back as UTF8
        $newContent | Set-Content $file.FullName -Encoding UTF8 -NoNewline
        Write-Host "ADDED: $($file.Name) (method 1)" -ForegroundColor Yellow
    } elseif ($content -match "use sea_orm;") {
        # Replace with specific imports
        $newContent = $content -replace "use sea_orm;", "use sea_orm::{EntityTrait, QuerySelect, QueryFilter, ActiveModelTrait};"
        $newContent | Set-Content $file.FullName -Encoding UTF8 -NoNewline
        Write-Host "ADDED: $($file.Name) (method 2)" -ForegroundColor Yellow
    } else {
        # Add new import
        $import = "use sea_orm::{EntityTrait, QuerySelect, QueryFilter, ActiveModelTrait};`n"
        $newContent = $import + $content
        $newContent | Set-Content $file.FullName -Encoding UTF8 -NoNewline
        Write-Host "ADDED: $($file.Name) (method 3)" -ForegroundColor Yellow
    }
}

Write-Host "`nCompleted!" -ForegroundColor Green
