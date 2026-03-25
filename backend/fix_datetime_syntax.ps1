# Fix DateTime<Utc> syntax errors in model files

$modelFiles = Get-ChildItem "src\models\*.rs"

foreach ($file in $modelFiles) {
    $content = Get-Content $file.FullName -Raw -Encoding UTF8
    $original = $content
    
    # Check if file has the problematic pattern
    if ($content -match 'DateTime<Utc>') {
        # This is actually correct Rust syntax, the issue might be missing imports
        # Let's ensure DateTime is properly imported
        if ($content -notmatch 'use chrono::DateTime') {
            # Add DateTime to chrono imports
            $content = $content -replace 'use chrono::\{([^}]*)\}', 'use chrono::{$1, DateTime}'
            $content = $content -replace ',,', ','
            $content = $content -replace '\{,', '{'
            
            if ($content -ne $original) {
                $content | Set-Content $file.FullName -Encoding UTF8 -NoNewline
                Write-Host "FIXED DateTime import: $($file.Name)" -ForegroundColor Yellow
            }
        }
    }
}

Write-Host "`nDateTime syntax fixes completed!" -ForegroundColor Green
