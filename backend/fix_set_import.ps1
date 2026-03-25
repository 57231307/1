# Fix missing Set import in service files

$serviceFiles = Get-ChildItem "src\services\*_service.rs"

foreach ($file in $serviceFiles) {
    $content = Get-Content $file.FullName -Raw -Encoding UTF8
    
    # Check if Set is already imported
    if ($content -match "use sea_orm::.*Set") {
        Write-Host "OK: $($file.Name) already has Set" -ForegroundColor Green
        continue
    }
    
    # Add Set to sea_orm imports
    if ($content -match "use sea_orm::\{") {
        $newContent = $content -replace "use sea_orm::\{", "use sea_orm::{Set, "
        $newContent | Set-Content $file.FullName -Encoding UTF8 -NoNewline
        Write-Host "ADDED Set: $($file.Name)" -ForegroundColor Yellow
    }
}

Write-Host "`nSet import completed!" -ForegroundColor Green
