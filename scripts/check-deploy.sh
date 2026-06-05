#!/bin/bash
# 部署配置检查脚本

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

ERRORS=0
WARNINGS=0

# 检查文件是否存在
check_file() {
    local file=$1
    local description=$2
    
    if [ -f "$file" ]; then
        echo -e "${GREEN}✓ $description: $file${NC}"
        return 0
    else
        echo -e "${RED}✗ $description: $file 不存在${NC}"
        ERRORS=$((ERRORS + 1))
        return 1
    fi
}

# 检查目录是否存在
check_dir() {
    local dir=$1
    local description=$2
    
    if [ -d "$dir" ]; then
        echo -e "${GREEN}✓ $description: $dir${NC}"
        return 0
    else
        echo -e "${RED}✗ $description: $dir 不存在${NC}"
        ERRORS=$((ERRORS + 1))
        return 1
    fi
}

# 检查命令是否存在
check_command() {
    local cmd=$1
    
    if command -v $cmd &> /dev/null; then
        echo -e "${GREEN}✓ $cmd 已安装${NC}"
        return 0
    else
        echo -e "${YELLOW}⚠ $cmd 未安装${NC}"
        WARNINGS=$((WARNINGS + 1))
        return 1
    fi
}

echo "=========================================="
echo "  部署配置检查"
echo "=========================================="
echo ""

# 检查部署脚本
echo "部署脚本:"
check_file "deploy/deploy.sh" "主部署脚本"
check_file "deploy/deploy-backend.sh" "后端部署脚本"
check_file "deploy/deploy-frontend.sh" "前端部署脚本"
check_file "deploy/deploy-prepare.sh" "部署准备脚本"

echo ""

# 检查配置文件
echo "配置文件:"
check_file "deploy/nginx.conf" "Nginx 配置"
check_file "deploy/bingxi-backend.service" "Systemd 服务"
check_file "backend/.env.example" "环境变量示例"
check_file "backend/config.yaml.example" "配置文件示例"

echo ""

# 检查 Docker 配置
echo "Docker 配置:"
check_file "Dockerfile" "主 Dockerfile"
check_file "Dockerfile.backend" "后端 Dockerfile"
check_file "Dockerfile.frontend" "前端 Dockerfile"
check_file "docker-compose.yml" "Docker Compose"
check_file "docker-entrypoint.sh" "Docker 入口脚本"
check_file ".dockerignore" "Docker 忽略文件"

echo ""

# 检查 CI/CD 配置
echo "CI/CD 配置:"
check_file ".github/workflows/ci-cd.yml" "GitHub Actions"

echo ""

# 检查环境变量验证脚本
echo "验证脚本:"
check_file "validate-env.sh" "环境变量验证脚本"

echo ""

# 检查目录结构
echo "目录结构:"
check_dir "backend" "后端目录"
check_dir "frontend" "前端目录"
check_dir "deploy" "部署目录"

echo ""

# 检查必要的工具
echo "必要工具:"
check_command "docker"
check_command "docker-compose"
check_command "nginx"
check_command "psql"
check_command "redis-cli"

echo ""
echo "=========================================="

if [ $ERRORS -gt 0 ]; then
    echo -e "${RED}发现 $ERRORS 个错误，请修复后重试${NC}"
    exit 1
elif [ $WARNINGS -gt 0 ]; then
    echo -e "${YELLOW}发现 $WARNINGS 个警告，建议安装缺失的工具${NC}"
    exit 0
else
    echo -e "${GREEN}所有检查通过${NC}"
    exit 0
fi