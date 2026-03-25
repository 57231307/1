@echo off
REM 秉羲管理系统 Rust 版 - Windows 构建脚本

echo === 秉羲管理系统 Rust 版 - 开始构建 ===

REM 进入后端目录
cd /d "%~dp0"

REM 清理旧的构建文件
echo 清理旧的构建文件...
cargo clean

REM 格式化代码
echo 格式化代码...
cargo fmt

REM 运行测试
echo 运行测试...
cargo test

REM 构建 release 版本
echo 构建 release 版本...
cargo build --release

REM 输出构建结果
echo.
echo === 构建完成 ===
echo 可执行文件位置：target\release\bingxi_backend.exe
echo.

REM 显示二进制文件大小
if exist "target\release\bingxi_backend.exe" (
    echo 二进制文件大小:
    dir target\release\bingxi_backend.exe
)

pause
