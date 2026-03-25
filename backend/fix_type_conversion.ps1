# Fix type conversion issues (i64 to u64) in service files

$serviceFiles = Get-ChildItem "src\services\*_service.rs"

$count = 0

foreach ($file in $serviceFiles) {
    $content = Get-Content $file.FullName -Raw -Encoding UTF8
    $original = $content
    
    # Fix offset parameter - convert i64 to u64
    $content = $content -replace '\.offset\(params\.page \* params\.page_size\)', '.offset((params.page * params.page_size) as u64)'
    
    # Fix limit parameter - convert i64 to u64  
    $content = $content -replace '\.limit\(params\.page_size\)', '.limit(params.page_size as u64)'
    $content = $content -replace '\.limit\(params\.limit\)', '.limit(params.limit as u64)'
    $content = $content -replace '\.limit\(limit\)', '.limit(limit as u64)'
    
    if ($content -ne $original) {
        $content | Set-Content $file.FullName -Encoding UTF8 -NoNewline
        Write-Host "FIXED: $($file.Name)" -ForegroundColor Yellow
        $count++
    }
}

Write-Host "`nFixed $count files!" -ForegroundColor Green
