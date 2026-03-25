#!/bin/bash
# 性能测试脚本 - 使用 wrk 进行 HTTP 压力测试
# 使用方法：./performance_test_wrk.sh

# 配置
BASE_URL="${BASE_URL:-http://localhost:8080}"
API_PREFIX="/api/v1/erp"
TOKEN="${JWT_TOKEN:-your_test_token}"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}======================================${NC}"
echo -e "${GREEN}秉羲管理系统 - 性能测试脚本${NC}"
echo -e "${GREEN}======================================${NC}"
echo ""

# 检查 wrk 是否安装
if ! command -v wrk &> /dev/null; then
    echo -e "${RED}错误：wrk 未安装${NC}"
    echo "请先安装 wrk:"
    echo "  Windows: choco install wrk"
    echo "  macOS:   brew install wrk"
    echo "  Linux:   apt-get install wrk 或 yum install wrk"
    exit 1
fi

# 测试函数
run_test() {
    local name=$1
    local url=$2
    local method=$3
    local body=$4
    local connections=${5:-100}
    local threads=${6:-8}
    local duration=${7:-30s}
    
    echo -e "${YELLOW}测试：${name}${NC}"
    echo "URL: ${method} ${url}"
    echo "连接数：${connections}, 线程数：${threads}, 持续时间：${duration}"
    echo ""
    
    if [ "$method" == "GET" ]; then
        wrk -t${threads} -c${connections} -d${duration} \
            -H "Authorization: Bearer ${TOKEN}" \
            -H "Content-Type: application/json" \
            "${url}"
    else
        wrk -t${threads} -c${connections} -d${duration} \
            -H "Authorization: Bearer ${TOKEN}" \
            -H "Content-Type: application/json" \
            -s <(echo "wrk.method='${method}'" && echo "wrk.body='${body}'") \
            "${url}"
    fi
    
    echo ""
    echo "--------------------------------------"
    echo ""
}

# 健康检查测试
echo -e "${GREEN}=== 健康检查测试 ===${NC}"
run_test "健康检查" "${BASE_URL}/api/health" "GET"

# 采购合同性能测试
echo -e "${GREEN}=== 采购合同性能测试 ===${NC}"
run_test "获取采购合同列表" "${BASE_URL}${API_PREFIX}/purchase-contracts?page=1&page_size=10" "GET"
run_test "获取单个采购合同" "${BASE_URL}${API_PREFIX}/purchase-contracts/1" "GET"

# 销售合同性能测试
echo -e "${GREEN}=== 销售合同性能测试 ===${NC}"
run_test "获取销售合同列表" "${BASE_URL}${API_PREFIX}/sales-contracts?page=1&page_size=10" "GET"
run_test "获取单个销售合同" "${BASE_URL}${API_PREFIX}/sales-contracts/1" "GET"

# 固定资产性能测试
echo -e "${GREEN}=== 固定资产性能测试 ===${NC}"
run_test "获取固定资产列表" "${BASE_URL}${API_PREFIX}/fixed-assets?page=1&page_size=10" "GET"
run_test "获取单个固定资产" "${BASE_URL}${API_PREFIX}/fixed-assets/1" "GET"

# 预算管理性能测试
echo -e "${GREEN}=== 预算管理性能测试 ===${NC}"
run_test "获取预算科目列表" "${BASE_URL}${API_PREFIX}/budget-items?page=1&page_size=10" "GET"
run_test "获取单个预算科目" "${BASE_URL}${API_PREFIX}/budget-items/1" "GET"

# 高并发测试
echo -e "${GREEN}=== 高并发测试 (500 连接) ===${NC}"
run_test "采购合同列表 - 高并发" "${BASE_URL}${API_PREFIX}/purchase-contracts" "GET" "" 500 16 30s

echo -e "${GREEN}======================================${NC}"
echo -e "${GREEN}性能测试完成!${NC}"
echo -e "${GREEN}======================================${NC}"
