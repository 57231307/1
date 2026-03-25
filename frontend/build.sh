// 前端构建脚本
# 安装 Trunk (如果未安装)
# cargo install --locked trunk

# 开发模式运行
echo "启动开发服务器..."
trunk serve --open

# 生产构建
echo "构建生产版本..."
trunk build --release

echo "构建完成！输出目录：dist/"
