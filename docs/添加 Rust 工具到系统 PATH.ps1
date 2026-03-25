# 添加 Rust 工具到系统 PATH
# 使用方法：右键点击此文件 -> "使用 PowerShell 运行" 或 "以管理员身份运行"

Write-Host "================================================" -ForegroundColor Cyan
Write-Host "  Rust 工具链 PATH 配置脚本" -ForegroundColor White
Write-Host "================================================" -ForegroundColor Cyan
Write-Host ""

# 定义所有 Rust 相关工具路径
$rustPaths = @(
    "e:\1\mingw64\mingw64\bin",                    # MinGW-w64 GCC
    "e:\1\protoc\bin",                              # Protocol Buffers
    "e:\1\10\llvm-mingw\llvm-mingw-20240619-ucrt-x86_64\bin"  # LLVM-MinGW
)

Write-Host "准备添加的路径:" -ForegroundColor Yellow
$rustPaths | ForEach-Object { Write-Host "  - $_" -ForegroundColor Gray }
Write-Host ""

# 获取当前系统 PATH
$currentPath = [Environment]::GetEnvironmentVariable("Path", "Machine")

# 找出需要添加的路径
$newPaths = $rustPaths | Where-Object { $currentPath -notlike "*$_*" }

if ($newPaths.Count -eq 0) {
    Write-Host "✓ 所有 Rust 工具路径已在系统 PATH 中" -ForegroundColor Green
    Write-Host ""
} else {
    Write-Host "发现 $($newPaths.Count) 个新路径需要添加:" -ForegroundColor Yellow
    $newPaths | ForEach-Object { Write-Host "  - $_" -ForegroundColor Cyan }
    Write-Host ""
    
    # 添加到系统 PATH
    try {
        $newPath = $currentPath + ";" + ($newPaths -join ";")
        [Environment]::SetEnvironmentVariable("Path", $newPath, "Machine")
        
        Write-Host "✓ 成功添加路径到系统 PATH!" -ForegroundColor Green
        Write-Host ""
        Write-Host "重要提示:" -ForegroundColor Yellow
        Write-Host "  1. 关闭所有已打开的终端窗口" -ForegroundColor White
        Write-Host "  2. 重新打开新的终端窗口" -ForegroundColor White
        Write-Host "  3. 运行以下命令验证:" -ForegroundColor White
        Write-Host "     gcc --version" -ForegroundColor Gray
        Write-Host "     protoc --version" -ForegroundColor Gray
        Write-Host ""
    } catch {
        Write-Host "✗ 添加路径失败：" $_.Exception.Message -ForegroundColor Red
        Write-Host ""
        Write-Host "请确保以管理员身份运行此脚本" -ForegroundColor Yellow
    }
}

# 显示当前系统 PATH 中的所有 Rust 相关路径
Write-Host "当前系统 PATH 中的 Rust 工具路径:" -ForegroundColor Cyan
$allRustPaths = [Environment]::GetEnvironmentVariable("Path", "Machine") -split ';' | Where-Object { $_ -like '*mingw*' -or $_ -like '*protoc*' -or $_ -like '*llvm*' }
if ($allRustPaths) {
    $allRustPaths | ForEach-Object { Write-Host "  ✓ $_" -ForegroundColor Green }
} else {
    Write-Host "  (无)" -ForegroundColor Gray
}

Write-Host ""
Write-Host "================================================" -ForegroundColor Cyan
Write-Host "  配置完成" -ForegroundColor White
Write-Host "================================================" -ForegroundColor Cyan
Write-Host ""

# 暂停，让用户看到结果
Read-Host "按回车键退出"
