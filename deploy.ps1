# 秉羲ERP系统部署脚本
# 部署到远程测试服务器

$RemoteHost = "129.204.17.232"
$RemoteUser = "root"
$RemotePassword = "Txx19960917"
$BackendPort = 8080
$FrontendPort = 80

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "秉羲ERP系统 - 远程部署脚本" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

# 创建临时目录
$TempDir = "$env:TEMP\bingxi_deploy_$(Get-Date -Format 'yyyyMMddHHmmss')"
New-Item -ItemType Directory -Path $TempDir -Force | Out-Null

# 复制后端二进制文件
$BackendBin = "E:\1\10\bingxi-rust\backend\target\release\server.exe"
$BackendDest = "$TempDir\server"
Copy-Item $BackendBin $BackendDest -Force
Write-Host "[1/5] 后端程序已准备: $BackendBin" -ForegroundColor Green

# 复制前端静态文件
$FrontendDist = "E:\1\10\bingxi-rust\frontend\dist"
$FrontendDest = "$TempDir\frontend"
if (Test-Path $FrontendDest) { Remove-Item $FrontendDest -Recurse -Force }
Copy-Item $FrontendDist $FrontendDest -Recurse -Force
Write-Host "[2/5] 前端文件已准备: $FrontendDist" -ForegroundColor Green

# 创建启动脚本
$StartScript = @"
#!/bin/bash
# 秉羲ERP系统启动脚本

# 停止旧进程
pkill -f server || true
sleep 2

# 创建日志目录
mkdir -p /var/log/bingxi

# 启动后端服务
nohup ./server --addr 0.0.0.0:$BackendPort > /var/log/bingxi/backend.log 2>&1 &
echo "后端服务已启动 (PID: $!)"

# 等待后端启动
sleep 3

# 检查后端是否运行
if pgrep -f server > /dev/null; then
    echo "后端服务运行正常"
else
    echo "后端服务启动失败，请查看日志"
    exit 1
fi
"@

$StartScriptPath = "$TempDir\start_server.sh"
$StartScript | Out-File -FilePath $StartScriptPath -Encoding UTF8
Write-Host "[3/5] 启动脚本已创建" -ForegroundColor Green

# 上传到远程服务器
Write-Host "[4/5] 正在上传到远程服务器 $RemoteHost ..." -ForegroundColor Yellow

# 使用pscp上传文件
$PSCPPath = "C:\Program Files\PuTTY\pscp.exe"
if (-not (Test-Path $PSCPPath)) {
    $PSCPPath = "pscp.exe"
}

# 上传压缩包
$UploadScript = @"
echo y | "$PSCPPath" -r "$TempDir\*" $RemoteUser@$RemoteHost:/tmp/bingxi/
"@

# 使用PowerShell SSH直接执行命令
$Session = New-PSSession -HostName $RemoteHost -UserName $RemoteUser -Password $RemotePassword -ConnectionTimeout 30

if ($Session) {
    Write-Host "已连接到远程服务器" -ForegroundColor Green

    # 创建远程目录
    Invoke-Command -Session $Session -ScriptBlock {
        mkdir -p /tmp/bingxi 2>/dev/null
        mkdir -p /var/www/bingxi 2>/dev/null
        mkdir -p /var/log/bingxi 2>/dev/null
    }

    # 上传文件 - 使用SCP
    $FilesToUpload = @("$TempDir\server", "$TempDir\start_server.sh")
    foreach ($File in FilesToUpload) {
        if (Test-Path $File) {
            Copy-Item -ToSession $Session -Path $File -Destination "/tmp/bingxi/" -Force
        }
    }

    # 上传前端文件
    Copy-Item -ToSession $Session -Path "$TempDir\frontend" -Destination "/tmp/bingxi/frontend" -Recurse -Force

    Write-Host "[5/5] 文件上传完成" -ForegroundColor Green

    # 执行部署脚本
    Write-Host "正在启动远程服务..." -ForegroundColor Yellow
    Invoke-Command -Session $Session -ScriptBlock {
        cd /tmp/bingxi
        chmod +x server start_server.sh

        # 停止旧进程
        pkill -f server || true
        sleep 2

        # 启动后端
        nohup ./server --addr 0.0.0.0:$Using:BackendPort > /var/log/bingxi/backend.log 2>&1 &
        echo "后端服务已启动 (PID: $!)"

        sleep 3

        # 检查后端
        if pgrep -f server > /dev/null; then
            echo "✓ 后端服务运行正常"
        else
            echo "✗ 后端服务启动失败"
            cat /var/log/bingxi/backend.log
        fi

        # 部署前端 (如果需要Nginx)
        # cp -r /tmp/bingxi/frontend/* /var/www/bingxi/
    }

    # 关闭会话
    Remove-PSSession -Session $Session

    Write-Host ""
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "部署完成！" -ForegroundColor Green
    Write-Host "后端地址: http://$RemoteHost`:$BackendPort" -ForegroundColor Cyan
    Write-Host "前端地址: http://$RemoteHost" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
} else {
    Write-Host "无法连接到远程服务器 $RemoteHost" -ForegroundColor Red
    Write-Host "请检查网络连接和服务器状态" -ForegroundColor Yellow
}

# 清理临时文件
Remove-Item $TempDir -Recurse -Force -ErrorAction SilentlyContinue
