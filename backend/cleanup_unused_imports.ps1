# Remove unused imports from service files

$serviceFiles = Get-ChildItem "src\services\*_service.rs"

foreach ($file in $serviceFiles) {
    $content = Get-Content $file.FullName -Raw -Encoding UTF8
    $original = $content
    
    # Remove chrono imports if not used
    # Check if file actually uses Utc, NaiveDate, or NaiveDateTime
    $usesUtc = $content -match '\bUtc\b'
    $usesNaiveDate = $content -match '\bNaiveDate\b(?!Time)'
    $usesNaiveDateTime = $content -match '\bNaiveDateTime\b'
    
    if ($content -match "use chrono::\{([^}]*)\}") {
        $chronoImports = $matches[1] -split ',' | ForEach-Object { $_.Trim() }
        $importsToRemove = @()
        
        if (-not $usesUtc -and $chronoImports -contains 'Utc') { $importsToRemove += 'Utc' }
        if (-not $usesNaiveDate -and $chronoImports -contains 'NaiveDate') { $importsToRemove += 'NaiveDate' }
        if (-not $usesNaiveDateTime -and $chronoImports -contains 'NaiveDateTime') { $importsToRemove += 'NaiveDateTime' }
        
        if ($importsToRemove.Count -gt 0) {
            $remainingImports = $chronoImports | Where-Object { $importsToRemove -notcontains $_ }
            if ($remainingImports.Count -gt 0) {
                $newImport = "use chrono::{" + ($remainingImports -join ", ") + "};"
                $content = $content -replace "use chrono::\{[^}]*\}", $newImport
            } else {
                # Remove entire chrono import line
                $content = $content -replace "use chrono::\{[^}]*\};`n", ""
            }
        }
    }
    
    # Remove unused sea_orm imports
    if ($content -match "use sea_orm::\{([^}]*)\}") {
        $ormImports = $matches[1] -split ',' | ForEach-Object { $_.Trim() }
        $importsToRemove = @()
        
        # Check for unused imports
        if ($content -notmatch '\bPaginatorTrait\b' -and $ormImports -contains 'PaginatorTrait') { $importsToRemove += 'PaginatorTrait' }
        if ($content -notmatch '\bQuerySelect\b' -and $ormImports -contains 'QuerySelect') { $importsToRemove += 'QuerySelect' }
        
        if ($importsToRemove.Count -gt 0) {
            $remainingImports = $ormImports | Where-Object { $importsToRemove -notcontains $_ }
            if ($remainingImports.Count -gt 0) {
                $newImport = "use sea_orm::{" + ($remainingImports -join ", ") + "};"
                $content = $content -replace "use sea_orm::\{[^}]*\}", $newImport
            }
        }
    }
    
    # Save if changed
    if ($content -ne $original) {
        $content | Set-Content $file.FullName -Encoding UTF8 -NoNewline
        Write-Host "CLEANED: $($file.Name)" -ForegroundColor Yellow
    }
}

Write-Host "`nCleanup completed!" -ForegroundColor Green
