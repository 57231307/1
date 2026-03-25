# Add chrono imports to all files that need them

$files = Get-ChildItem "src\**\*.rs" -Recurse

foreach ($file in $files) {
    try {
        $content = Get-Content $file.FullName -Raw -Encoding UTF8
        $original = $content
        
        # Check if file uses chrono types but doesn't have import
        if (($content -match '\b(NaiveDate|Utc|DateTime|NaiveDateTime|Duration)\b') -and 
            ($content -notmatch '^use chrono::')) {
            
            # Add chrono import after first use statement
            if ($content -match "^use\s+\w+::") {
                $chronoImport = "use chrono::{Utc, NaiveDate, NaiveDateTime, DateTime, Duration};`n"
                $content = $content -replace "^(use\s+\w+::)", "$chronoImport`$1"
                $content = $content -replace ';;', ';'  # Fix double semicolons
                $content | Set-Content $file.FullName -Encoding UTF8 -NoNewline
                Write-Host "ADDED chrono: $($file.Name)" -ForegroundColor Yellow
            }
        }
    } catch {
        Write-Host "ERROR: $($file.Name) - $($_.Exception.Message)" -ForegroundColor Red
    }
}

Write-Host "`nChrono imports completed!" -ForegroundColor Green
