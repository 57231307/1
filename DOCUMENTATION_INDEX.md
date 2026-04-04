# 秉羲管理系统 - 文档索引与导航

## 📚 文档总览

欢迎使用秉羲管理系统完整文档！本索引将帮助您快速找到所需的文档。

### 文档清单

| 文档名称 | 文件路径 | 说明 | 状态 |
|---------|---------|------|------|
| **超级完整项目文档** | [PROJECT_COMPLETE.md](file:///workspace/PROJECT_COMPLETE.md) | 最完整的项目说明文档（进行中） | 🟡 进行中 |
| **项目文档** | [PROJECT_DOCUMENTATION.md](file:///workspace/PROJECT_DOCUMENTATION.md) | 详细的项目说明文档 | 🟢 完成 |
| **代码Wiki** | [CODE_WIKI.md](file:///workspace/CODE_WIKI.md) | 项目代码知识库 | 🟢 完成 |
| **项目检查报告** | [项目检查报告.md](file:///workspace/项目检查报告.md) | 项目检查与修复报告 | 🟢 完成 |
| **合并摘要** | [MERGE_SUMMARY.md](file:///workspace/MERGE_SUMMARY.md) | 分支合并变更摘要 | 🟢 完成 |
| **合并摘要** | [merge_summary.md](file:///workspace/merge_summary.md) | 分支合并变更摘要 | 🟢 完成 |

---

## 🏗️ 项目架构总览

### 技术栈

#### 后端技术
- **语言**：Rust 2021
- **Web框架**：Axum 0.7
- **数据库**：PostgreSQL 14+
- **ORM**：SeaORM 1.0
- **认证**：JWT + Argon2
- **gRPC**：Tonic 0.12
- **监控**：Prometheus 0.13

#### 前端技术
- **框架**：Yew 0.21 (Rust WebAssembly)
- **路由**：Yew Router 0.18
- **状态管理**：Yew Context API
- **HTTP客户端**：Gloo Net 0.4

#### 部署技术
- **服务管理**：Systemd
- **反向代理**：Nginx 1.18+
- **监控**：Prometheus + Grafana + Alertmanager

### 项目规模统计

| 类别 | 数量 | 说明 |
|------|------|------|
| **后端Handler** | 60+ | API处理函数 |
| **后端Service** | 60+ | 业务逻辑服务 |
| **后端Model** | 140+ | 数据模型 |
| **前端页面** | 50+ | 页面组件 |
| **前端Service** | 40+ | API服务 |
| **前端Model** | 50+ | 数据模型 |
| **测试文件** | 10+ | 集成测试和单元测试 |

---

## 📋 核心功能模块清单

### 1. 基础管理模块
- ✅ 用户与权限管理
- ✅ 部门管理
- ✅ 角色管理
- ✅ 系统初始化

### 2. 产品管理模块
- ✅ 产品管理
- ✅ 产品分类
- ✅ 产品色号
- ✅ 产品编码映射

### 3. 仓库管理模块
- ✅ 仓库管理
- ✅ 库位管理

### 4. 库存管理模块
- ✅ 库存查询
- ✅ 库存调拨
- ✅ 库存盘点
- ✅ 库存调整
- ✅ 库存预留
- ✅ 匹数管理
- ✅ 五维度查询

### 5. 销售管理模块
- ✅ 销售订单
- ✅ 销售合同
- ✅ 销售价格
- ✅ 销售分析
- ✅ 面料销售订单
- ✅ 销售交货

### 6. 采购管理模块
- ✅ 采购订单
- ✅ 采购合同
- ✅ 采购收货
- ✅ 采购退货
- ✅ 采购价格
- ✅ 采购检验
- ✅ 采购合同执行

### 7. 供应商管理模块
- ✅ 供应商管理
- ✅ 供应商分类
- ✅ 供应商评估
- ✅ 供应商资格
- ✅ 供应商黑名单
- ✅ 供应商产品

### 8. 客户管理模块
- ✅ 客户管理
- ✅ 客户信用
- ✅ CRM（销售线索、销售机会）

### 9. 财务管理模块
- ✅ 总账管理
- ✅ 科目管理
- ✅ 凭证管理
- ✅ 应付账款（AP）
- ✅ 应收账款（AR）
- ✅ 财务分析
- ✅ 发票管理
- ✅ 付款管理
- ✅ 账户余额

### 10. 成本管理模块
- ✅ 成本归集
- ✅ 成本分析

### 11. 辅助核算模块
- ✅ 辅助核算维度
- ✅ 辅助核算记录
- ✅ 辅助核算汇总

### 12. 业务追溯模块
- ✅ 业务追溯链
- ✅ 数据快照
- ✅ 批次追踪日志

### 13. 预算管理模块
- ✅ 预算计划
- ✅ 预算执行
- ✅ 预算调整

### 14. 固定资产模块
- ✅ 固定资产管理
- ✅ 固定资产处置

### 15. 资金管理模块
- ✅ 资金账户
- ✅ 资金管理
- ✅ 资金转账记录

### 16. 质量管理模块
- ✅ 质量标准
- ✅ 质量检验
- ✅ 质量检验记录
- ✅ 不合格品管理

### 17. 面料行业核心模块
- ✅ 批次管理
- ✅ 染色批次
- ✅ 染色配方
- ✅ 缸号管理
- ✅ 坯布管理
- ✅ 匹号映射
- ✅ 双计量单位转换

### 18. OA模块
- ✅ 公告管理

### 19. BPM流程模块
- ✅ 流程定义
- ✅ 流程实例
- ✅ 流程任务

### 20. 报表模块
- ✅ 报表定义

### 21. 日志模块
- ✅ API访问日志
- ✅ 登录日志
- ✅ 系统日志
- ✅ 操作日志

### 22. 仪表盘
- ✅ 数据可视化

### 23. 系统管理
- ✅ 系统初始化
- ✅ 系统更新
- ✅ 系统版本

---

## 🔗 关键文件导航

### 后端核心文件

#### Handler文件（60+个）
- [auth_handler.rs](file:///workspace/backend/src/handlers/auth_handler.rs) - 认证处理
- [user_handler.rs](file:///workspace/backend/src/handlers/user_handler.rs) - 用户管理
- [inventory_stock_handler.rs](file:///workspace/backend/src/handlers/inventory_stock_handler.rs) - 库存管理
- [sales_order_handler.rs](file:///workspace/backend/src/handlers/sales_order_handler.rs) - 销售订单
- [purchase_order_handler.rs](file:///workspace/backend/src/handlers/purchase_order_handler.rs) - 采购订单
- [voucher_handler.rs](file:///workspace/backend/src/handlers/voucher_handler.rs) - 财务凭证
- [dye_batch_handler.rs](file:///workspace/backend/src/handlers/dye_batch_handler.rs) - 染色批次
- [greige_fabric_handler.rs](file:///workspace/backend/src/handlers/greige_fabric_handler.rs) - 坯布管理
- ... 更多60+个handler

#### Service文件（60+个）
- [auth_service.rs](file:///workspace/backend/src/services/auth_service.rs) - 认证服务
- [user_service.rs](file:///workspace/backend/src/services/user_service.rs) - 用户服务
- [inventory_stock_service.rs](file:///workspace/backend/src/services/inventory_stock_service.rs) - 库存服务
- [batch_service.rs](file:///workspace/backend/src/services/batch_service.rs) - 批次服务
- ... 更多60+个service

#### Model文件（140+个）
- [user.rs](file:///workspace/backend/src/models/user.rs) - 用户模型
- [inventory_stock.rs](file:///workspace/backend/src/models/inventory_stock.rs) - 库存模型
- [dye_batch.rs](file:///workspace/backend/src/models/dye_batch.rs) - 染色批次模型
- [greige_fabric.rs](file:///workspace/backend/src/models/greige_fabric.rs) - 坯布模型
- ... 更多140+个model

#### 中间件
- [auth.rs](file:///workspace/backend/src/middleware/auth.rs) - 认证中间件
- [logger_middleware.rs](file:///workspace/backend/src/middleware/logger_middleware.rs) - 日志中间件
- [rate_limit.rs](file:///workspace/backend/src/middleware/rate_limit.rs) - 限流中间件
- [permission.rs](file:///workspace/backend/src/middleware/permission.rs) - 权限中间件

### 前端核心文件

#### 页面文件（50+个）
- [login.rs](file:///workspace/frontend/src/pages/login.rs) - 登录页面
- [dashboard.rs](file:///workspace/frontend/src/pages/dashboard.rs) - 仪表盘
- [inventory_stock.rs](file:///workspace/frontend/src/pages/inventory_stock.rs) - 库存查询
- [dye_batch.rs](file:///workspace/frontend/src/pages/dye_batch.rs) - 染色批次
- [greige_fabric.rs](file:///workspace/frontend/src/pages/greige_fabric.rs) - 坯布管理
- ... 更多50+个页面

#### Service文件（40+个）
- [api.rs](file:///workspace/frontend/src/services/api.rs) - API基础服务
- [auth.rs](file:///workspace/frontend/src/services/auth.rs) - 认证服务
- ... 更多40+个service

### 部署与监控

- [deploy.sh](file:///workspace/deploy/deploy.sh) - 主部署脚本
- [nginx.conf](file:///workspace/deploy/nginx.conf) - Nginx配置
- [bingxi-backend.service](file:///workspace/deploy/bingxi-backend.service) - Systemd服务配置
- [prometheus.yml](file:///workspace/monitoring/prometheus/prometheus.yml) - Prometheus配置
- [alert_rules.yml](file:///workspace/monitoring/prometheus/alert_rules.yml) - 告警规则

### 配置文件

- [backend/Cargo.toml](file:///workspace/backend/Cargo.toml) - 后端依赖管理
- [frontend/Cargo.toml](file:///workspace/frontend/Cargo.toml) - 前端依赖管理
- [backend/config/settings.rs](file:///workspace/backend/src/config/settings.rs) - 应用配置
- [backend/.env.example](file:///workspace/backend/.env.example) - 环境变量示例

---

## 🚀 快速开始

### 开发环境启动

```bash
# 后端开发
cd backend
cargo run

# 前端开发
cd frontend
trunk serve
```

### 生产环境部署

```bash
# 使用部署脚本
./deploy/deploy.sh
```

### 运行测试

```bash
# 后端单元测试
cd backend
cargo test

# 后端集成测试
cd backend
cargo test --test api_test
```

---

## 📖 使用说明

### 阅读顺序建议

1. **新手入门**：
   - 先阅读 [CODE_WIKI.md](file:///workspace/CODE_WIKI.md) 了解项目基础
   - 再阅读 [PROJECT_DOCUMENTATION.md](file:///workspace/PROJECT_DOCUMENTATION.md) 了解详细内容

2. **开发者**：
   - 查看项目结构了解代码组织
   - 参考Handler、Service、Model文件进行开发
   - 查看测试文件了解测试方法

3. **运维人员**：
   - 阅读部署脚本和配置文件
   - 查看监控配置
   - 参考常见问题排查

---

## 🔄 更新记录

| 日期 | 更新内容 | 版本 |
|------|---------|------|
| 2026-04-04 | 创建文档索引和导航 | 1.0.0 |

---

## 📞 获取帮助

如有问题，请参考：
- 常见问题部分
- 项目检查报告
- 或联系开发团队

---

*本文档由秉羲团队维护，定期更新。*
