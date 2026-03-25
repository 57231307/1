#!/bin/bash
# 秉羲管理系统 Rust 版 - 构建脚本

set -e

echo "=== 秉羲管理系统 Rust 版 - 开始构建 ==="

# 进入后端目录
cd "$(dirname "$0")"

# 清理旧的构建文件
echo "清理旧的构建文件..."
cargo clean

# 格式化代码
echo "格式化代码..."
cargo fmt

# 运行测试
echo "运行测试..."
cargo test

# 构建 release 版本
echo "构建 release 版本..."
cargo build --release

# 输出构建结果
echo ""
echo "=== 构建完成 ==="
echo "可执行文件位置：target/release/bingxi_backend"
echo ""

# 显示二进制文件大小
if [ -f "target/release/bingxi_backend" ]; then
    echo "二进制文件大小:"
    ls -lh target/release/bingxi_backend
fi
