# Bingxi ERP 浏览器UI测试 - 完整指南

## 📖 概述

本文档提供了Bingxi ERP系统进行全面浏览器UI测试的完整指南,包括测试计划、执行步骤、自动化脚本和清理工具。

**项目位置**: `/home/root0/桌面/121/1`  
**测试范围**: 58个前端页面,30+测试用例  
**测试类型**: 功能测试、UI交互测试、业务流程测试、异常处理测试

---

## 🚀 快速开始

### 一键启动与测试

```bash
cd /home/root0/桌面/121/1
bash quick_start.sh
```

该脚本将:
1. ✅ 检查并启动后端服务
2. ✅ 检查并启动前端服务
3. ✅ 验证系统状态
4. ✅ 提供测试选项

---

## 📁 文档结构

```
/home/root0/桌面/121/1/
├── browser_ui_test_plan.md          # 详细测试计划(604行)
├── browser_agent_instructions.md    # Browser Agent执行指令(386行)
├── run_browser_tests.sh             # 测试执行脚本(314行)
├── cleanup_test_files.sh            # 清理脚本(180行)
├── quick_start.sh                   # 快速启动脚本(229行)
├── test_comprehensive.sh            # API综合测试脚本(519行)
└── TEST_EXECUTION_SUMMARY.md        # 执行总结(278行)
```

---

## 🎯 测试覆盖模块

### 1. 认证与权限 (3个测试)
- 登录功能
- 登出功能
- 权限控制

### 2. 采购管理 (4个测试)
- 采购订单CRUD
- 采购收货
- 状态流转

### 3. 销售管理 (2个测试)
- 销售订单CRUD
- 订单确认

### 4. 库存管理 (3个测试)
- 库存查询
- 库存调拨
- 库存调整

### 5. 财务管理 (5个测试)
- 应付发票
- 应付付款
- 应收发票
- 应收收款
- 财务凭证

### 6. 基础数据 (3个测试)
- 客户管理
- 供应商管理
- 产品管理

### 7. 特色功能 (3个测试)
- 坯布管理
- 染色配方
- 质量管理

### 8. 报表仪表板 (1个测试)
- 仪表板数据展示

### 9. 打印功能 (3个测试)
- 采购订单打印
- 销售订单打印
- 发票打印

### 10. 异常处理 (3个测试)
- 网络异常
- 并发操作
- 表单验证

**总计**: 30+ 测试用例

---

## 🔧 环境要求

### 必需组件

1. **后端服务**
   - 端口: 8082
   - 数据库: PostgreSQL (39.99.34.194:5432/bingxi)
   - 健康检查: http://localhost:8082/api/v1/erp/health

2. **前端服务**
   - 端口: 3000
   - 访问地址: http://localhost:3000
   - 构建工具: Trunk

3. **浏览器**
   - Chrome/Firefox/Safari/Edge
   - 启用JavaScript
   - 支持localStorage

### 启动命令

```bash
# 启动后端
cd /home/root0/桌面/121/1/backend
./target/release/server

# 启动前端
cd /home/root0/桌面/121/1/frontend
trunk serve --port 3000
```

---

## 📝 测试执行

### 方式一: 手动测试

适合首次测试或详细验证:

1. 打开浏览器访问: http://localhost:3000
2. 使用管理员账户登录
3. 按照 [browser_agent_instructions.md](browser_agent_instructions.md) 逐一执行测试
4. 记录测试结果和截图
5. 填写测试报告

### 方式二: Browser Agent自动化

适合回归测试或批量验证:

```bash
bash run_browser_tests.sh
```

Browser Agent将自动:
- 导航到各个页面
- 执行CRUD操作
- 验证功能正常性
- 截图保存
- 生成测试报告

### 方式三: API测试

适合后端接口验证:

```bash
bash test_comprehensive.sh
```

测试内容包括:
- 健康检查
- 认证API
- CRUD API
- 业务流程API
- 性能测试

---

## 📊 测试报告

### 报告位置

```
/tmp/bingxi_browser_test_results/
├── test_report.md          # 完整测试报告
├── test_config.json        # 测试配置
├── issues.csv              # 问题清单
└── screenshots/            # 测试截图
```

### 报告内容

- 测试统计(通过率、失败数)
- 详细测试结果
- 发现的问题汇总
- 性能评估
- 兼容性测试
- 改进建议

---

## 🧹 清理测试数据

测试完成后清理:

```bash
bash cleanup_test_files.sh
```

清理内容:
- ✅ 测试报告目录
- ✅ 测试日志文件
- ✅ 临时SQL文件
- ⚠️  (可选)数据库测试数据
- ⚠️  (可选)前端构建缓存
- ⚠️  (可选)后端target目录

---

## ⚠️ 常见问题

### 1. 后端服务无法启动

**症状**: 服务启动后立即退出

**解决**:
```bash
# 检查端口占用
ss -tlnp | grep 8082

# 查看日志
tail -100 /tmp/server.log

# 检查配置
cat backend/config.yaml

# 重新启动
cd backend
RUST_LOG=debug ./target/release/server
```

### 2. 前端服务无法访问

**症状**: http://localhost:3000 无法打开

**解决**:
```bash
# 检查是否运行
ps aux | grep "trunk serve"

# 查看日志
tail -f /tmp/frontend.log

# 重新构建
cd frontend
trunk build --release
trunk serve --port 3000
```

### 3. 登录失败

**症状**: 提示"无效的密码"

**解决**:
- 确认系统已初始化
- 使用正确的管理员凭据
- 检查数据库中用户是否存在

### 4. API返回401未授权

**症状**: 所有需要认证的API返回401

**解决**:
- 确认已成功登录
- 检查Token是否保存在localStorage
- 验证Token是否过期

### 5. CORS错误

**症状**: 浏览器控制台显示CORS错误

**解决**:
```yaml
# 修改 backend/config.yaml
cors:
  allowed_origins:
    - "http://localhost:3000"
    - "http://127.0.0.1:3000"
```

---

## 📈 成功标准

测试通过的标准:

- ✅ 核心功能测试通过率 ≥ 95%
- ✅ 无阻塞性严重Bug
- ✅ API平均响应时间 < 200ms
- ✅ 页面加载时间 < 2秒
- ✅ 所有打印功能正常工作
- ✅ 权限控制有效
- ✅ 数据一致性验证通过

---

## 🎓 学习资源

### 项目文档

- [README.md](README.md) - 项目总览
- [CODE_WIKI.md](CODE_WIKI.md) - 代码Wiki
- [TEST_EXECUTION_SUMMARY.md](TEST_EXECUTION_SUMMARY.md) - 测试执行总结

### 测试文档

- [browser_ui_test_plan.md](browser_ui_test_plan.md) - 详细测试计划
- [browser_agent_instructions.md](browser_agent_instructions.md) - Browser Agent指令

### API文档

启动后端后访问:
- Swagger UI: http://localhost:8082/swagger-ui
- OpenAPI JSON: http://localhost:8082/api-docs/openapi.json

---

## 🔄 持续改进

### 测试流程优化

1. **建立测试基线**
   - 记录当前测试结果
   - 建立性能基准
   - 制定质量标准

2. **自动化测试**
   - 编写自动化测试脚本
   - 集成到CI/CD流程
   - 定期执行回归测试

3. **问题跟踪**
   - 使用issues.csv跟踪问题
   - 按优先级修复
   - 验证修复效果

### 质量提升

1. **代码审查**
   - 定期代码审查
   - 遵循最佳实践
   - 保持代码质量

2. **性能优化**
   - 监控API响应时间
   - 优化慢查询
   - 减少页面加载时间

3. **用户体验**
   - 收集用户反馈
   - 优化UI/UX
   - 改进错误提示

---

## 📞 支持与帮助

### 获取帮助

1. **查看日志**
   - 后端: `/tmp/server.log`
   - 前端: `/tmp/frontend.log`
   - 浏览器: DevTools Console

2. **查阅文档**
   - 测试计划: `browser_ui_test_plan.md`
   - 执行指令: `browser_agent_instructions.md`
   - API文档: Swagger UI

3. **联系团队**
   - 提交Issue
   - 代码审查
   - 技术讨论

---

## 📅 版本历史

| 版本 | 日期 | 更新内容 |
|------|------|---------|
| 1.0 | 2026-05-01 | 初始版本,创建完整测试框架 |

---

## 📄 许可证

本项目遵循与主项目相同的许可证。

---

**最后更新**: 2026-05-01  
**维护者**: AI Assistant  
**联系方式**: 通过项目Issues提交问题
