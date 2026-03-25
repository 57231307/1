# Remove duplicate imports

$files = Get-ChildItem "src\**\*.rs" -Recurse

foreach ($file in $files) {
    try {
        $content = Get-Content $file.FullName -Raw -Encoding UTF8
        $original = $content
        
        # Remove duplicate chrono imports
        $chronoMatches = [regex]::Matches($content, "^use chrono::\{[^}]*\};`r?`n", [System.Text.RegularExpressions.RegexOptions]::Multiline)
        if ($chronoMatches.Count -gt 1) {
            # Keep only the first one
            for ($i = $chronoMatches.Count - 1; $i -gt 0; $i--) {
                $content = $content.Remove($chronoMatches[$i].Index, $chronoMatches[$i].Length)
            }
            Write-Host "FIXED duplicate chrono: $($file.Name)" -ForegroundColor Yellow
        }
        
        # Remove duplicate rust_decimal imports
        $decimalMatches = [regex]::Matches($content, "^use rust_decimal::Decimal;`r?`n", [System.Text.RegularExpressions.RegexOptions]::Multiline)
        if ($decimalMatches.Count -gt 1) {
            for ($i = $decimalMatches.Count - 1; $i -gt 0; $i--) {
                $content = $content.Remove($decimalMatches[$i].Index, $decimalMatches[$i].Length)
            }
            Write-Host "FIXED duplicate Decimal: $($file.Name)" -ForegroundColor Yellow
        }
        
        # Save if changed
        if ($content -ne $original) {
            $content | Set-Content $file.FullName -Encoding UTF8 -NoNewline
        }
    } catch {
        Write-Host "ERROR: $($file.Name) - $($_.Exception.Message)" -ForegroundColor Red
    }
}

Write-Host "`nDuplicate cleanup completed!" -ForegroundColor Green
