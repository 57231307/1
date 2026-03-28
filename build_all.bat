@echo off
REM 秉羲管理系统 - 完整构建和打包脚本
REM 自动构建前端和后端，并打包成发布包

setlocal enabledelayedexpansion

echo ========================================
echo 秉羲管理系统 - 完整构建和打包
echo ========================================
echo.

REM 设置版本号
set VERSION=1.0.0
for /f "tokens=2 delims==" %%i in ('findstr /C:"version" "backend\Cargo.toml"') do (
    set VERSION=%%i
)
set VERSION=%VERSION: =%
set VERSION=%VERSION:"=%

REM 获取当前日期
for /f "tokens=2 delims==" %%a in ('wmic OS Get localdatetime /value') do set "dt=%%a"
set "YYYY=%dt:~0,4%"
set "MM=%dt:~4,2%"
set "DD=%dt:~6,2%"
set "DATE=%YYYY%%MM%%DD%"

echo 版本号: %VERSION%
echo 构建日期: %DATE%
echo.

REM 创建临时构建目录
set BUILD_TEMP=build_temp
if exist "%BUILD_TEMP%" rmdir /s /q "%BUILD_TEMP%"
mkdir "%BUILD_TEMP%"

REM 构建后端
echo [1/5] 构建后端...
cd backend
call cargo build --release
if errorlevel 1 (
    echo 后端构建失败！
    cd ..
    exit /b 1
)
cd ..

echo 后端构建成功！
echo.

REM 构建前端
echo [2/5] 构建前端...
cd frontend
call trunk build --release
if errorlevel 1 (
    echo 前端构建失败！
    cd ..
    exit /b 1
)
cd ..

echo 前端构建成功！
echo.

REM 准备发布包内容
echo [3/5] 准备发布包内容...

mkdir "%BUILD_TEMP%\backend"
mkdir "%BUILD_TEMP%\frontend"
mkdir "%BUILD_TEMP%\database\migration"
mkdir "%BUILD_TEMP%\deploy"

REM 复制后端文件
copy "backend\target\release\server.exe" "%BUILD_TEMP%\backend\server.exe"
copy "backend\.env.example" "%BUILD_TEMP%\backend\.env.example"

REM 复制前端文件
xcopy /e /i /y "frontend\dist" "%BUILD_TEMP%\frontend\dist"

REM 复制数据库迁移脚本
xcopy /e /i /y "backend\database\migration" "%BUILD_TEMP%\database\migration"

REM 复制部署脚本
copy "deploy\nginx.conf" "%BUILD_TEMP%\deploy\nginx.conf"
copy "deploy\deploy.sh" "%BUILD_TEMP%\deploy\deploy.sh"
copy "deploy\bingxi-backend.service" "%BUILD_TEMP%\deploy\bingxi-backend.service"

REM 创建VERSION文件
echo %VERSION% > "%BUILD_TEMP%\VERSION"

REM 创建UPDATE_MANIFEST.json
echo { > "%BUILD_TEMP%\UPDATE_MANIFEST.json"
echo   "version": "%VERSION%", >> "%BUILD_TEMP%\UPDATE_MANIFEST.json"
echo   "release_date": "%YYYY%-%MM%-%DD%", >> "%BUILD_TEMP%\UPDATE_MANIFEST.json"
echo   "build_date": "%YYYY%-%MM%-%DD%", >> "%BUILD_TEMP%\UPDATE_MANIFEST.json"
echo   "components": [ >> "%BUILD_TEMP%\UPDATE_MANIFEST.json"
echo     "backend", >> "%BUILD_TEMP%\UPDATE_MANIFEST.json"
echo     "frontend", >> "%BUILD_TEMP%\UPDATE_MANIFEST.json"
echo     "database", >> "%BUILD_TEMP%\UPDATE_MANIFEST.json"
echo     "deploy" >> "%BUILD_TEMP%\UPDATE_MANIFEST.json"
echo   ] >> "%BUILD_TEMP%\UPDATE_MANIFEST.json"
echo } >> "%BUILD_TEMP%\UPDATE_MANIFEST.json"

REM 创建部署说明
echo # 秉羲管理系统 - 部署说明 > "%BUILD_TEMP%\README.md"
echo. >> "%BUILD_TEMP%\README.md"
echo ## 版本 %VERSION% (%YYYY%-%MM%-%DD%) >> "%BUILD_TEMP%\README.md"
echo. >> "%BUILD_TEMP%\README.md"
echo ### 部署步骤 >> "%BUILD_TEMP%\README.md"
echo. >> "%BUILD_TEMP%\README.md"
echo 1. 上传压缩包到服务器 >> "%BUILD_TEMP%\README.md"
echo 2. 解压到目标目录 >> "%BUILD_TEMP%\README.md"
echo 3. 配置 .env 文件（复制 .env.example） >> "%BUILD_TEMP%\README.md"
echo 4. 导入数据库迁移脚本 >> "%BUILD_TEMP%\README.md"
echo 5. 配置 Nginx >> "%BUILD_TEMP%\README.md"
echo 6. 启动后端服务 >> "%BUILD_TEMP%\README.md"
echo. >> "%BUILD_TEMP%\README.md"
echo ### 文件说明 >> "%BUILD_TEMP%\README.md"
echo. >> "%BUILD_TEMP%\README.md"
echo - backend/: 后端二进制文件和配置 >> "%BUILD_TEMP%\README.md"
echo - frontend/dist/: 前端静态文件 >> "%BUILD_TEMP%\README.md"
echo - database/migration/: 数据库迁移脚本 >> "%BUILD_TEMP%\README.md"
echo - deploy/: 部署相关文件 >> "%BUILD_TEMP%\README.md"

echo 发布包内容准备完成！
echo.

REM 打包成压缩包
echo [4/5] 打包发布包...

set RELEASE_NAME=bingxi-erp-v%VERSION%-%DATE%
set RELEASE_ZIP=%RELEASE_NAME%.zip
set RELEASES_DIR=releases

if not exist "%RELEASES_DIR%" mkdir "%RELEASES_DIR%"

powershell -Command "Compress-Archive -Path '%BUILD_TEMP%\*' -DestinationPath '%RELEASES_DIR%\%RELEASE_ZIP%' -Force"

if errorlevel 1 (
    echo 打包失败！
    rmdir /s /q "%BUILD_TEMP%"
    exit /b 1
)

echo 打包成功！
echo.

REM 清理临时目录
echo [5/5] 清理临时文件...
rmdir /s /q "%BUILD_TEMP%"

echo.
echo ========================================
echo 构建和打包完成！
echo ========================================
echo 发布包: %RELEASES_DIR%\%RELEASE_ZIP%
echo 版本: %VERSION%
echo 日期: %DATE%
echo ========================================
echo.

pause
