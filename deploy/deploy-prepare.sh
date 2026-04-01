#!/bin/bash
# 秉羲ERP系统部署准备脚本
# 用途：在部署前执行准备工作

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 日志函数
log() {
    local level=$1
    local message=$2
    local timestamp=$(date +"%Y-%m-%d %H:%M:%S")
    
    case $level in
        "INFO")
            echo -e "${BLUE}[INFO]${NC} $timestamp - $message"
            ;;
        "SUCCESS")
            echo -e "${GREEN}[SUCCESS]${NC} $timestamp - $message"
            ;;
        "WARNING")
            echo -e "${YELLOW}[WARNING]${NC} $timestamp - $message"
            ;;
        "ERROR")
            echo -e "${RED}[ERROR]${NC} $timestamp - $message"
            ;;
        *)
            echo -e "[UNKNOWN] $timestamp - $message"
            ;;
    esac
}

# 检查命令是否存在
check_command() {
    local cmd=$1
    if ! command -v $cmd &> /dev/null; then
        log "ERROR" "命令 $cmd 不存在，请安装"
        exit 1
    fi
}

# 检查依赖
check_dependencies() {
    log "INFO" "检查依赖..."
    
    # 检查 Rust 相关
    check_command "cargo"
    
    # 检查前端相关
    check_command "npm"
    
    # 检查 Trunk (WebAssembly 构建工具)
    if ! command -v trunk &> /dev/null; then
        log "INFO" "安装 Trunk..."
        curl -sSL https://trunkrs.dev/install.sh | bash
        log "SUCCESS" "Trunk 安装完成"
    fi
    
    # 检查 Git
    check_command "git"
    
    log "SUCCESS" "依赖检查完成"
}

# 版本管理
manage_version() {
    log "INFO" "版本管理..."
    
    # 获取当前版本
    if [ -f "VERSION" ]; then
        CURRENT_VERSION=$(cat VERSION)
        log "INFO" "当前版本: $CURRENT_VERSION"
    else
        CURRENT_VERSION="1.0.0"
        log "INFO" "首次部署，设置初始版本: $CURRENT_VERSION"
        echo $CURRENT_VERSION > VERSION
    fi
    
    # 生成新版本号
    IFS="." read -r major minor patch <<< "$CURRENT_VERSION"
    new_patch=$((patch + 1))
    NEW_VERSION="$major.$minor.$new_patch"
    
    # 更新版本号
    echo $NEW_VERSION > VERSION
    log "SUCCESS" "版本已更新到: $NEW_VERSION"
    
    # 提交版本号变更
    if git status | grep -q "VERSION"; then
        git add VERSION
        git commit -m "Bump version to $NEW_VERSION"
        git push
        log "SUCCESS" "版本号已提交到 Git"
    fi
    
    echo $NEW_VERSION
}

# 构建前端
build_frontend() {
    log "INFO" "构建前端..."
    
    cd frontend
    
    # 安装依赖
    npm install
    
    # 构建
    trunk build --release
    
    cd ..
    
    log "SUCCESS" "前端构建完成"
}

# 构建后端
build_backend() {
    log "INFO" "构建后端..."
    
    cd backend
    
    # 构建
    cargo build --release
    
    cd ..
    
    log "SUCCESS" "后端构建完成"
}

# 打包发布
package_release() {
    local version=$1
    log "INFO" "打包发布版本: $version"
    
    # 创建发布目录
    mkdir -p release/bingxi-erp/backend
    mkdir -p release/bingxi-erp/frontend
    mkdir -p release/bingxi-erp/deploy
    
    # 复制后端文件
    cp backend/target/release/bingxi_backend release/bingxi-erp/backend/
    cp backend/.env.example release/bingxi-erp/backend/
    
    # 复制前端文件
    cp -r frontend/dist/* release/bingxi-erp/frontend/
    
    # 复制部署文件
    cp -r deploy/* release/bingxi-erp/deploy/
    
    # 复制版本文件
    cp VERSION release/bingxi-erp/
    
    # 创建发布包
    cd release
    tar -czvf bingxi-erp-$version-$(date +%Y%m%d_%H%M%S).tar.gz bingxi-erp
    cd ..
    
    log "SUCCESS" "发布包创建完成"
}

# 主函数
main() {
    log "INFO" "========================================"
    log "INFO" "  秉羲ERP系统部署准备脚本"
    log "INFO" "========================================"
    
    # 检查依赖
    check_dependencies
    
    # 版本管理
    VERSION=$(manage_version)
    
    # 构建前端
    build_frontend
    
    # 构建后端
    build_backend
    
    # 打包发布
    package_release $VERSION
    
    log "INFO" "========================================"
    log "SUCCESS" "  部署准备完成！"
    log "INFO" "========================================"
    log "INFO" "版本: $VERSION"
    log "INFO" "发布包位置: release/bingxi-erp-$VERSION-*.tar.gz"
    log "INFO" ""
    log "INFO" "下一步:"
    log "INFO" "  1. 将发布包上传到服务器"
    log "INFO" "  2. 在服务器上运行: ./deploy.sh"
    log "INFO" "  3. 访问系统: http://服务器IP"
}

# 运行主函数
main