# Fix common import path issues

$files = Get-ChildItem "src\**\*.rs" -Recurse

Write-Host "Starting import path fixes..." -ForegroundColor Cyan

foreach ($file in $files) {
    try {
        $content = Get-Content $file.FullName -Raw -Encoding UTF8
        $original = $content
        $changed = $false
        
        # Fix crate::error -> crate::utils::error
        if ($content -match 'use crate::error::') {
            $content = $content -replace 'use crate::error::', 'use crate::utils::error::'
            Write-Host "FIXED error import: $($file.Name)" -ForegroundColor Yellow
            $changed = $true
        }
        
        # Fix crate::middleware::auth::AuthContext -> crate::middleware::auth_context::AuthContext
        if ($content -match 'use crate::middleware::auth::AuthContext') {
            $content = $content -replace 'use crate::middleware::auth::AuthContext', 'use crate::middleware::auth_context::AuthContext'
            Write-Host "FIXED AuthContext import: $($file.Name)" -ForegroundColor Yellow
            $changed = $true
        }
        
        # Fix crate::config::AppState -> crate::utils::AppState
        if ($content -match 'use crate::config::AppState') {
            $content = $content -replace 'use crate::config::AppState', 'use crate::utils::AppState'
            Write-Host "FIXED AppState import: $($file.Name)" -ForegroundColor Yellow
            $changed = $true
        }
        
        # Save if changed
        if ($changed) {
            $content | Set-Content $file.FullName -Encoding UTF8 -NoNewline
        }
    } catch {
        Write-Host "ERROR: $($file.Name) - $($_.Exception.Message)" -ForegroundColor Red
    }
}

Write-Host "`nImport path fixes completed!" -ForegroundColor Green
