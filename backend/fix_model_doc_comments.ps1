# Fix doc comments in model files - convert //! to ///

$modelFiles = Get-ChildItem "src\models\*.rs"

Write-Host "Fixing doc comments in model files..." -ForegroundColor Cyan

foreach ($file in $modelFiles) {
    try {
        $content = Get-Content $file.FullName -Raw -Encoding UTF8
        $original = $content
        
        # Replace //! with /// at the beginning of lines
        if ($content -match '^//!') {
            $content = $content -replace '^//!\s*', '/// '
            $content | Set-Content $file.FullName -Encoding UTF8 -NoNewline
            Write-Host "FIXED doc comment: $($file.Name)" -ForegroundColor Yellow
        }
    } catch {
        Write-Host "ERROR: $($file.Name)" -ForegroundColor Red
    }
}

Write-Host "`nDoc comment fixes completed!" -ForegroundColor Green
