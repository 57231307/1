# Remove unused ConnectionTrait imports and fix all remaining issues

$files = Get-ChildItem "src\**\*.rs" -Recurse

foreach ($file in $files) {
    try {
        $content = Get-Content $file.FullName -Raw -Encoding UTF8
        $original = $content
        
        # Remove unused ConnectionTrait from imports
        if ($content -match "use sea_orm::\{[^}]*ConnectionTrait[^}]*\}") {
            # Check if ConnectionTrait is actually used (not just imported)
            $usageCount = ([regex]::Matches($content, '\bConnectionTrait\b')).Count
            if ($usageCount -eq 1) {  # Only in import, not used
                $content = $content -replace ',\s*ConnectionTrait', ''
                $content = $content -replace 'ConnectionTrait,\s*', ''
                Write-Host "REMOVED unused ConnectionTrait: $($file.Name)" -ForegroundColor Yellow
            }
        }
        
        # Save if changed
        if ($content -ne $original) {
            $content | Set-Content $file.FullName -Encoding UTF8 -NoNewline
        }
    } catch {
        Write-Host "ERROR: $($file.Name)" -ForegroundColor Red
    }
}

Write-Host "`nConnectionTrait cleanup completed!" -ForegroundColor Green
