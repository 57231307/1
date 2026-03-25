# Simple script to add Rust tools to System PATH
# Run as Administrator

$ErrorActionPreference = "Stop"

Write-Host "Adding Rust tools to System PATH..." -ForegroundColor Cyan

$rustPaths = @(
    "e:\1\mingw64\mingw64\bin",
    "e:\1\protoc\bin",
    "e:\1\10\llvm-mingw\llvm-mingw-20240619-ucrt-x86_64\bin"
)

$currentPath = [Environment]::GetEnvironmentVariable("Path", "Machine")
$newPaths = $rustPaths | Where-Object { $currentPath -notlike "*$_*" }

if ($newPaths.Count -gt 0) {
    $newPath = $currentPath + ";" + ($newPaths -join ";")
    [Environment]::SetEnvironmentVariable("Path", $newPath, "Machine")
    Write-Host "SUCCESS: Added $($newPaths.Count) paths to System PATH" -ForegroundColor Green
    $newPaths | ForEach-Object { Write-Host "  - $_" -ForegroundColor Cyan }
    Write-Host "`nPlease close all terminals and open a new one to apply changes." -ForegroundColor Yellow
} else {
    Write-Host "All Rust tool paths are already in System PATH" -ForegroundColor Green
}
