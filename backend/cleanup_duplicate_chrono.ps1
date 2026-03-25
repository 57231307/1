# Remove duplicate chrono imports

$files = Get-ChildItem "src\**\*.rs" -Recurse

foreach ($file in $files) {
    try {
        $content = Get-Content $file.FullName -Raw -Encoding UTF8
        $original = $content
        
        # Find all chrono imports
        $chronoPattern = "^use chrono::\{[^}]*\};`r?`n"
        $chronoMatches = [regex]::Matches($content, $chronoPattern, [System.Text.RegularExpressions.RegexOptions]::Multiline)
        
        if ($chronoMatches.Count -gt 1) {
            # Keep only the first one, remove the rest
            for ($i = $chronoMatches.Count - 1; $i -gt 0; $i--) {
                $content = $content.Remove($chronoMatches[$i].Index, $chronoMatches[$i].Length)
            }
            $content | Set-Content $file.FullName -Encoding UTF8 -NoNewline
            Write-Host "FIXED duplicate chrono: $($file.Name)" -ForegroundColor Yellow
        }
    } catch {
        Write-Host "ERROR: $($file.Name)" -ForegroundColor Red
    }
}

Write-Host "`nDuplicate chrono imports cleaned!" -ForegroundColor Green
