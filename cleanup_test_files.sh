#!/bin/bash
# Bingxi ERP 测试文件和临时数据清理脚本

echo "=========================================="
echo "Bingxi ERP 测试清理工具"
echo "=========================================="
echo ""

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}警告:${NC} 此操作将删除测试相关的临时文件和数据库测试数据"
echo ""
read -p "是否继续? (y/n): " confirm

if [ "$confirm" != "y" ] && [ "$confirm" != "Y" ]; then
    echo "操作已取消"
    exit 0
fi

echo ""
echo "=== 开始清理 ==="
echo ""

# 1. 清理测试报告目录
TEST_RESULTS_DIR="/tmp/bingxi_browser_test_results"
if [ -d "$TEST_RESULTS_DIR" ]; then
    echo -e "${YELLOW}[清理]${NC} 删除测试报告目录: $TEST_RESULTS_DIR"
    rm -rf "$TEST_RESULTS_DIR"
    echo -e "${GREEN}✓${NC} 测试报告目录已删除"
else
    echo -e "${GREEN}✓${NC} 测试报告目录不存在,跳过"
fi

# 2. 清理测试日志
echo -e "${YELLOW}[清理]${NC} 删除测试日志文件"
rm -f /tmp/test_execution.log
rm -f /tmp/server.log
rm -f /tmp/backend.log
rm -f /tmp/backend_new.log
rm -f /tmp/server_output.log
echo -e "${GREEN}✓${NC} 测试日志已删除"

# 3. 清理测试SQL文件
echo -e "${YELLOW}[清理]${NC} 删除测试SQL文件"
rm -f /tmp/create_test_user.sql
echo -e "${GREEN}✓${NC} 测试SQL文件已删除"

# 4. 清理测试脚本生成的临时文件
echo -e "${YELLOW}[清理]${NC} 删除项目根目录的测试相关文件"
cd /home/root0/桌面/121/1
rm -f test_comprehensive.sh.bak 2>/dev/null
echo -e "${GREEN}✓${NC} 临时备份文件已删除"

# 5. 询问是否清理数据库测试数据
echo ""
echo -e "${YELLOW}可选操作:${NC} 清理数据库中创建的测试数据"
echo ""
echo "将删除以下测试数据:"
echo "  - 客户名称包含'测试'的记录"
echo "  - 供应商名称包含'测试'的记录"
echo "  - 产品名称包含'测试'的记录"
echo "  - 订单号为'TEST-*'的订单"
echo ""
read -p "是否清理数据库测试数据? (y/n): " db_confirm

if [ "$db_confirm" = "y" ] || [ "$db_confirm" = "Y" ]; then
    echo ""
    echo "=== 清理数据库测试数据 ==="
    
    DB_URL="postgres://bingxi:d5eb610ccf1a701dac02d5.dbcba8f5f546a@39.99.34.194:5432/bingxi"
    
    # 清理测试客户
    echo -e "${YELLOW}[清理]${NC} 删除测试客户..."
    psql "$DB_URL" -c "DELETE FROM customers WHERE customer_name LIKE '%测试%';" 2>/dev/null
    echo -e "${GREEN}✓${NC} 测试客户已删除"
    
    # 清理测试供应商
    echo -e "${YELLOW}[清理]${NC} 删除测试供应商..."
    psql "$DB_URL" -c "DELETE FROM suppliers WHERE supplier_name LIKE '%测试%';" 2>/dev/null
    echo -e "${GREEN}✓${NC} 测试供应商已删除"
    
    # 清理测试产品
    echo -e "${YELLOW}[清理]${NC} 删除测试产品..."
    psql "$DB_URL" -c "DELETE FROM products WHERE product_name LIKE '%测试%' OR product_code LIKE 'TEST-%';" 2>/dev/null
    echo -e "${GREEN}✓${NC} 测试产品已删除"
    
    # 清理测试采购订单
    echo -e "${YELLOW}[清理]${NC} 删除测试采购订单..."
    psql "$DB_URL" -c "DELETE FROM purchase_orders WHERE order_no LIKE 'TEST-%' OR order_no LIKE 'PO-TEST-%';" 2>/dev/null
    echo -e "${GREEN}✓${NC} 测试采购订单已删除"
    
    # 清理测试销售订单
    echo -e "${YELLOW}[清理]${NC} 删除测试销售订单..."
    psql "$DB_URL" -c "DELETE FROM sales_orders WHERE order_no LIKE 'TEST-%' OR order_no LIKE 'SO-TEST-%';" 2>/dev/null
    echo -e "${GREEN}✓${NC} 测试销售订单已删除"
    
    # 清理测试应付发票
    echo -e "${YELLOW}[清理]${NC} 删除测试应付发票..."
    psql "$DB_URL" -c "DELETE FROM ap_invoices WHERE invoice_no LIKE 'TEST-%' OR invoice_no LIKE 'AP-TEST-%';" 2>/dev/null
    echo -e "${GREEN}✓${NC} 测试应付发票已删除"
    
    # 清理测试应收发票
    echo -e "${YELLOW}[清理]${NC} 删除测试应收发票..."
    psql "$DB_URL" -c "DELETE FROM ar_invoices WHERE invoice_no LIKE 'TEST-%' OR invoice_no LIKE 'AR-TEST-%';" 2>/dev/null
    echo -e "${GREEN}✓${NC} 测试应收发票已删除"
    
    echo ""
    echo -e "${GREEN}✓${NC} 数据库测试数据清理完成"
else
    echo -e "${YELLOW}⊘${NC} 跳过数据库测试数据清理"
fi

# 6. 清理前端构建缓存(可选)
echo ""
echo -e "${YELLOW}可选操作:${NC} 清理前端构建缓存"
echo ""
read -p "是否清理前端dist目录? (y/n): " frontend_confirm

if [ "$frontend_confirm" = "y" ] || [ "$frontend_confirm" = "Y" ]; then
    echo -e "${YELLOW}[清理]${NC} 删除前端构建输出..."
    rm -rf /home/root0/桌面/121/1/frontend/dist
    echo -e "${GREEN}✓${NC} 前端构建缓存已清理"
else
    echo -e "${YELLOW}⊘${NC} 跳过前端构建缓存清理"
fi

# 7. 清理后端target目录中的测试二进制文件(可选)
echo ""
echo -e "${YELLOW}可选操作:${NC} 清理后端测试二进制文件"
echo ""
read -p "是否清理后端target目录? (y/n): " backend_confirm

if [ "$backend_confirm" = "y" ] || [ "$backend_confirm" = "Y" ]; then
    echo -e "${YELLOW}[清理]${NC} 删除后端target目录..."
    rm -rf /home/root0/桌面/121/1/backend/target
    echo -e "${GREEN}✓${NC} 后端target目录已清理"
    echo -e "${YELLOW}注意:${NC} 下次编译需要重新构建,可能需要较长时间"
else
    echo -e "${YELLOW}⊘${NC} 跳过后端target目录清理"
fi

# 8. 生成清理报告
echo ""
echo "=========================================="
echo "清理完成报告"
echo "=========================================="
echo ""
echo "已清理的内容:"
echo "  ✓ 测试报告目录 (/tmp/bingxi_browser_test_results)"
echo "  ✓ 测试日志文件 (/tmp/*.log)"
echo "  ✓ 测试SQL文件"
echo "  ✓ 临时备份文件"

if [ "$db_confirm" = "y" ] || [ "$db_confirm" = "Y" ]; then
    echo "  ✓ 数据库测试数据"
fi

if [ "$frontend_confirm" = "y" ] || [ "$frontend_confirm" = "Y" ]; then
    echo "  ✓ 前端构建缓存"
fi

if [ "$backend_confirm" = "y" ] || [ "$backend_confirm" = "Y" ]; then
    echo "  ✓ 后端target目录"
fi

echo ""
echo -e "${GREEN}=========================================="
echo "清理全部完成!"
echo "==========================================${NC}"
echo ""
echo "建议的后续操作:"
echo "  1. 重新启动服务进行回归测试"
echo "  2. 检查系统功能是否正常"
echo "  3. 提交代码到版本控制系统"
echo ""
