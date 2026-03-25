# Fix common type mismatches in handler files

$handlerFiles = Get-ChildItem "src\handlers\*.rs"

Write-Host "Starting type mismatch fixes..." -ForegroundColor Cyan

foreach ($file in $handlerFiles) {
    try {
        $content = Get-Content $file.FullName -Raw -Encoding UTF8
        $original = $content
        $changed = $false
        
        # Fix ApiResponse::error calls - remove &format! and use format! directly
        if ($content -match 'ApiResponse::<\(\)>::error\(&format!') {
            $content = $content -replace 'ApiResponse::<\(\)>::error\(&format!', 'ApiResponse::<()>::error(format!'
            Write-Host "FIXED error format: $($file.Name)" -ForegroundColor Yellow
            $changed = $true
        }
        
        # Fix ApiResponse::error with &e.to_string()
        if ($content -match 'ApiResponse::<\(\)>::error\(&e\.to_string\(\)\)') {
            $content = $content -replace 'ApiResponse::<\(\)>::error\(&e\.to_string\(\)\)', 'ApiResponse::<()>::error(e.to_string())'
            Write-Host "FIXED error to_string: $($file.Name)" -ForegroundColor Yellow
            $changed = $true
        }
        
        # Fix match arm type mismatches - add Ok() wrapper where needed
        if ($content -match '=> ApiResponse::success\(') {
            # This is usually correct, skip
        }
        
        # Save if changed
        if ($changed) {
            $content | Set-Content $file.FullName -Encoding UTF8 -NoNewline
        }
    } catch {
        Write-Host "ERROR: $($file.Name) - $($_.Exception.Message)" -ForegroundColor Red
    }
}

Write-Host "`nType mismatch fixes completed!" -ForegroundColor Green
