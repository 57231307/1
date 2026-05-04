# 秉羲面料管理系统 - 浏览器深入测试计划

## 一、测试范围概述

### 项目信息
- 项目名称: 秉羲面料管理 (BingXi Fabric Management)
- 技术栈: Rust后端(Axum+SeaORM) + Rust前端(Yew/WASM)
- 数据库: PostgreSQL 15
- 后端API: http://127.0.0.1:8082/api/v1/erp
- 前端界面: http://127.0.0.1:3000

### 测试策略
按优先级分三层执行:
1. **核心模块** (P0): 认证、用户、产品、销售、库存、采购、客户、供应商、仓库、财务
2. **P1/P2模块** (P1): 总账、应付、应收、成本、预算、固定资产、资金、质量
3. **边缘模块** (P2): BPM、CRM、物流、扫码、五维、辅助核算、业务追溯

## 二、业务模块分类

### 2.1 核心模块 (P0 - 最高优先级)

| 序号 | 模块名称 | API前缀 | 页面 | 关键功能 |
|------|---------|---------|------|---------|
| 1 | 认证登录 | /auth | LoginPage | 登录/登出/Token刷新/TOTP |
| 2 | 用户管理 | /users | UserListPage | 用户CRUD |
| 3 | 角色管理 | /roles | RoleListPage | 角色CRUD/权限分配 |
| 4 | 产品管理 | /products | ProductListPage | 产品CRUD/批量操作/色号管理 |
| 5 | 产品类别 | /product-categories | ProductCategoryPage | 类别CRUD/树形结构 |
| 6 | 销售订单 | /sales | SalesOrderPage | 订单CRUD/提交/审批/发货/完成 |
| 7 | 面料订单 | /sales/fabric-orders | FabricOrderPage | 面料订单CRUD/审批 |
| 8 | 库存管理 | /inventory | InventoryStockPage | 库存CRUD/出入库/库存摘要 |
| 9 | 库存转移 | /inventory/transfers | InventoryTransferPage | 转移CRUD/审批/发货/接收 |
| 10 | 库存盘点 | /inventory/counts | InventoryCountPage | 盘点CRUD/审批/完成 |
| 11 | 库存调整 | /inventory/adjustments | InventoryAdjustmentPage | 调整CRUD/审批/拒绝 |
| 12 | 采购订单 | /purchases | PurchaseOrderPage | 采购CRUD/审批/提交/拒绝/关闭 |
| 13 | 采购收货 | /purchases/receipts | PurchaseReceiptPage | 收货CRUD |
| 14 | 采购退货 | /purchases/returns | PurchaseReturnPage | 退货CRUD/审批 |
| 15 | 采购检验 | /purchases/inspections | PurchaseInspectionPage | 检验CRUD |
| 16 | 客户管理 | /customers | CustomerPage | 客户CRUD |
| 17 | 供应商管理 | /suppliers | SupplierPage | 供应商CRUD |
| 18 | 仓库管理 | /warehouses | WarehouseListPage | 仓库CRUD/库位管理 |
| 19 | 部门管理 | /departments | DepartmentListPage | 部门CRUD/树形结构 |
| 20 | 仪表板 | /dashboard | DashboardPage | 概览/销售统计/库存统计/预警 |
| 21 | 财务管理 | /finance | FinanceInvoicePage | 发票CRUD/付款管理 |
| 22 | 批次管理 | /batches | BatchPage | 批次CRUD/转移 |

### 2.2 P1/P2模块 (高优先级)

| 序号 | 模块名称 | API前缀 | 页面 | 关键功能 |
|------|---------|---------|------|---------|
| 23 | 总账科目 | /gl/subjects | AccountSubjectPage | 科目CRUD/树形结构 |
| 24 | 总账凭证 | /gl/vouchers | VoucherPage | 凭证CRUD/提交/审核/过账 |
| 25 | 应付发票 | /ap/invoices | ApInvoicePage | 应付发票CRUD/审批/账龄 |
| 26 | 应付付款 | /ap/payments | ApPaymentPage | 付款CRUD/确认 |
| 27 | 付款申请 | /ap/payment-requests | ApPaymentRequestPage | 申请CRUD/提交/审批 |
| 28 | 应付核销 | /ap/verifications | ApVerificationPage | 核销/自动/手动/取消 |
| 29 | 应付对账 | /ap/reconciliations | ApReconciliationPage | 对账/确认/争议 |
| 30 | 应付报表 | /ap/reports | ApReportPage | 统计/日报/月报/账龄 |
| 31 | 应收发票 | /ar/invoices | ArInvoicePage | 应收发票CRUD/审批 |
| 32 | 成本归集 | /cost-collections | CostCollectionPage | 成本归集CRUD |
| 33 | 预算管理 | /budgets | BudgetManagementPage | 预算CRUD/审批/执行/控制 |
| 34 | 固定资产 | /fixed-assets | FixedAssetPage | 资产CRUD/折旧 |
| 35 | 客户信用 | /customer-credits | CustomerCreditPage | 信用CRUD/占用/释放/调整 |
| 36 | 资金管理 | /fund-management | FundManagementPage | 账户CRUD/存取/冻结/转账 |
| 37 | 质量检验 | /quality-inspections | QualityInspectionPage | 检验CRUD/缺陷处理 |
| 38 | 质量标准 | /quality-standards | (嵌入质量检验) | 标准CRUD/版本/审批 |
| 39 | 财务分析 | /financial-analysis | FinancialAnalysisPage | 报表/指标/趋势 |
| 40 | 销售分析 | /sales-analysis | SalesAnalysisPage | 统计/趋势/排名/目标 |
| 41 | 销售价格 | /sales-prices | SalesPricePage | 价格CRUD/审批/历史 |
| 42 | 采购价格 | /purchase-prices | PurchasePricePage | 价格CRUD |
| 43 | 销售合同 | /sales-contracts | SalesContractPage | 合同CRUD/审批/执行 |
| 44 | 采购合同 | /purchase-contracts | PurchaseContractPage | 合同CRUD/审批/执行 |
| 45 | 供应商评估 | /supplier-evaluations | SupplierEvaluationPage | 评估CRUD/指标/排名 |
| 46 | 销售退货 | /sales-returns | SalesReturnPage | 退货CRUD |

### 2.3 面料行业核心模块 (P0)

| 序号 | 模块名称 | API前缀 | 页面 | 关键功能 |
|------|---------|---------|------|---------|
| 47 | 缸号管理 | /dye-batches | DyeBatchPage | 缸号CRUD/完成/按色查询 |
| 48 | 染色配方 | /dye-recipes | DyeRecipePage | 配方CRUD/审批/版本/查询 |
| 49 | 坯布管理 | /greige-fabrics | GreigeFabricPage | 坯布CRUD/出入库/按供应商查询 |

### 2.4 边缘/辅助模块 (P2)

| 序号 | 模块名称 | API前缀 | 页面 | 关键功能 |
|------|---------|---------|------|---------|
| 50 | BPM流程 | /bpm | MyTasks | 启动流程/审批任务/查询任务 |
| 51 | CRM线索 | /crm/leads | CrmLeadPage | 线索CRUD/状态更新 |
| 52 | CRM商机 | /crm/opportunities | CrmOpportunityPage | 商机CRUD |
| 53 | 物流管理 | /logistics | (物流页面) | 运单CRUD/状态更新 |
| 54 | 扫码出库 | /scanner | (扫码页面) | 扫码出库 |
| 55 | 五维管理 | /five-dimension | FiveDimensionPage | 统计/搜索/解析 |
| 56 | 辅助核算 | /assist-accounting | AssistAccountingPage | 维度/记录/汇总 |
| 57 | 业务追溯 | /business-trace | BusinessTracePage | 追溯/快照 |
| 58 | 双单位换算 | /dual-unit | DualUnitConverterPage | 换算/验证 |
| 59 | 系统设置 | /system-update | SystemSettingsPage | 更新/版本检查 |
| 60 | 匹号拆分 | /inventory | (库存内) | 面料匹号拆分 |

## 三、测试类型定义

### 3.1 功能测试
- CRUD操作完整性验证（创建/读取/更新/删除）
- 业务流程验证（提交→审批→执行→完成）
- 列表分页、排序、筛选功能
- 树形结构展开/收起/选择

### 3.2 性能测试
- 页面加载时间（首次渲染/API响应）
- 大数据量列表渲染性能
- 并发操作响应时间

### 3.3 按钮测试
- 每个按钮的可点击状态
- 按钮的启用/禁用逻辑
- 按钮点击后的反馈（加载状态/Success/Error）

### 3.4 异常测试
- 网络断开恢复
- API超时处理
- 无效输入验证
- 权限不足访问
- Token过期处理

### 3.5 边界测试
- 空列表状态
- 超长文本输入
- 特殊字符输入
- 最大值/最小值边界
- 并发操作冲突

### 3.6 跨模块业务流程测试
- 销售订单→库存扣减→财务发票
- 采购订单→采购收货→应付发票→付款
- 库存盘点→库存调整→凭证生成
- 客户信用→销售订单→应收发票

## 四、测试数据

### 测试账号
- 管理员: admin / (通过reset-password设置)
- 测试用户: testuser / test123456

### 测试数据范围
- 产品: 面料类产品（含色号、缸号、坯布）
- 客户/供应商: 模拟企业名称
- 订单: 模拟销售/采购订单
- 库存: 模拟仓库/库位/批次

## 五、通过准则

### 功能通过准则
- 所有CRUD操作成功率 >= 99%
- 所有业务流程完整走通
- 所有按钮响应正确
- 表单验证100%生效

### 性能通过准则
- 页面首次加载 < 3秒
- API响应时间 < 1秒
- 大数据列表渲染 < 2秒

### 安全通过准则
- 未授权访问返回401
- CSRF防护生效
- XSS防护生效
- SQL注入防护生效

## 六、风险分析

| 风险项 | 影响 | 概率 | 缓解措施 |
|--------|------|------|---------|
| WASM加载慢 | 高 | 中 | 使用CDN加速/预加载 |
| 数据库连接丢失 | 高 | 低 | 连接池重连机制 |
| 并发数据冲突 | 中 | 中 | 乐观锁/事务隔离 |
| 前端内存泄漏 | 中 | 中 | 定期内存监控 |
| API兼容性 | 高 | 低 | 版本化API/向后兼容 |

## 七、测试执行顺序

1. 连接性测试 (健康检查/数据库/初始化)
2. 认证测试 (登录/登出/Token)
3. 核心CRUD模块 (用户/角色/产品/客户/供应商)
4. 库存管理 (仓库/库存/转移/盘点/调整)
5. 销售管理 (销售订单/面料订单/退货)
6. 采购管理 (采购订单/收货/退货/检验)
7. 财务管理 (发票/付款/科目/凭证)
8. 应付管理 (应付发票/付款/核销/对账)
9. 应收管理 (应收发票)
10. 辅助模块 (预算/固定资产/信用/资金)
11. 分析模块 (质量/财务分析/销售分析)
12. 面料行业模块 (缸号/配方/坯布)
13. 边缘模块 (BPM/CRM/物流/扫码)
14. 跨模块业务流程
15. 回归测试

## 八、测试报告输出

测试完成后输出包含以下内容的完整报告:
- 测试范围与执行摘要
- 测试用例执行统计 (通过/失败/跳过)
- 缺陷清单（含严重级别、重现步骤、截图）
- 性能指标汇总
- 风险更新与建议
- 修复建议优先级排序
