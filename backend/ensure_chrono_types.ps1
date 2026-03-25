# Ensure chrono types are available everywhere they're used

$files = Get-ChildItem "src\**\*.rs" -Recurse

foreach ($file in $files) {
    try {
        $content = Get-Content $file.FullName -Raw -Encoding UTF8
        $original = $content
        
        # Check if file uses chrono types but doesn't have proper import
        $usesUtc = $content -match '\bUtc\b'
        $usesNaiveDate = $content -match '\bNaiveDate\b(?!Time)'
        $hasImport = $content -match '^use chrono::'
        
        if (($usesUtc -or $usesNaiveDate) -and -not $hasImport) {
            # Add chrono import at the beginning
            $chronoImport = "use chrono::{Utc, NaiveDate};`n"
            $content = $chronoImport + $content
            $content | Set-Content $file.FullName -Encoding UTF8 -NoNewline
            Write-Host "ADDED chrono import: $($file.Name)" -ForegroundColor Yellow
        }
    } catch {
        Write-Host "ERROR: $($file.Name)" -ForegroundColor Red
    }
}

Write-Host "`nChrono type imports completed!" -ForegroundColor Green
