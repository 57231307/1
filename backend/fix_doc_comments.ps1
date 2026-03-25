# Fix doc comment issues (E0753)

$files = Get-ChildItem "src\**\*.rs" -Recurse

Write-Host "Starting doc comment fixes..." -ForegroundColor Cyan

$count = 0

foreach ($file in $files) {
    try {
        $content = Get-Content $file.FullName -Raw -Encoding UTF8
        $original = $content
        
        # Remove outer doc comments that are causing issues
        // This pattern matches //! at the beginning of files
        if ($content -match '^//!\[') {
            $content = $content -replace '^//!\[', '// ['
            $content | Set-Content $file.FullName -Encoding UTF8 -NoNewline
            Write-Host "FIXED doc comment: $($file.Name)" -ForegroundColor Yellow
            $count++
        }
    } catch {
        Write-Host "ERROR: $($file.Name)" -ForegroundColor Red
    }
}

Write-Host "`nFixed $count files with doc comment issues!" -ForegroundColor Green
