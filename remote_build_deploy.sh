#!/bin/bash
# 秉羲ERP系统 - 远程服务器编译和部署脚本
# 在远程服务器上执行

echo "========================================"
echo "秉羲ERP系统 - 远程编译部署脚本"
echo "========================================"

# 创建工作目录
mkdir -p /tmp/bingxi_build
cd /tmp/bingxi_build

# 停止旧服务
pkill -f server || true
sleep 2

# 安装Rust（如果没有）
if ! command -v rustc &> /dev/null; then
    echo "正在安装 Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
fi

# 克隆或更新代码
if [ -d "/tmp/bingxi_source" ]; then
    echo "更新代码..."
    cd /tmp/bingxi_source
    git pull origin main
else
    echo "克隆代码（请替换为实际仓库地址）..."
    # git clone <repository_url> /tmp/bingxi_source
    # cd /tmp/bingxi_source
fi

# 创建前端构建目录
mkdir -p /var/www/bingxi
mkdir -p /var/log/bingxi

# 编译后端
echo "编译后端..."
cd /tmp/bingxi_source/backend
cargo build --release
if [ $? -eq 0 ]; then
    echo "✓ 后端编译成功"
    cp target/release/server /tmp/bingxi/server
else
    echo "✗ 后端编译失败"
    exit 1
fi

# 编译前端（需要安装wasm-pack）
echo "安装wasm-pack..."
if ! command -v wasm-pack &> /dev/null; then
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

echo "编译前端..."
cd /tmp/bingxi_source/frontend
wasm-pack build --target web --release
if [ $? -eq 0 ]; then
    echo "✓ 前端编译成功"
    cp -r pkg/* /var/www/bingxi/
else
    echo "✗ 前端编译失败"
    exit 1
fi

# 启动后端服务
cd /tmp
nohup ./server --addr 0.0.0.0:8080 > /var/log/bingxi/backend.log 2>&1 &
BACKEND_PID=$!
echo "后端服务已启动 (PID: $BACKEND_PID)"

# 等待启动
sleep 5

# 检查状态
if pgrep -f server > /dev/null; then
    echo "✓ 后端服务运行正常 (PID: $(pgrep -f server))"
else
    echo "✗ 后端服务启动失败"
    cat /var/log/bingxi/backend.log
    exit 1
fi

# 检查端口
if netstat -tuln | grep -q ":8080 "; then
    echo "✓ 后端端口 8080 监听正常"
fi

echo ""
echo "========================================"
echo "部署完成！"
echo "后端地址: http://$(hostname -I | awk '{print $1}'):8080"
echo "前端地址: http://$(hostname -I | awk '{print $1}'):80"
echo "日志位置: /var/log/bingxi/"
echo "========================================"
