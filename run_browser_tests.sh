#!/bin/bash
# Bingxi ERP 浏览器UI自动化测试执行脚本
# 此脚本将启动Browser Agent进行全面的UI测试

echo "=========================================="
echo "Bingxi ERP 浏览器UI自动化测试"
echo "=========================================="
echo ""

# 检查服务状态
echo "=== 检查后端服务 ==="
if curl -s http://localhost:8082/api/v1/erp/health > /dev/null 2>&1; then
    echo "✅ 后端服务运行正常 (端口 8082)"
else
    echo "❌ 后端服务未运行"
    echo "请先启动后端: cd /home/root0/桌面/121/1/backend && ./target/release/server"
    exit 1
fi

echo ""
echo "=== 检查前端服务 ==="
if curl -s http://localhost:3000 > /dev/null 2>&1; then
    echo "✅ 前端服务运行正常 (端口 3000)"
else
    echo "⚠️  前端服务未运行"
    echo "建议启动前端: cd /home/root0/桌面/121/1/frontend && trunk serve --port 3000"
    echo "将继续使用后端API进行测试..."
fi

echo ""
echo "=========================================="
echo "开始浏览器UI测试"
echo "=========================================="
echo ""

# 创建测试结果目录
TEST_RESULTS_DIR="/tmp/bingxi_browser_test_results"
mkdir -p ${TEST_RESULTS_DIR}/screenshots
mkdir -p ${TEST_RESULTS_DIR}/reports

# 测试配置文件
cat > ${TEST_RESULTS_DIR}/test_config.json << 'EOF'
{
  "backend_url": "http://localhost:8082",
  "frontend_url": "http://localhost:3000",
  "test_user": {
    "username": "admin",
    "password": "admin123"
  },
  "test_modules": [
    "authentication",
    "purchase_orders",
    "sales_orders",
    "inventory",
    "finance_ap",
    "finance_ar",
    "customers",
    "suppliers",
    "products",
    "dashboard",
    "printing"
  ],
  "options": {
    "capture_screenshots": true,
    "timeout_ms": 30000,
    "headless": false
  }
}
EOF

echo "测试配置已保存至: ${TEST_RESULTS_DIR}/test_config.json"
echo ""

# 由于需要使用Browser Agent,这里提供测试指令
echo "=========================================="
echo "测试执行说明"
echo "=========================================="
echo ""
echo "请按照以下步骤执行浏览器测试:"
echo ""
echo "1. 确保后端和前端服务已启动"
echo "2. 打开浏览器访问: http://localhost:3000"
echo "3. 使用以下凭据登录:"
echo "   用户名: admin"
echo "   密码: admin123 (或初始化时设置的密码)"
echo ""
echo "4. 依次测试以下模块:"
echo "   - 登录/登出功能"
echo "   - 采购订单管理 (CRUD)"
echo "   - 销售订单管理 (CRUD)"
echo "   - 库存管理 (查询、调拨、调整)"
echo "   - 应付账款 (发票、付款)"
echo "   - 应收账款 (发票、收款)"
echo "   - 客户管理"
echo "   - 供应商管理"
echo "   - 产品管理"
echo "   - 仪表板数据展示"
echo "   - 打印功能 (如有)"
echo ""
echo "5. 记录发现的问题并截图"
echo "6. 填写测试报告模板"
echo ""

# 生成测试报告模板
cat > ${TEST_RESULTS_DIR}/test_report_template.md << 'EOF'
# Bingxi ERP 浏览器UI测试报告

## 测试基本信息

- **测试日期**: $(date '+%Y-%m-%d %H:%M:%S')
- **测试环境**: Local Development
- **后端地址**: http://localhost:8082
- **前端地址**: http://localhost:3000
- **浏览器**: [请填写]
- **测试人员**: AI Browser Agent

## 测试统计

| 模块 | 测试项数 | 通过 | 失败 | 跳过 | 备注 |
|------|---------|------|------|------|------|
| 认证与权限 | 3 | | | | |
| 采购管理 | 4 | | | | |
| 销售管理 | 1 | | | | |
| 库存管理 | 3 | | | | |
| 财务管理 | 3 | | | | |
| 基础数据 | 3 | | | | |
| 特色功能 | 3 | | | | |
| 报表仪表板 | 2 | | | | |
| 系统设置 | 2 | | | | |
| 打印功能 | 3 | | | | |
| **总计** | **27** | | | | |

## 详细测试结果

### 1. 认证与权限

#### 1.1 登录功能
- [ ] 正常登录成功
- [ ] 错误密码提示正确
- [ ] 空字段验证有效
- [ ] 登录后跳转正确

**问题记录**:


#### 1.2 登出功能
- [ ] 登出成功
- [ ] Token清除
- [ ] 跳转到登录页

**问题记录**:


#### 1.3 权限控制
- [ ] 无权限页面拦截
- [ ] 按钮级权限控制
- [ ] 角色切换正常

**问题记录**:


### 2. 采购管理

#### 2.1 采购订单创建
- [ ] 表单显示正常
- [ ] 必填项验证
- [ ] 供应商选择
- [ ] 产品选择
- [ ] 保存成功
- [ ] 订单号生成

**问题记录**:


#### 2.2 采购订单编辑
- [ ] 加载订单详情
- [ ] 修改字段
- [ ] 保存更改
- [ ] 数据更新

**问题记录**:


#### 2.3 采购订单删除
- [ ] 删除确认对话框
- [ ] 删除成功
- [ ] 列表刷新

**问题记录**:


#### 2.4 采购收货
- [ ] 创建收货单
- [ ] 关联采购订单
- [ ] 库存更新
- [ ] 状态流转

**问题记录**:


### 3. 销售管理

[类似结构,略]

### 4. 库存管理

[类似结构,略]

### 5. 财务管理

[类似结构,略]

### 6. 基础数据

[类似结构,略]

### 7. 特色功能

[类似结构,略]

### 8. 报表仪表板

[类似结构,略]

### 9. 系统设置

[类似结构,略]

### 10. 打印功能

[类似结构,略]

## 发现的问题汇总

### 严重问题 (阻塞性Bug)

1. **[问题标题]**
   - 模块: 
   - 页面: 
   - 复现步骤: 
   - 预期结果: 
   - 实际结果: 
   - 截图: 
   - 优先级: P0

### 一般问题

[类似问题记录]

### 轻微问题 (UI/UX改进)

[类似问题记录]

## 性能评估

- 首页加载时间: _____ ms
- 列表页加载时间: _____ ms
- 表单提交响应时间: _____ ms
- API平均响应时间: _____ ms

## 兼容性测试

| 浏览器 | 版本 | 测试结果 | 备注 |
|--------|------|---------|------|
| Chrome | | ✅/❌ | |
| Firefox | | ✅/❌ | |
| Safari | | ✅/❌ | |
| Edge | | ✅/❌ | |

## 测试结论

### 整体评价
[对系统质量的整体评价]

### 主要优点
1. 
2. 
3. 

### 主要不足
1. 
2. 
3. 

### 改进建议
1. 
2. 
3. 

### 发布建议
- [ ] 可以发布
- [ ] 需要修复严重问题后发布
- [ ] 不建议发布,存在重大缺陷

---

**报告生成时间**: $(date '+%Y-%m-%d %H:%M:%S')  
**下次测试计划**: [日期]
EOF

echo ""
echo "测试报告模板已生成: ${TEST_RESULTS_DIR}/test_report_template.md"
echo ""
echo "=========================================="
echo "测试准备完成"
echo "=========================================="
echo ""
echo "下一步操作:"
echo "1. 手动或使用Browser Agent执行测试"
echo "2. 填写测试报告模板"
echo "3. 保存截图到: ${TEST_RESULTS_DIR}/screenshots/"
echo "4. 完成后运行清理脚本"
echo ""
