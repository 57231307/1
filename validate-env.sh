#!/bin/bash
# 环境变量验证脚本

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

ERRORS=0

# 检查必需的环境变量
check_required_var() {
    local var_name=$1
    local min_length=${2:-1}
    local value="${!var_name}"
    
    if [ -z "$value" ]; then
        echo -e "${RED}错误: $var_name 环境变量未设置${NC}"
        ERRORS=$((ERRORS + 1))
    elif [ ${#value} -lt $min_length ]; then
        echo -e "${RED}错误: $var_name 长度不足，最少需要 $min_length 个字符${NC}"
        ERRORS=$((ERRORS + 1))
    else
        echo -e "${GREEN}✓ $var_name 已设置${NC}"
    fi
}

# 检查可选的环境变量
check_optional_var() {
    local var_name=$1
    local default_value=$2
    local value="${!var_name}"
    
    if [ -z "$value" ]; then
        echo -e "${YELLOW}警告: $var_name 未设置，将使用默认值: $default_value${NC}"
    else
        echo -e "${GREEN}✓ $var_name 已设置${NC}"
    fi
}

echo "=========================================="
echo "  环境变量验证"
echo "=========================================="
echo ""

# 检查数据库配置
echo "数据库配置:"
check_required_var "DATABASE_PASSWORD" 8
check_optional_var "DATABASE__HOST" "localhost"
check_optional_var "DATABASE__PORT" "5432"
check_optional_var "DATABASE__NAME" "bingxi"
check_optional_var "DATABASE__USERNAME" "bingxi"

echo ""

# 检查认证配置
echo "认证配置:"
check_required_var "JWT_SECRET" 32
check_required_var "COOKIE_SECRET" 32
check_required_var "AUDIT_SECRET_KEY" 32

echo ""

# 检查 Redis 配置
echo "Redis 配置:"
check_optional_var "REDIS__URL" "redis://127.0.0.1:6379"

echo ""

# 检查 CORS 配置
echo "CORS 配置:"
check_optional_var "CORS__ALLOWED_ORIGINS" "http://localhost:3000"

echo ""
echo "=========================================="

if [ $ERRORS -gt 0 ]; then
    echo -e "${RED}发现 $ERRORS 个错误，请修复后重试${NC}"
    exit 1
else
    echo -e "${GREEN}所有环境变量验证通过${NC}"
    exit 0
fi