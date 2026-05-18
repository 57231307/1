# ERP用户故事与任务清单

**版本**: v1.0
**日期**: 2026-05-09
**状态**: 基于审计报告整理

---

## 用户角色定义

| 角色 | 职责 | 使用模块 |
|------|------|----------|
| 系统管理员 | 系统配置、用户管理、权限分配 | 系统管理、安全审计 |
| 销售经理 | 销售订单管理、客户管理、价格策略 | 销售、CRM、客户信用 |
| 采购经理 | 采购订单、供应商管理、价格谈判 | 采购、供应商评估 |
| 仓库管理员 | 库存管理、出入库、盘点调拨 | 库存、批次、扫码 |
| 生产计划员 | 生产排程、物料需求、产能分析 | 生产计划(MRP) |
| 财务人员 | 应收应付、凭证、报表、对账 | 财务、成本、预算 |
| 质量检验员 | 来料检验、过程检验、标准管理 | 质量检验 |
| 面料专员 | 面料特色功能、缸号、配方 | 面料订单、染色 |

---

## Phase 1: 基础加固 (第1-4周)

### 史诗1: 安全加固

#### 用户故事1.1: 权限检查补全
**作为** 系统管理员
**我希望** 所有敏感操作都有权限验证
**以便** 防止未授权访问

**验收标准**:
- [ ] 所有POST/PUT/DELETE接口检查权限
- [ ] 敏感查询添加数据范围限制
- [ ] 越权操作返回403错误

**任务清单**:
```markdown
- [ ] TASK-SEC-001: 审查所有Handler权限检查 (2d)
  - 遍历 handlers/*.rs 60+文件
  - 标记缺失权限检查的接口
  
- [ ] TASK-SEC-002: 添加缺失的permission中间件 (3d)
  - 修改 middleware/permission.rs
  - 添加默认权限检查逻辑
  
- [ ] TASK-SEC-003: 敏感操作添加操作日志 (2d)
  - 修改 services/audit_log_service.rs
  - 记录操作人、时间、IP、操作内容
  
- [ ] TASK-SEC-004: SQL注入风险排查 (1d)
  - 检查所有字符串拼接SQL
  - 替换为参数化查询
  
- [ ] TASK-SEC-005: 敏感数据日志脱敏 (1d)
  - 密码、身份证号、银行卡号脱敏
  - 配置脱敏规则
```

**涉及文件**:
- `middleware/permission.rs`
- `services/audit_log_service.rs`
- `handlers/*.rs` (60+文件)

---

### 史诗2: 数据关联

#### 用户故事2.1: BPM业务关联
**作为** 业务人员
**我希望** BPM流程与业务单据直接关联
**以便** 快速查看流程状态

**验收标准**:
- [ ] BPM流程实例显示业务单据信息
- [ ] 业务单据显示关联的流程状态
- [ ] 数据一致性100%

**任务清单**:
```markdown
- [ ] TASK-DB-001: BPM流程实例添加枚举约束 (2d)
  - 修改 models/bpm_process_instance.rs
  - 添加 BusinessType 枚举
  - 数据库迁移: 003_foreign_keys.sql
  
- [ ] TASK-DB-002: 添加联合索引 (1d)
  - (business_type, business_id) 联合索引
  - 查询性能优化
```

#### 用户故事2.2: CRM客户关联
**作为** 销售人员
**我希望** CRM线索与客户数据互通
**以便** 统一管理客户信息

**验收标准**:
- [ ] 线索可关联到客户
- [ ] 客户显示所有关联线索
- [ ] 线索转换自动创建客户

**任务清单**:
```markdown
- [ ] TASK-DB-003: CRM线索添加customer_id外键 (2d)
  - 修改 models/crm_lead.rs
  - 添加 customer_id: Option<i32>
  - 数据库迁移
  
- [ ] TASK-DB-004: 线索转换逻辑更新 (1d)
  - 修改 crm_service.rs
  - 转换时自动关联/创建客户
```

#### 用户故事2.3: 成本与生产关联
**作为** 财务人员
**我希望** 成本数据与生产批次直接关联
**以便** 准确核算生产成本

**验收标准**:
- [ ] 成本归集显示关联的生产批次
- [ ] 批次显示所有成本记录
- [ ] 成本分析可按批次筛选

**任务清单**:
```markdown
- [ ] TASK-DB-005: 成本归集添加batch_id外键 (2d)
  - 修改 models/cost_collection.rs
  - 添加 batch_id: Option<i32>
  - 保留 batch_no 用于显示
  
- [ ] TASK-DB-006: 成本查询逻辑更新 (1d)
  - 修改 cost_collection_service.rs
  - 支持按batch_id查询
```

---

### 史诗3: 代码质量

#### 用户故事3.1: 单元测试覆盖
**作为** 开发人员
**我希望** 核心功能有单元测试
**以便** 保证代码质量

**验收标准**:
- [ ] 核心Service测试覆盖率>60%
- [ ] CI自动运行测试
- [ ] 测试报告可查看

**任务清单**:
```markdown
- [ ] TASK-TEST-001: auth_service单元测试 (1d)
  - 测试登录/注册/密码验证
  - 测试Token生成/验证
  
- [ ] TASK-TEST-002: user_service单元测试 (1d)
  - 测试用户CRUD
  - 测试权限检查
  
- [ ] TASK-TEST-003: sales_service单元测试 (2d)
  - 测试订单创建/审批/发货
  - 测试状态流转
  
- [ ] TASK-TEST-004: purchase_order_service单元测试 (2d)
  - 测试采购订单CRUD
  - 测试交期计算
  
- [ ] TASK-TEST-005: 集成测试 (2d)
  - 登录->创建订单->审批->发货流程
  - 采购->入库->检验->付款流程
```

#### 用户故事3.2: 代码文档补全
**作为** 新加入的开发人员
**我希望** 代码有完整的文档注释
**以便** 快速理解系统

**验收标准**:
- [ ] 所有pub函数有文档注释
- [ ] 模块有模块级文档
- [ ] 复杂逻辑有示例代码

**任务清单**:
```markdown
- [ ] TASK-DOC-001: handlers文档补全 (2d)
  - 为所有Handler函数添加 /// 注释
  - 说明接口功能、参数、返回值
  
- [ ] TASK-DOC-002: services文档补全 (2d)
  - 为所有Service方法添加文档
  - 说明业务逻辑
  
- [ ] TASK-DOC-003: models文档补全 (1d)
  - 为所有Model添加字段说明
  - 说明关联关系
```

---

## Phase 2: 核心功能 (第5-10周)

### 史诗4: 生产计划管理 (MRP)

#### 用户故事4.1: 生产订单管理
**作为** 生产计划员
**我希望** 创建和管理生产订单
**以便** 安排生产任务

**验收标准**:
- [ ] 生产订单CRUD
- [ ] 订单状态流转
- [ ] 订单优先级管理

**任务清单**:
```markdown
- [ ] TASK-MRP-001: 生产订单数据模型 (1d)
  - 新建 models/production_order.rs
  - 定义 ProductionOrder 实体
  
- [ ] TASK-MRP-002: 生产订单Service (2d)
  - 新建 services/production_order_service.rs
  - 实现CRUD + 状态流转
  
- [ ] TASK-MRP-003: 生产订单Handler (2d)
  - 新建 handlers/production_order_handler.rs
  - 实现RESTful API
  
- [ ] TASK-MRP-004: 生产订单路由 (1d)
  - 修改 routes/mod.rs
  - 注册 /api/v1/erp/production/* 路由
```

#### 用户故事4.2: BOM物料清单
**作为** 生产计划员
**我希望** 维护产品的物料清单
**以便** 计算物料需求

**验收标准**:
- [ ] BOM多级结构
- [ ] BOM版本管理
- [ ] BOM用量计算

**任务清单**:
```markdown
- [ ] TASK-MRP-005: BOM数据模型 (1d)
  - 新建 models/bom.rs
  - 定义 Bom + BomItem 实体
  
- [ ] TASK-MRP-006: BOM Service (2d)
  - 新建 services/bom_service.rs
  - 实现BOM CRUD + 版本管理
  
- [ ] TASK-MRP-007: BOM Handler (1d)
  - 新建 handlers/bom_handler.rs
  - 实现RESTful API
```

#### 用户故事4.3: 物料需求计算
**作为** 生产计划员
**我希望** 系统自动计算物料需求
**以便** 及时采购原材料

**验收标准**:
- [ ] 毛需求计算(基于销售订单)
- [ ] 净需求计算(考虑库存)
- [ ] 需求日期推算

**任务清单**:
```markdown
- [ ] TASK-MRP-008: MRP计算引擎 (3d)
  - 新建 services/mrp_engine.rs
  - 实现MRP算法
  - 考虑库存/在途/在制
  
- [ ] TASK-MRP-009: MRP结果存储 (1d)
  - 新建 models/mrp_result.rs
  - 存储计算结果
  
- [ ] TASK-MRP-010: MRP API (1d)
  - 新建 handlers/mrp_handler.rs
  - 触发计算/查询结果
```

#### 用户故事4.4: 产能负荷分析
**作为** 生产经理
**我希望** 查看设备产能负荷
**以便** 合理安排生产

**验收标准**:
- [ ] 设备产能维护
- [ ] 负荷计算
- [ ] 瓶颈识别

**任务清单**:
```markdown
- [ ] TASK-MRP-011: 工作中心模型 (1d)
  - 新建 models/work_center.rs
  - 定义设备/产线信息
  
- [ ] TASK-MRP-012: 产能计算 (2d)
  - 新建 services/capacity_service.rs
  - 负荷计算与可视化数据
```

#### 用户故事4.5: 缺料预警
**作为** 采购经理
**我希望** 系统提前预警缺料
**以便** 及时采购

**验收标准**:
- [ ] 预警规则配置
- [ ] 预警通知
- [ ] 缺料报表

**任务清单**:
```markdown
- [ ] TASK-MRP-013: 预警规则 (1d)
  - 新建 models/alert_rule.rs
  - 配置预警阈值
  
- [ ] TASK-MRP-014: 预警服务 (2d)
  - 新建 services/alert_service.rs
  - 定时检查/触发预警
  
- [ ] TASK-MRP-015: 预警通知 (1d)
  - 集成事件总线
  - 发送通知(站内信/邮件)
```

---

### 史诗5: 应收对账

#### 用户故事5.1: 应收对账单
**作为** 财务人员
**我希望** 生成应收对账单
**以便** 与客户对账

**验收标准**:
- [ ] 对账单自动生成
- [ ] 包含发票/收款明细
- [ ] 期初/期末余额

**任务清单**:
```markdown
- [ ] TASK-AR-001: 应收对账模型 (1d)
  - 参考 ap_reconciliation.rs
  - 新建 models/ar_reconciliation.rs
  
- [ ] TASK-AR-002: 对账服务 (2d)
  - 新建 services/ar_reconciliation_service.rs
  - 复制应付对账逻辑并修改
  
- [ ] TASK-AR-003: 对账Handler (1d)
  - 新建 handlers/ar_reconciliation_handler.rs
  - 实现RESTful API
  
- [ ] TASK-AR-004: 对账路由 (1d)
  - 修改 routes/mod.rs
  - 注册 /api/v1/erp/ar/reconciliations/*
```

#### 用户故事5.2: 账龄分析
**作为** 财务经理
**我希望** 查看应收账款账龄
**以便** 评估坏账风险

**验收标准**:
- [ ] 账龄分段(30/60/90/180/360天)
- [ ] 按客户汇总
- [ ] 可视化报表

**任务清单**:
```markdown
- [ ] TASK-AR-005: 账龄分析服务 (2d)
  - 新建 services/ar_aging_service.rs
  - 计算账龄分布
  
- [ ] TASK-AR-006: 账龄报表API (1d)
  - 新建 handlers/ar_aging_handler.rs
  - 返回账龄分析数据
```

---

### 史诗6: 多币种支持

#### 用户故事6.1: 币种管理
**作为** 财务人员
**我希望** 管理多种货币
**以便** 处理外贸业务

**验收标准**:
- [ ] 币种CRUD
- [ ] 本位币设置
- [ ] 汇率管理

**任务清单**:
```markdown
- [ ] TASK-CUR-001: 币种模型 (1d)
  - 新建 models/currency.rs
  - 定义 Currency 实体
  
- [ ] TASK-CUR-002: 汇率模型 (1d)
  - 新建 models/exchange_rate.rs
  - 定义 ExchangeRate 实体
  
- [ ] TASK-CUR-003: 币种服务 (1d)
  - 新建 services/currency_service.rs
  - 实现CRUD
  
- [ ] TASK-CUR-004: 汇率服务 (1d)
  - 新建 services/exchange_rate_service.rs
  - 手动/自动更新汇率
```

#### 用户故事6.2: 多币种交易
**作为** 销售人员
**我希望** 创建外币订单
**以便** 服务海外客户

**验收标准**:
- [ ] 订单支持选择币种
- [ ] 自动换算本位币
- [ ] 发票显示双币种

**任务清单**:
```markdown
- [ ] TASK-CUR-005: 订单币种支持 (2d)
  - 修改 models/sales_order.rs
  - 添加 currency_code + exchange_rate 字段
  
- [ ] TASK-CUR-006: 发票币种支持 (2d)
  - 修改 models/ar_invoice.rs
  - 添加币种字段
  
- [ ] TASK-CUR-007: 换算逻辑 (1d)
  - 修改相关Service
  - 自动计算本位币金额
```

---

## Phase 3: 高级功能 (第11-18周)

### 史诗7: AI智能分析

#### 用户故事7.1: 销售预测
**作为** 销售总监
**我希望** 系统预测未来销售
**以便** 制定销售策略

**验收标准**:
- [ ] 基于历史数据的预测
- [ ] 按产品/客户/区域维度
- [ ] 预测准确率>80%

**任务清单**:
```markdown
- [ ] TASK-AI-001: 数据准备 (2d)
  - 提取历史销售数据
  - 数据清洗/特征工程
  
- [ ] TASK-AI-002: 预测模型 (3d)
  - 选择时间序列算法
  - 训练/验证模型
  
- [ ] TASK-AI-003: 预测API (2d)
  - 新建 handlers/ai_forecast_handler.rs
  - 返回预测结果
```

#### 用户故事7.2: 库存优化
**作为** 仓库经理
**我希望** 系统推荐安全库存
**以便** 降低库存成本

**验收标准**:
- [ ] 安全库存计算
- [ ] 再订货点提醒
- [ ] 库存周转分析

**任务清单**:
```markdown
- [ ] TASK-AI-004: 库存分析服务 (3d)
  - 新建 services/inventory_analysis_service.rs
  - 计算安全库存/再订货点
  
- [ ] TASK-AI-005: 优化建议API (1d)
  - 新建 handlers/inventory_optimization_handler.rs
  - 返回优化建议
```

---

### 史诗8: 报表自定义

#### 用户故事8.1: 自定义报表
**作为** 管理人员
**我希望** 自定义报表格式
**以便** 满足个性化需求

**验收标准**:
- [ ] 拖拽式报表设计
- [ ] 多种图表类型
- [ ] 报表导出(PDF/Excel)

**任务清单**:
```markdown
- [ ] TASK-RPT-001: 报表模板引擎 (3d)
  - 新建 services/report_engine.rs
  - 支持动态SQL/字段选择
  
- [ ] TASK-RPT-002: 报表设计器API (3d)
  - 新建 handlers/report_designer_handler.rs
  - 保存/加载报表模板
  
- [ ] TASK-RPT-003: 报表导出 (2d)
  - 集成PDF/Excel导出库
  - 实现导出功能
```

---

## Phase 4: 生态扩展 (第19-30周)

### 史诗9: 多租户SaaS

#### 用户故事9.1: 租户管理
**作为** 系统运营商
**我希望** 支持多租户
**以便** SaaS化运营

**验收标准**:
- [ ] 租户注册/开通
- [ ] 数据隔离
- [ ] 配置隔离

**任务清单**:
```markdown
- [ ] TASK-SAAS-001: 租户模型 (2d)
  - 新建 models/tenant.rs
  - 定义 Tenant 实体
  
- [ ] TASK-SAAS-002: 租户隔离 (4d)
  - 修改所有查询添加tenant_id过滤
  - 或Schema级别隔离
  
- [ ] TASK-SAAS-003: 租户配置 (2d)
  - 新建 services/tenant_config_service.rs
  - 租户级配置管理
```

---

### 史诗10: 开放API平台

#### 用户故事10.1: 开发者平台
**作为** 第三方开发者
**我希望** 接入ERP系统
**以便** 开发集成应用

**验收标准**:
- [ ] API文档完善
- [ ] SDK提供
- [ ] 沙箱环境

**任务清单**:
```markdown
- [ ] TASK-OPEN-001: API网关 (3d)
  - 新建 middleware/api_gateway.rs
  - 路由/限流/认证
  
- [ ] TASK-OPEN-002: 开发者文档 (2d)
  - 完善Swagger文档
  - 编写接入指南
  
- [ ] TASK-OPEN-003: SDK开发 (5d)
  - 提供JavaScript/Python SDK
  - 示例代码
```

---

## 任务总览

### Phase 1 (第1-4周)
| 任务ID | 任务 | 工时 | 依赖 |
|--------|------|------|------|
| SEC-001~005 | 安全加固 | 9d | - |
| DB-001~006 | 数据关联 | 7d | - |
| TEST-001~005 | 单元测试 | 8d | - |
| DOC-001~003 | 代码文档 | 5d | - |

### Phase 2 (第5-10周)
| 任务ID | 任务 | 工时 | 依赖 |
|--------|------|------|------|
| MRP-001~015 | 生产计划管理 | 18d | Phase 1 |
| AR-001~006 | 应收对账 | 7d | Phase 1 |
| CUR-001~007 | 多币种支持 | 9d | Phase 1 |

### Phase 3 (第11-18周)
| 任务ID | 任务 | 工时 | 依赖 |
|--------|------|------|------|
| AI-001~005 | AI智能分析 | 11d | Phase 2 |
| RPT-001~003 | 报表自定义 | 8d | Phase 2 |

### Phase 4 (第19-30周)
| 任务ID | 任务 | 工时 | 依赖 |
|--------|------|------|------|
| SAAS-001~003 | 多租户SaaS | 8d | Phase 3 |
| OPEN-001~003 | 开放API平台 | 10d | Phase 3 |

---

## 任务依赖图

```
Phase 1:
  SEC-001 --> SEC-002 --> SEC-003
  DB-001 --> DB-002 --> DB-003 --> DB-004 --> DB-005 --> DB-006
  TEST-001~005 (并行)
  DOC-001~003 (并行)

Phase 2:
  MRP-001 --> MRP-002 --> MRP-003 --> MRP-004
  MRP-005 --> MRP-006 --> MRP-007
  MRP-008 --> MRP-009 --> MRP-010
  MRP-011 --> MRP-012
  MRP-013 --> MRP-014 --> MRP-015
  
  AR-001 --> AR-002 --> AR-003 --> AR-004
  AR-005 --> AR-006
  
  CUR-001 --> CUR-002 --> CUR-003 --> CUR-004
  CUR-005 --> CUR-006 --> CUR-007

Phase 3:
  AI-001 --> AI-002 --> AI-003
  AI-004 --> AI-005
  
  RPT-001 --> RPT-002 --> RPT-003

Phase 4:
  SAAS-001 --> SAAS-002 --> SAAS-003
  
  OPEN-001 --> OPEN-002 --> OPEN-003
```

---

**文档结束**
