# Fix &self.db to &*self.db in all service files

$serviceFiles = Get-ChildItem "src\services\*_service.rs"

foreach ($file in $serviceFiles) {
    $content = Get-Content $file.FullName -Raw -Encoding UTF8
    
    # Replace &self.db with &*self.db
    if ($content -match '&self\.db') {
        $newContent = $content -replace '&self\.db', '&*self.db'
        $newContent | Set-Content $file.FullName -Encoding UTF8 -NoNewline
        Write-Host "FIXED: $($file.Name)" -ForegroundColor Yellow
    }
}

Write-Host "`nAll &self.db references fixed!" -ForegroundColor Green
