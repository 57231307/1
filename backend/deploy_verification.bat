@echo off
echo ========================================
echo 秉羲 ERP 系统 - 部署验证脚本
echo ========================================
echo.

cd /d "E:\1\10\bingxi-rust\backend"

echo [步骤 1/5] 检查环境变量...
if not exist ".env" (
    echo [错误] 未找到 .env 配置文件！
    exit /b 1
)
echo [√] .env 文件存在

set "RUST_LOG=info"
echo.

echo [步骤 2/5] 运行编译检查...
cargo check --message-format=short
if %errorlevel% neq 0 (
    echo [错误] 编译检查失败！
    echo 请查看上面的错误信息
    exit /b %errorlevel%
)
echo [√] 编译检查通过

pause
