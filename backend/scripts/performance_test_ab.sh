#!/bin/bash
# 性能测试脚本 - 使用 Apache Bench (ab) 进行 HTTP 压力测试
# 使用方法：./performance_test_ab.sh

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
echo -e "${GREEN}秉羲管理系统 - 性能测试脚本 (Apache Bench)${NC}"
echo -e "${GREEN}======================================${NC}"
echo ""

# 检查 ab 是否安装
if ! command -v ab &> /dev/null; then
    echo -e "${RED}错误：Apache Bench (ab) 未安装${NC}"
    echo "请先安装 apache2-utils:"
    echo "  Windows: choco install apache"
    echo "  macOS:   brew install httpd"
    echo "  Linux:   apt-get install apache2-utils 或 yum install httpd-tools"
    exit 1
fi

# 测试函数
run_test() {
    local name=$1
    local url=$2
    local method=$3
    local body=$4
    local requests=${5:-1000}
    local concurrency=${6:-100}
    
    echo -e "${YELLOW}测试：${name}${NC}"
    echo "URL: ${method} ${url}"
    echo "总请求数：${requests}, 并发数：${concurrency}"
    echo ""
    
    if [ "$method" == "GET" ]; then
        ab -n ${requests} -c ${concurrency} \
            -H "Authorization: Bearer ${TOKEN}" \
            -H "Content-Type: application/json" \
            "${url}"
    else
        # POST/PUT 请求
        echo "${body}" > /tmp/ab_body.json
        ab -n ${requests} -c ${concurrency} \
            -H "Authorization: Bearer ${TOKEN}" \
            -H "Content-Type: application/json" \
            -p /tmp/ab_body.json \
            "${url}"
        rm /tmp/ab_body.json
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
echo -e "${GREEN}=== 高并发测试 (500 并发) ===${NC}"
run_test "采购合同列表 - 高并发" "${BASE_URL}${API_PREFIX}/purchase-contracts" "GET" "" 2000 500

echo -e "${GREEN}======================================${NC}"
echo -e "${GREEN}性能测试完成!${NC}"
echo -e "${GREEN}======================================${NC}"
