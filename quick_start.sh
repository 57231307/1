#!/bin/bash
# Bingxi ERP 快速启动与测试指南

echo "=========================================="
echo "Bingxi ERP 快速启动指南"
echo "=========================================="
echo ""

# 颜色定义
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${YELLOW}本脚本将帮助您快速启动服务并执行浏览器UI测试${NC}"
echo ""

# 步骤1: 检查后端服务
echo "=== 步骤1: 检查后端服务 ==="
if curl -s http://localhost:8082/api/v1/erp/health > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} 后端服务已运行 (端口 8082)"
else
    echo -e "${YELLOW}⚠${NC} 后端服务未运行"
    echo ""
    read -p "是否启动后端服务? (y/n): " start_backend
    
    if [ "$start_backend" = "y" ] || [ "$start_backend" = "Y" ]; then
        echo "正在启动后端服务..."
        cd /home/root0/桌面/121/1/backend
        nohup ./target/release/server > /tmp/server.log 2>&1 &
        BACKEND_PID=$!
        echo "后端服务启动中 (PID: $BACKEND_PID)..."
        
        # 等待服务启动
        for i in {1..15}; do
            sleep 1
            if curl -s http://localhost:8082/api/v1/erp/health > /dev/null 2>&1; then
                echo -e "${GREEN}✓${NC} 后端服务启动成功!"
                break
            fi
            if [ $i -eq 15 ]; then
                echo -e "${RED}✗${NC} 后端服务启动超时,请检查日志: /tmp/server.log"
                exit 1
            fi
        done
    else
        echo "跳过后端启动"
    fi
fi

echo ""

# 步骤2: 检查前端服务
echo "=== 步骤2: 检查前端服务 ==="
if curl -s http://localhost:3000 > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} 前端服务已运行 (端口 3000)"
else
    echo -e "${YELLOW}⚠${NC} 前端服务未运行"
    echo ""
    read -p "是否启动前端服务? (y/n): " start_frontend
    
    if [ "$start_frontend" = "y" ] || [ "$start_frontend" = "Y" ]; then
        echo "正在启动前端服务..."
        cd /home/root0/桌面/121/1/frontend
        
        # 检查是否需要构建
        if [ ! -d "dist" ]; then
            echo "前端未构建,开始构建..."
            trunk build --release
        fi
        
        # 启动开发服务器
        trunk serve --port 3000 > /tmp/frontend.log 2>&1 &
        FRONTEND_PID=$!
        echo "前端服务启动中 (PID: $FRONTEND_PID)..."
        
        # 等待服务启动
        for i in {1..10}; do
            sleep 1
            if curl -s http://localhost:3000 > /dev/null 2>&1; then
                echo -e "${GREEN}✓${NC} 前端服务启动成功!"
                break
            fi
            if [ $i -eq 10 ]; then
                echo -e "${YELLOW}⚠${NC} 前端服务启动较慢,请稍候..."
                break
            fi
        done
    else
        echo "跳过前端启动"
    fi
fi

echo ""

# 步骤3: 验证系统状态
echo "=== 步骤3: 验证系统状态 ==="

# 检查后端健康
echo -n "后端健康检查: "
HEALTH=$(curl -s http://localhost:8082/api/v1/erp/health 2>/dev/null)
if echo "$HEALTH" | grep -q "healthy"; then
    echo -e "${GREEN}✓ 正常${NC}"
else
    echo -e "${RED}✗ 失败${NC}"
fi

# 检查初始化状态
echo -n "系统初始化状态: "
INIT_STATUS=$(curl -s http://localhost:8082/api/v1/erp/init/status 2>/dev/null)
if echo "$INIT_STATUS" | grep -q '"initialized":true'; then
    echo -e "${GREEN}✓ 已初始化${NC}"
else
    echo -e "${YELLOW}⚠ 未初始化${NC}"
    echo "请访问 http://localhost:3000/init 完成系统初始化"
fi

# 检查前端
echo -n "前端可访问性: "
if curl -s http://localhost:3000 > /dev/null 2>&1; then
    echo -e "${GREEN}✓ 可访问${NC}"
else
    echo -e "${RED}✗ 不可访问${NC}"
fi

echo ""

# 步骤4: 提供测试选项
echo "=========================================="
echo "服务已就绪,请选择测试方式"
echo "=========================================="
echo ""
echo "1. 手动浏览器测试"
echo "   - 打开浏览器访问: http://localhost:3000"
echo "   - 使用管理员账户登录"
echo "   - 按照 browser_agent_instructions.md 执行测试"
echo ""
echo "2. 自动化测试(需要Browser Agent)"
echo "   - 运行: bash run_browser_tests.sh"
echo "   - 自动执行所有测试用例"
echo "   - 自动生成测试报告"
echo ""
echo "3. API测试"
echo "   - 运行: bash test_comprehensive.sh"
echo "   - 测试后端API接口"
echo "   - 验证数据一致性"
echo ""
echo "4. 查看文档"
echo "   - 测试计划: browser_ui_test_plan.md"
echo "   - 执行指令: browser_agent_instructions.md"
echo "   - 执行总结: TEST_EXECUTION_SUMMARY.md"
echo ""

read -p "请选择操作 (1/2/3/4/q): " choice

case $choice in
    1)
        echo ""
        echo "=========================================="
        echo "手动测试指南"
        echo "=========================================="
        echo ""
        echo "1. 打开浏览器访问: http://localhost:3000"
        echo "2. 登录系统 (admin/admin123)"
        echo "3. 参考文档: browser_agent_instructions.md"
        echo "4. 逐一执行测试用例"
        echo "5. 记录问题和截图"
        echo ""
        echo "测试页面列表:"
        echo "  - /login - 登录页"
        echo "  - /dashboard - 仪表板"
        echo "  - /purchase-orders - 采购订单"
        echo "  - /sales-orders - 销售订单"
        echo "  - /inventory/stock - 库存查询"
        echo "  - /ap/invoices - 应付发票"
        echo "  - /ar/invoices - 应收发票"
        echo "  - /customers - 客户管理"
        echo "  - /suppliers - 供应商管理"
        echo "  - /products - 产品管理"
        echo ""
        ;;
    2)
        echo ""
        echo "启动自动化浏览器测试..."
        echo ""
        bash run_browser_tests.sh
        ;;
    3)
        echo ""
        echo "启动API测试..."
        echo ""
        bash test_comprehensive.sh
        ;;
    4)
        echo ""
        echo "打开文档目录..."
        echo ""
        ls -lh /home/root0/桌面/121/1/*.md | grep -E "browser|TEST|test"
        echo ""
        echo "请使用文本编辑器查看相应文档"
        ;;
    q|Q)
        echo "退出"
        exit 0
        ;;
    *)
        echo "无效选择"
        ;;
esac

echo ""
echo "=========================================="
echo "提示"
echo "=========================================="
echo ""
echo "停止服务:"
echo "  后端: kill \$(ps aux | grep 'target/release/server' | grep -v grep | awk '{print \$2}')"
echo "  前端: kill \$(ps aux | grep 'trunk serve' | grep -v grep | awk '{print \$2}')"
echo ""
echo "查看日志:"
echo "  后端: tail -f /tmp/server.log"
echo "  前端: tail -f /tmp/frontend.log"
echo ""
echo "清理测试数据:"
echo "  bash cleanup_test_files.sh"
echo ""
echo -e "${GREEN}祝测试顺利!${NC}"
echo ""
