# Bingxi ERP 全面缺失功能实施计划

> 基于深度代码审计，覆盖 10+ 模块、100+ 功能项的完整实施计划
> 技术栈：Rust (Axum + SeaORM) + Vue 3 + TypeScript + Element Plus

## 规划概览

| 阶段 | 模块 | 功能项数 | 核心内容 |
|------|------|---------|---------|
| 第一阶段 | 生产计划(MRP) | 35 | BOM管理、MRP计算引擎、产能分析、缺料预警、排程甘特图 |
| 第二阶段 | 应收对账+多币种 | 18 | 自动对账、账龄分析、订单币种支持、汇率自动更新 |
| 第三阶段 | 报表引擎+导入导出 | 14 | PDF/Excel导出、报表设计器、通用导入框架 |
| 第四阶段 | AI分析真实化 | 10 | 替换假数据、真实ML算法、准确率评估 |
| 第五阶段 | BPM+CRM增强 | 24 | 流程定义管理、客户360视图、客户分级、客户信息维护、客户公海、客户分配、客户新增 |
| 第六阶段 | 系统级功能 | 16 | 字段权限、操作审计、邮件通知、Webhook集成 |
| 第七阶段 | 多租户SaaS | 10 | 数据隔离、计费系统、自助注册 |
| 第八阶段 | 前端交互增强 | 16 | ECharts图表、批量操作、高级筛选、拖拽排序 |

---

## 第一阶段：生产计划管理（MRP）完善

- [ ] 1. BOM 物料清单管理后端
  - [ ] 1.1 创建 `bom_model.rs` 数据模型
    - 定义 `boms` 表结构（id, product_id, version, status, created_at）
    - 定义 `bom_items` 表结构（id, bom_id, material_product_id, quantity, unit, scrap_rate）
    - 实现 SeaORM Entity 和 ActiveModel
  - [ ] 1.2 创建 `bom_service.rs` 业务逻辑
    - 实现 BOM CRUD（创建/读取/更新/删除）
    - 实现 BOM 版本管理（版本号自增、历史版本保留、版本对比）
    - 实现 BOM 复制功能（从现有 BOM 复制创建新版本）
    - 实现 BOM 树形结构查询（递归展开子物料）
  - [ ] 1.3 创建 `bom_handler.rs` HTTP 接口
    - `POST /api/v1/erp/boms` - 创建 BOM
    - `GET /api/v1/erp/boms` - 列表查询（支持分页、筛选产品）
    - `GET /api/v1/erp/boms/:id` - 获取 BOM 详情（含物料明细）
    - `PUT /api/v1/erp/boms/:id` - 更新 BOM
    - `DELETE /api/v1/erp/boms/:id` - 删除 BOM
    - `GET /api/v1/erp/boms/:id/versions` - 获取版本历史
    - `POST /api/v1/erp/boms/:id/copy` - 复制 BOM
  - [ ] 1.4 注册路由到 `routes/mod.rs`
  - [ ] 1.5 编写 BOM 单元测试

- [ ] 2. MRP 物料需求计算引擎
  - [ ] 2.1 扩展 `mrp_engine_service.rs`
    - 实现 MRP 计算核心算法（毛需求→净需求→计划订单）
    - 实现 BOM 展开逻辑（递归展开多层 BOM）
    - 实现库存扣减逻辑（考虑在手量、在途量、安全库存）
    - 实现批量计算（支持多个产品同时计算）
  - [ ] 2.2 创建 `mrp_handler.rs` HTTP 接口
    - `POST /api/v1/erp/mrp/calculate` - 触发 MRP 计算
    - `GET /api/v1/erp/mrp/results` - 查询计算结果
    - `GET /api/v1/erp/mrp/requirements` - 物料需求清单
    - `POST /api/v1/erp/mrp/convert-orders` - 将需求转为采购/生产订单
  - [ ] 2.3 注册路由到 `routes/mod.rs`
  - [ ] 2.4 编写 MRP 计算单元测试

- [ ] 3. 生产订单增强
  - [ ] 3.1 扩展 `production_order_handler.rs`
    - 添加关联销售订单字段和查询
    - 添加 BOM 关联字段
    - 实现生产订单审批流程（通过 BPM 引擎）
    - 实现订单优先级排序
  - [ ] 3.2 扩展 `production_order_service.rs`
    - 实现从销售订单自动生成生产订单
    - 实现生产订单状态流转（草稿→审批中→已批准→生产中→完成→关闭）
    - 实现生产进度跟踪（工序完成百分比）

- [ ] 4. 产能负荷分析
  - [ ] 4.1 创建 `capacity_service.rs`
    - 定义产能数据模型（工作中心、日产能、班次）
    - 实现产能负荷计算（已排产/总产能）
    - 实现产能瓶颈识别
  - [ ] 4.2 创建 `capacity_handler.rs` HTTP 接口
    - `GET /api/v1/erp/capacity/overview` - 产能概览
    - `GET /api/v1/erp/capacity/work-centers` - 工作中心列表
    - `GET /api/v1/erp/capacity/load-analysis` - 负荷分析
  - [ ] 4.3 注册路由

- [ ] 5. 缺料预警
  - [ ] 5.1 创建 `material_shortage_service.rs`
    - 实现缺料检测逻辑（对比库存与生产需求）
    - 实现预警阈值配置
    - 实现缺料清单生成
  - [ ] 5.2 创建 `material_shortage_handler.rs`
    - `GET /api/v1/erp/material-shortage/alerts` - 缺料预警列表
    - `POST /api/v1/erp/material-shortage/check` - 手动触发检查
    - `GET /api/v1/erp/material-shortage/summary` - 缺料汇总

- [ ] 6. 生产排程
  - [ ] 6.1 创建 `scheduling_service.rs`
    - 实现简单排程算法（前向排程/后向排程）
    - 实现工序时间计算
    - 实现甘特图数据生成
  - [ ] 6.2 创建 `scheduling_handler.rs`
    - `POST /api/v1/erp/scheduling/auto` - 自动排程
    - `GET /api/v1/erp/scheduling/gantt` - 甘特图数据

- [ ] 7. 前端 - 生产计划模块完善
  - [ ] 7.1 创建 BOM 管理页面 `frontend/src/views/bom/`
    - BOM 列表页（产品树 + BOM 版本列表）
    - BOM 编辑页（物料明细表格，支持拖拽排序）
    - BOM 版本对比视图
  - [ ] 7.2 创建 MRP 计算页面 `frontend/src/views/mrp/`
    - MRP 计算触发面板
    - 物料需求清单表格
    - 需求转订单操作
  - [ ] 7.3 完善生产订单页面 `frontend/src/views/production/`
    - 添加关联销售订单选择下拉框
    - 添加 BOM 选择下拉框
    - 添加审批流程状态显示
  - [ ] 7.4 创建产能分析页面 `frontend/src/views/capacity/`
    - 产能负荷仪表盘（ECharts 环形图）
    - 工作中心负荷时间轴
  - [ ] 7.5 创建缺料预警页面 `frontend/src/views/material-shortage/`
    - 缺料预警列表
    - 缺料汇总统计
  - [ ] 7.6 创建排程甘特图页面 `frontend/src/views/scheduling/`
    - 甘特图组件（使用 dhtmlx-gantt 或 ECharts Gantt）
    - 排程操作面板
  - [ ] 7.7 创建对应 API 文件 `frontend/src/api/bom.ts`, `mrp.ts`, `capacity.ts`, `material-shortage.ts`, `scheduling.ts`

---

## 第二阶段：应收对账 + 多币种完善

- [ ] 8. 应收对账模块增强
  - [ ] 8.1 扩展 `ar_reconciliation_handler.rs`
    - 实现对账明细项（从发票和收款自动匹配）
    - 实现自动对账算法（按金额/日期/客户匹配）
    - 实现账龄分析计算
    - 实现对账单自动生成（从发票/收款汇总）
    - 实现客户确认/争议处理流程
  - [ ] 8.2 扩展 `ar_reconciliation_service.rs` 或创建新 service
    - 实现自动对账核心逻辑
    - 实现账龄分桶计算（0-30天/31-60天/61-90天/90天以上）
    - 实现对账单 PDF 生成
  - [ ] 8.3 新增 API 端点
    - `POST /api/v1/erp/ar-reconciliations/auto-match` - 自动对账
    - `GET /api/v1/erp/ar-reconciliations/aging-report` - 账龄分析
    - `GET /api/v1/erp/ar-reconciliations/:id/details` - 对账明细
    - `POST /api/v1/erp/ar-reconciliations/:id/confirm` - 客户确认
    - `POST /api/v1/erp/ar-reconciliations/:id/dispute` - 争议处理
  - [ ] 8.4 前端 - 应收对账页面完善
    - 自动对账按钮和进度显示
    - 账龄分析图表（ECharts 堆叠柱状图）
    - 对账明细展开行
    - 客户确认/争议操作按钮
  - [ ] 8.5 创建 API 文件 `frontend/src/api/ar-reconciliation-enhanced.ts`

- [ ] 9. 多币种支持完善
  - [ ] 9.1 数据库迁移 - 添加币种字段
    - 为 `sales_orders` 表添加 `currency_code`, `exchange_rate` 字段
    - 为 `purchase_orders` 表添加 `currency_code`, `exchange_rate` 字段
    - 为 `ar_invoices` 表添加 `currency_code`, `exchange_rate`, `base_amount` 字段
    - 为 `ap_invoices` 表添加 `currency_code`, `exchange_rate`, `base_amount` 字段
  - [ ] 9.2 扩展 `currency_service.rs`
    - 实现本位币自动换算逻辑
    - 实现汇率历史查询
    - 实现外部汇率 API 集成（定时更新）
  - [ ] 9.3 扩展销售/采购/发票 handler
    - 销售订单创建时支持币种参数
    - 采购订单创建时支持币种参数
    - 发票金额自动换算为本位币
  - [ ] 9.4 前端 - 多币种支持
    - 订单页面添加币种选择下拉框
    - 发票页面显示原币和本位币金额
    - 汇率管理页面添加汇率走势图（ECharts 折线图）
  - [ ] 9.5 创建 API 文件 `frontend/src/api/currency-enhanced.ts`

---

## 第三阶段：报表引擎 + 数据导入导出

- [ ] 10. 报表引擎增强
  - [ ] 10.1 扩展 `report_engine_service.rs`
    - 实现动态报表模板创建（用户自定义列、筛选条件）
    - 实现 PDF 导出（使用 wkhtmltopdf 或 weasyprint）
    - 实现 Excel 导出（使用 xlsxwriter 或 calamine）
    - 实现报表订阅和定时发送
  - [ ] 10.2 扩展 `report_engine_handler.rs`
    - `POST /api/v1/erp/reports/templates` - 创建自定义模板
    - `GET /api/v1/erp/reports/templates` - 模板列表
    - `POST /api/v1/erp/reports/export/pdf` - PDF 导出
    - `POST /api/v1/erp/reports/export/excel` - Excel 导出
    - `POST /api/v1/erp/reports/subscriptions` - 创建订阅
    - `GET /api/v1/erp/reports/subscriptions` - 订阅列表
  - [ ] 10.3 前端 - 报表引擎页面完善
    - 报表设计器组件（拖拽式列配置）
    - 报表预览和导出按钮
    - 图表展示组件（ECharts）
    - 报表订阅管理
  - [ ] 10.4 创建 API 文件 `frontend/src/api/report-enhanced.ts`

- [ ] 11. 数据导入导出增强
  - [ ] 11.1 创建 `import_export_service.rs`
    - 实现通用 CSV 导入框架（模板下载、数据校验、批量导入）
    - 实现通用 Excel 导入框架
    - 实现数据迁移工具（表间数据转移）
  - [ ] 11.2 创建 `import_export_handler.rs`
    - `POST /api/v1/erp/import/csv` - CSV 导入
    - `POST /api/v1/erp/import/excel` - Excel 导入
    - `GET /api/v1/erp/import/templates/:type` - 下载导入模板
    - `GET /api/v1/erp/export/csv/:type` - CSV 导出
    - `GET /api/v1/erp/export/excel/:type` - Excel 导出
  - [ ] 11.3 前端 - 导入导出组件
    - 通用导入组件（文件上传 + 预览 + 确认）
    - 导出按钮组件
    - 导入模板下载链接
  - [ ] 11.4 创建 API 文件 `frontend/src/api/import-export.ts`

---

## 第四阶段：AI 智能分析真实化

- [ ] 12. 替换 advanced_handler.rs 假数据
  - [ ] 12.1 重写销售预测服务
    - 使用简单时间序列算法（移动平均 + 指数平滑）
    - 基于历史销售数据计算预测值
    - 实现准确率评估（MAPE 计算）
  - [ ] 12.2 重写库存优化服务
    - 基于历史出库数据计算安全库存
    - 实现 ABC 分类分析
    - 实现库存周转率计算
  - [ ] 12.3 重写异常检测服务
    - 基于统计方法检测异常值（Z-score / IQR）
    - 实现销售异常检测（突增/突降）
    - 实现库存异常检测（积压/短缺）
  - [ ] 12.4 重写智能推荐服务
    - 基于关联规则的采购推荐
    - 基于销售趋势的产品推荐
  - [ ] 12.5 前端 - AI 分析页面完善
    - 销售预测趋势图（ECharts 折线图 + 预测区间）
    - 库存优化建议列表
    - 异常数据标记和高亮
    - 推荐结果卡片

---

## 第五阶段：BPM 流程 + CRM 客户关系增强

- [ ] 13. BPM 流程管理增强
  - [ ] 13.1 创建 `bpm_definition_handler.rs`
    - 实现流程定义 CRUD
    - 实现流程版本管理
    - 实现流程模板库
  - [ ] 13.2 扩展 `bpm_service.rs`
    - 实现流程定义解析（节点、连线、条件）
    - 实现条件分支评估
    - 实现并行审批
    - 实现超时自动处理
  - [ ] 13.3 前端 - BPM 页面完善
    - 流程设计器（使用 bpmn.js 或自定义拖拽组件）
    - 流程定义列表
    - 流程模板库
    - 流程监控仪表盘

- [ ] 14. CRM 客户关系增强
  - [ ] 14.1 扩展 `crm_handler.rs` - 客户信息维护
    - 实现客户完整信息管理（基本信息/联系方式/开票信息/收货地址）
    - 实现客户标签管理（行业/规模/来源/自定义标签）
    - 实现客户联系人管理（多联系人支持）
    - 实现客户附件管理（合同/资质/照片等）
  - [ ] 14.2 创建 `crm_customer_pool_handler.rs` - 客户公海
    - 实现公海客户池（未分配/回收的客户）
    - 实现公海领取规则（每人上限/领取冷却期/自动回收）
    - 实现客户回收机制（超期未跟进自动回收到公海）
    - 实现公海客户搜索和筛选
  - [ ] 14.3 创建 `crm_assignment_handler.rs` - 客户分配
    - 实现手动分配（管理员指定销售员）
    - 实现自动分配（轮询/权重/区域规则）
    - 实现批量分配
    - 实现分配历史记录
  - [ ] 14.4 扩展 `crm_handler.rs` - 客户新增流程
    - 实现客户新增表单（必填/选填字段校验）
    - 实现客户查重（名称/手机/邮箱去重）
    - 实现客户新增审批（可选）
    - 实现从线索转化创建客户
  - [ ] 14.5 扩展 `crm_handler.rs` - 客户 360 视图与分级
    - 实现客户 360 视图（聚合订单/发票/跟进/商机数据）
    - 实现跟进记录管理
    - 实现客户自动分级（基于 RFM 模型：交易金额/频次/最近交易时间）
    - 实现客户价值评分
    - 实现客户生命周期分析
  - [ ] 14.6 扩展 `crm_service.rs`
    - 实现客户数据聚合查询（订单总额/发票余额/跟进次数）
    - 实现公海回收定时任务
    - 实现客户分配规则引擎
  - [ ] 14.7 前端 - CRM 页面完善
    - 客户列表页增强（新增按钮/批量导入/高级筛选）
    - 客户新增/编辑表单页
    - 客户公海页面（领取/归还操作）
    - 客户分配页面（手动/自动分配面板）
    - 客户 360 视图页面（多 Tab 布局：概览/订单/发票/跟进/商机）
    - 跟进记录时间轴组件
    - 客户分级标签和评分展示
    - 商机漏斗图（ECharts 漏斗图）
  - [ ] 14.8 创建/更新 API 文件
    - `frontend/src/api/crm-customer.ts` - 客户信息维护
    - `frontend/src/api/crm-pool.ts` - 客户公海
    - `frontend/src/api/crm-assignment.ts` - 客户分配
    - `frontend/src/api/crm-follow-up.ts` - 跟进记录

---

## 第六阶段：系统级功能完善

- [ ] 15. 安全与权限增强
  - [ ] 15.1 实现字段级权限控制
    - 扩展权限中间件，支持字段级过滤
    - 在 handler 层根据权限返回/隐藏字段
  - [ ] 15.2 实现操作日志审计增强
    - 扩展 `operation_log_service.rs` 记录所有关键操作
    - 实现审计日志查询和导出
  - [ ] 15.3 实现登录安全策略
    - 登录失败锁定（连续 N 次失败后锁定账号）
    - 异地登录检测
    - 登录日志记录
  - [ ] 15.4 前端 - 安全管理页面
    - 操作日志查询页面
    - 登录日志查询页面
    - 安全策略配置页面

- [ ] 16. 消息通知增强
  - [ ] 16.1 扩展 `email_service.rs`
    - 实现邮件模板管理
    - 实现邮件发送队列
    - 实现邮件发送记录
  - [ ] 16.2 创建 `webhook_integration_service.rs`
    - 实现企业微信 Webhook 集成
    - 实现钉钉 Webhook 集成
    - 实现通用 Webhook 回调
  - [ ] 16.3 前端 - 通知设置页面完善
    - 邮件配置界面
    - Webhook 配置界面
    - 通知模板编辑器

---

## 第七阶段：多租户 SaaS 完善

- [ ] 17. 多租户数据隔离
  - [ ] 17.1 实现行级数据隔离
    - 在所有查询中添加 tenant_id 过滤
    - 创建数据隔离中间件
    - 实现租户上下文传递
  - [ ] 17.2 实现租户权限体系
    - 租户内角色管理
    - 租户内用户管理
    - 租户级功能开关
  - [ ] 17.3 实现租户计费系统
    - 计费套餐定义（免费版/基础版/专业版）
    - 用量统计（用户数/存储量/API 调用次数）
    - 账单生成
  - [ ] 17.4 实现租户自助注册
    - 注册流程（邮箱验证/手机验证）
    - 试用期管理
    - 套餐升级/降级
  - [ ] 17.5 前端 - 租户管理完善
    - 租户配置界面
    - 计费管理页面
    - 套餐选择页面
    - 租户自助注册页面

---

## 第八阶段：前端交互体验增强

- [ ] 18. 数据可视化增强
  - [ ] 18.1 创建通用 ECharts 组件库
    - 折线图组件
    - 柱状图组件
    - 饼图/环形图组件
    - 漏斗图组件
    - 仪表盘组件
    - 热力图组件
  - [ ] 18.2 在关键页面集成图表
    - Dashboard 添加销售趋势、库存周转、应收应付等图表
    - 销售分析页面添加销售漏斗、区域分布等图表
    - 库存页面添加库存分布、周转率等图表
    - 财务页面添加现金流、利润趋势等图表

- [ ] 19. 批量操作增强
  - [ ] 19.1 创建通用批量操作组件
    - 批量选择（全选/反选）
    - 批量删除确认
    - 批量状态变更
    - 批量导出
  - [ ] 19.2 在关键列表页面集成批量操作
    - 订单列表批量审批
    - 库存列表批量调整
    - 客户列表批量分级

- [ ] 20. 高级筛选增强
  - [ ] 20.1 创建通用高级筛选组件
    - 多条件组合筛选（AND/OR）
    - 日期范围选择
    - 数值范围输入
    - 筛选条件保存/加载
  - [ ] 20.2 在关键列表页面集成高级筛选

- [ ] 21. 拖拽排序
  - [ ] 21.1 创建通用拖拽排序组件
    - 表格行拖拽排序
    - 列表项拖拽排序
    - 树形结构拖拽
  - [ ] 21.2 在需要排序的页面集成拖拽功能
    - BOM 物料顺序
    - 生产工序顺序
    - 看板卡片顺序

---

## 检查点

- [ ] CP1. 第一阶段完成检查 - MRP 模块全部功能可用
- [ ] CP2. 第二阶段完成检查 - 应收对账和多币种功能可用
- [ ] CP3. 第三阶段完成检查 - 报表引擎和导入导出功能可用
- [ ] CP4. 第四阶段完成检查 - AI 分析使用真实数据
- [ ] CP5. 第五阶段完成检查 - BPM 和 CRM 功能完善
- [ ] CP6. 第六阶段完成检查 - 系统级功能完善
- [ ] CP7. 第七阶段完成检查 - 多租户功能可用
- [ ] CP8. 第八阶段完成检查 - 前端交互体验提升
- [ ] CP9. 全面回归测试 - 所有模块功能正常
- [ ] CP10. 性能测试 - 关键接口响应时间 < 500ms

---

## 技术依赖

### 后端新增依赖
- `calamine` - Excel 读取
- `xlsxwriter` - Excel 写入
- `weasyprint` 或 `wkhtmltopdf` - PDF 生成
- `reqwest` - 外部 API 调用（汇率等）
- `cron` - 定时任务调度

### 前端新增依赖
- `echarts` - 数据可视化
- `dhtmlx-gantt` 或 `@vue-gantt` - 甘特图组件
- `bpmn-js` - BPMN 流程设计器（可选）
- `xlsx` - Excel 处理
- `vuedraggable` - 拖拽排序

### 数据库新增表
- `boms` - BOM 主表
- `bom_items` - BOM 明细
- `work_centers` - 工作中心
- `capacity_records` - 产能记录
- `scheduling_tasks` - 排程任务
- `mrp_results` - MRP 计算结果
- `mrp_requirements` - MRP 物料需求
- `report_templates` - 报表模板
- `report_subscriptions` - 报表订阅
- `import_records` - 导入记录
- `email_templates` - 邮件模板
- `email_logs` - 邮件发送记录
- `tenant_configs` - 租户配置
- `billing_plans` - 计费套餐
- `usage_records` - 用量记录
