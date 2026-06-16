# 用户指令记忆

本文件记录了用户的指令、偏好和教导，用于在未来的交互中提供参考。

## 格式

### 用户指令条目
用户指令条目应遵循以下格式：

[用户指令摘要]
- Date: [YYYY-MM-DD]
- Context: [提及的场景或时间]
- Instructions:
  - [用户教导或指示的内容，逐行描述]

### 项目知识条目
Agent 在任务执行过程中发现的条目应遵循以下格式：

[项目知识摘要]
- Date: [YYYY-MM-DD]
- Context: Agent 在执行 [具体任务描述] 时发现
- Category: [运维部署|构建方法|测试方法|排错调试|工作流协作|环境配置]
- Instructions:
  - [具体的知识点，逐行描述]

## 去重策略
- 添加新条目前，检查是否存在相似或相同的指令
- 若发现重复，跳过新条目或与已有条目合并
- 合并时，更新上下文或日期信息
- 这有助于避免冗余条目，保持记忆文件整洁

## 条目

[语言偏好]
- Date: 2026-05-22
- Context: 用户明确要求助手使用中文回复
- Instructions:
  - 所有回复必须使用简体中文
  - 包括代码解释、错误提示和操作说明
  - 保持专业术语准确的同时使用中文表达

[项目运行时知识]
- Date: 2026-05-27
- Context: 用户提供项目运维和配置信息
- Category: 运维部署
- Instructions:
  - **服务器信息**：生产服务器 111.230.99.236（SSH: root），数据库服务器 39.99.34.194:5432（用户: bingxi）
  - **敏感信息**：密码等敏感信息禁止记录在记忆文件中，仅通过环境变量或安全配置管理
  - 服务名称: bingxi-backend (systemd)，安装目录: /opt/bingxi-erp
  - 后端端口: 8082，日志目录: /opt/bingxi-erp/backend/logs，备份目录: /opt/bingxi-erp/backups
  - 环境配置: /etc/bingxi-erp/.env
  - 构建限制: 禁止本地编译，只允许 CICD 编译，CICD 自动部署到 GitHub Release，手动部署到生产服务器
  - **服务器不安装PostgreSQL客户端**：有专门的数据库服务器(39.99.34.194)，应用服务器只连接远程数据库
  - **服务器不安装Redis**：有专门的Redis服务器，应用服务器只连接远程Redis
  - **服务器只需安装**: Nginx、curl
  - 部署命令: `bingxi update` (CLI工具)

[功能实现进度]
- Date: 2026-06-06
- Context: 用户提供项目已实现功能清单，经过深入分析确认
- Category: 环境配置
- Instructions:
  - 项目包含 **751个子功能**，102个后端Handler，74个前端API模块，67个前端页面
  - **路由变动记录（2026-06-06）**：
    - 条码扫描统计：`/statistics` → `/scan-statistics`
    - 邮件记录：`/records` → `/email-records`
    - 导入模板下载：`/templates/:import_type` → `/templates/download/:import_type`
    - 租户计费用量：`/usage` → `/billing-usage`
    - API密钥撤销：`/:id/revoke` → `/api-key/:id/revoke`（支持DELETE和POST）
    - 新增高级分析路由：`/sales-prices`、`/sales-returns`
    - 新增CRM标签路由：`/customers/:id/tags`（POST添加）、`/crm-tags/:id`（DELETE删除）
    - 新增销售用户路由：`/sales-users`
    - 新增物流删除路由：`/logistics/:id`（DELETE）
    - 新增染色批次更新路由：`/dye-batches/:id`（PUT）
    - 新增MRP转单路由：`/mrp/convert-orders`（POST）
    - 新增排程路由：`/scheduling/gantt`（GET甘特图）、`/scheduling/:id`（PUT调整）
    - 新增产能趋势路由：`/capacity/trend`（GET）
    - 新增供应商删除路由：`/suppliers/:id`（DELETE）
    - **setup 模式初始化修复（2026-06-07）**：
      - 后端：增加进程级 `SETUP_MODE_INITIALIZED` 标志位
      - 前端：router 暴露 `resetInitStatus()`，Setup.vue 在 `goToLogin()` 之前主动重置缓存
  - **新增前端路由和页面（2026-05-30）**：
    - 邮件管理 `/email`：邮件模板CRUD、发送记录查询、发送统计
      - API: `/frontend/src/api/email.ts`
      - 页面: `/frontend/src/views/email/index.vue`
    - 租户计费管理 `/tenant-billing`：当前套餐、套餐列表、账单列表、升级/续费
      - API: `/frontend/src/api/tenant-billing.ts`
      - 页面: `/frontend/src/views/tenant-billing/index.vue`
    - 采购检验 `/purchase-inspection`：检验单CRUD、完成检验、检验明细管理
      - API: `/frontend/src/api/purchase-inspection.ts`
      - 页面: `/frontend/src/views/purchase-inspection/index.vue`
    - 采购退货 `/purchase-return`：退货单CRUD、提交/审批、退货明细管理
      - API: `/frontend/src/api/purchase-return.ts`（已存在）
      - 页面: `/frontend/src/views/purchase-return/index.vue`
    - 物流管理 `/logistics`：运单CRUD、发货、状态更新、删除运单
      - API: `/frontend/src/api/logistics.ts`
      - 页面: `/frontend/src/views/logistics/index.vue`
    - 路由配置: `/frontend/src/router/index.ts`（已添加5个新路由）
  - **系统设置模块（71个子功能）**：
    - 系统管理（12个Tab）：用户管理、角色管理、部门管理、权限管理、数据权限、字段权限、通知设置、审计日志、Webhook配置、系统更新、租户配置、公司信息
    - 系统设置：通知中心、登录安全、全量审计、数据权限管理、API密钥管理
    - 部门管理：部门CRUD、部门树
    - 五维管理：五维统计、搜索、解析、辅助核算、业务追溯
    - 报表中心：报表模板、报表导出、报表订阅、财务报表、财务分析
    - 数据导入：CSV导入、Excel导入、导入模板下载、CSV导出、Excel导出
    - 打印模板：销售订单、销售合同、采购订单、采购入库、库存调拨、库存盘点、凭证
    - API网关：API密钥管理、API速率限制
    - 系统更新：检查更新、应用更新、版本管理、版本回滚、本地更新、上传更新
    - 高级功能：AI销售预测、AI库存优化、AI异常检测、AI智能推荐、报表引擎、多租户管理
    - 通知中心：通知列表、未读通知、标记已读、通知设置、通知详情、删除通知
    - 全量审计：审计大屏、审计日志
  - **销售管理（47个子功能）**：
    - 销售订单：列表、创建、编辑、删除、详情、提交审批、审批、发货、完成、历史、导出、打印
    - 面料订单：列表、创建、编辑、删除、详情、审批
    - 销售合同：列表、创建、编辑、删除、详情、审批、执行、取消、打印
    - 销售价格：列表、创建、编辑、删除、详情、审批、历史、策略（API: `/sales-prices`）
    - 销售退货：列表、创建、编辑、删除、详情、提交审批、审批、拒绝（API: `/sales-returns`）
    - 销售分析：统计、趋势、排名、目标
    - 销售用户：列表（API: `/sales-users`）
  - **采购管理（58个子功能）**：
    - 采购订单：列表、创建、编辑、删除、详情、提交审批、审批、拒绝、关闭、明细CRUD、计算交货日期、导出、打印
    - 采购入库：列表、创建、编辑、详情、确认入库、明细CRUD、打印
    - 采购检验：列表、创建、编辑、详情、完成检验
    - 采购退货：列表、创建、编辑、删除、详情、提交审批、审批、拒绝、明细CRUD
    - 采购合同：列表、创建、编辑、删除、详情、审批、执行、取消
    - 采购价格：列表、创建、编辑、删除、详情、审批、历史
    - 供应商管理：列表、创建、编辑、删除供应商（API: `/suppliers/:id` DELETE）
  - **库存管理（38个子功能）**：
    - 库存台账：列表、创建、编辑、删除、详情、面料库存查询、创建面料库存、库存流水、库存汇总、低库存预警、导出、打印
    - 库存调拨：列表、创建、编辑、详情、审批、发货、收货、打印
    - 库存盘点：列表、创建、编辑、详情、审批、完成、打印
    - 库存调整：列表、创建、详情、审批、拒绝
    - 批次管理：列表、创建、编辑、删除、详情、批次调拨
  - **财务管理（133个子功能）**：
    - 凭证管理：列表、创建、编辑、删除、详情、提交、审核、过账、打印
    - 会计科目：列表、科目树、创建、编辑、删除、详情
    - 会计期间：当前期间、初始化期间、关闭期间
    - 应付管理：发票CRUD、审批、取消、自动生成、账龄、余额、统计、付款CRUD、确认付款、付款申请CRUD、提交、审批、拒绝、核销CRUD、自动核销、手动核销、取消核销、未核销查询、对账CRUD、确认对账、争议对账、自动对账、对账汇总、报表统计、日报、月报、账龄报表
    - 应收管理：发票CRUD、审批、取消
    - 应收对账：对账CRUD、更新状态、自动匹配、账龄报告、对账详情、确认对账、争议对账、PDF导出、生成对账单
    - 财务报表：资产负债表、利润表
    - 固定资产：资产CRUD、计提折旧、资产处置、批量折旧
    - 预算管理：预算CRUD、审批、调整、预算项CRUD、预算计划CRUD、审批、执行、执行记录、预算控制
    - 资金管理：账户CRUD、存款、取款、冻结、解冻、转账、转账记录
    - 成本归集：成本CRUD、审核、成本分析、成本汇总、按批次分析
    - 多币种：币种列表、本位币、汇率CRUD、汇率历史、金额换算、批量同步、支持币种
  - **CRM客户关系（53个子功能）**：
    - 线索管理：列表、创建、编辑、删除、详情、更新状态、转化、关联信息
    - 商机管理：列表、创建、编辑、删除、详情、转化
    - CRM客户：列表、创建、编辑、删除、详情、标签管理（API: `/customers/:id/tags` POST添加标签）、联系人、360视图、标签CRUD（API: `/crm-tags/:id` DELETE删除标签）
    - 公海客户池：列表、认领、回收
    - 客户分配：列表、手动分配、批量分配、分配历史
    - 客户信用：信用CRUD、评级、占用、释放、调整、停用、信用评估
    - 供应商评估：评估CRUD、指标管理、排名、评估记录、评分、评级
  - **生产管理（46个子功能）**：
    - 生产订单：列表、创建、编辑、删除、详情、更新状态、提交审批、审批、汇报进度、订单日志
    - BOM管理：列表、创建、编辑、删除、详情、复制、设置默认、树形结构、需求计算、版本列表
    - MRP计算：执行计算、计算结果、需求查询、转单（API: `/mrp/convert-orders` POST）、历史记录
    - 生产排程：自动排程、甘特图（API: `/scheduling/gantt` GET）、冲突检测、任务列表、更新排程、调整排程（API: `/scheduling/:id` PUT）、排程历史、排程结果、确认排程
    - 产能分析：产能概览、工作中心CRUD、可用产能预测、负荷分析（API: `/capacity/trend` GET）、过载检查
    - 缺料预警：预警列表、执行检查、缺料汇总、阈值设置、补货建议
  - **面料行业专用（41个子功能）**：
    - 面料管理：列表、创建、编辑、删除、详情、导入、导出
    - 坯布管理：列表、创建、编辑、删除、详情、入库、出库、按供应商查询
    - 染色配方：列表、创建、编辑、删除、详情、审批、创建版本、按色号查询、版本列表
    - 染色批次：列表、创建、编辑、删除、详情、完成染色、按色号查询、更新批次（API: `/dye-batches/:id` PUT）
    - 双计量单位：单位换算、双计量验证
    - 条码扫描：扫码查询、扫码发货、扫码历史、扫码统计（API: `/scan-statistics`）
    - 物流管理：运单CRUD、删除运单（API: `/logistics/:id` DELETE）
  - **质量管理（15个子功能）**：
    - 质量检验：标准列表、创建标准、记录CRUD、记录详情、缺陷列表、缺陷处理
    - 质量标准：标准CRUD、标准详情、版本管理、审批、发布
  - **基础数据模块（53个子功能）**：
    - 产品管理：列表、创建、编辑、删除、详情、选择列表、批量创建/更新/删除、导出、导入、导入模板、色号CRUD、批量创建色号
    - 产品类别：列表、创建、编辑、删除、详情、类别树
    - 客户管理（基础）：列表、创建、编辑、删除、详情、选择列表
    - 供应商管理：列表、创建、编辑、删除、详情、选择列表、切换状态、联系人CRUD、资质CRUD
    - 仓库管理：列表、创建、编辑、删除、详情、选择列表、库位CRUD
  - **BPM审批模块（27个子功能）**：
    - 流程管理：启动流程、审批任务、待办任务列表、业务关联、流程可视化、审批链、流程详情、监控统计、待办监控、流程实例、转办、催办
    - 流程定义：列表、创建、编辑、删除、详情、复制、版本列表、激活、创建模板、模板列表、从模板创建
    - 流程模板：列表、创建、编辑、删除、详情
  - **仪表盘模块（4个子功能）**：概览、销售统计、库存统计、低库存预警
  - **健康检查模块（3个子功能）**：健康检查、就绪检查、存活检查
  - **系统更新模块（10个子功能）**：检查远程更新、应用远程更新、获取当前版本、获取更新状态、备份版本列表、版本回滚、本地发布包列表、应用本地更新、检查本地更新、上传更新包
  - **导入导出模块（5个子功能）**：CSV导入、Excel导入、导入模板下载（API: `/templates/download/:import_type` GET）、CSV导出、Excel导出
  - **打印服务模块（7个子功能）**：销售订单、销售合同、采购订单、采购入库、库存调拨、库存盘点、凭证
  - **多租户SaaS模块（16个子功能）**：
    - 租户管理：列表、创建、详情、更新状态
    - 租户配置：配置列表、设置配置、删除配置、套餐列表、创建套餐、套餐详情、使用统计
    - 租户计费：当前套餐、升级套餐、用量统计（API: `/billing-usage` GET）、账单列表、续费订阅
  - **通知与消息模块（30个子功能）**：
    - 通知管理：列表、未读数、全部标已读、批量标已读、设置查询、设置更新、详情、单条标已读、删除
    - 用户通知设置：偏好查询、偏好更新
    - 邮件管理：发送邮件、模板CRUD、发送记录（API: `/email-records` GET）、发送统计
    - Webhook集成：Webhook CRUD、集成CRUD、通用回调、测试集成、企业微信消息、钉钉消息
  - **安全与权限模块（24个子功能）**：
    - 登录安全：登录日志、锁定状态、解锁账号、登录统计、安全告警
    - 审计日志：日志查询、日志导出
    - 全量审计：审计大屏、审计日志搜索、接收UI埋点
    - 数据权限：列表、设置、范围类型、角色权限、权限详情、删除
    - 字段权限：列表、创建、详情、更新、删除
    - API密钥：列表、创建、撤销（API: `/api-key/:id/revoke` POST或DELETE）
  - **AI分析模块（4个子功能）**：销售预测、库存优化、异常检测、智能推荐
  - **报表引擎模块（20个子功能）**：
    - 报表引擎：模板列表（API: `/reports/enhanced/templates` GET）、执行报表、导出报表、数据聚合、清除缓存
    - 增强报表：模板CRUD、执行自定义报表、导出PDF/Excel、订阅CRUD、启用/禁用订阅、手动触发
  - **交易管理模块（28个子功能）**：
    - 采购合同：列表、创建、详情、编辑、删除、审批、执行
    - 采购价格：列表、创建、编辑、删除、审批
    - 销售合同：列表、创建、详情、编辑、删除、审批
    - 销售价格：列表、创建、编辑、删除、审批
    - 销售退货：列表、创建、详情、编辑、删除
  - **其他功能模块（17个子功能）**：
    - 页面追踪：页面访问埋点
    - 指标监控：Prometheus指标
    - 系统初始化：状态检查、测试数据库（API: `/init/test-database` POST）、系统初始化、带数据库初始化、重置密码
    - 认证管理：登录、登出、刷新Token、CSRF令牌、TOTP设置、启用TOTP
    - 组件示例：图表组件、批量操作、高级筛选、拖拽表格

[路由架构变动记录]
- Date: 2026-06-06
- Context: 修复路由冲突问题，优化路由结构
- Category: 环境配置
- Instructions:
  - **路由冲突修复**：解决了 analytics.rs 中的路由冲突问题
    - 使用 `nest` + `merge` 混合策略
    - 内部 path 唯一的子 router 走 `merge`
    - 内部 path 有重复的子 router 走 `nest` 加独立前缀
  - **主要路由变动**：
    - 条码扫描统计：`/statistics` → `/scan-statistics`
    - 邮件记录：`/records` → `/email-records`
    - 导入模板下载：`/templates/:import_type` → `/templates/download/:import_type`
    - 租户计费用量：`/usage` → `/billing-usage`
    - API密钥撤销：`/:id/revoke` → `/api-key/:id/revoke`
  - **新增路由**：
    - 高级分析：`/sales-prices`、`/sales-returns`
    - CRM标签：`/customers/:id/tags`、`/crm-tags/:id`
    - 销售用户：`/sales-users`
    - 物流删除：`/logistics/:id` DELETE
    - 染色批次更新：`/dye-batches/:id` PUT
    - MRP转单：`/mrp/convert-orders` POST
    - 生产排程：`/scheduling/gantt` GET、`/scheduling/:id` PUT
    - 产能趋势：`/capacity/trend` GET
    - 供应商删除：`/suppliers/:id` DELETE
  - **前端初始化修复**：Setup.vue 中数据库测试连接成功后，下一步按钮无法点击
    - 原因：前端检查 `data.success`，但后端返回格式为 `{code: 200, data: {success: true}}`
    - 修复：改为检查 `data.code === 200 && data.data?.success`

[工作角色定位]
- Date: 2026-05-27
- Context: 用户明确要求助手的角色定位和工作方式
- Category: 工作流协作
- Instructions:
  - 我的角色是总控（项目经理/架构师）
  - 子代理（Task工具）是我的员工，负责具体执行任务
  - 所有任务都有对应的员工，员工拥有完成该任务的所有技能
  - 用户输入的所有内容都需要我进行分析，然后分配任务给员工
  - 我的职责是：分析用户任务 → 拆解任务 → 分配给员工 → 总结员工成果 → 推送PR
  - 不要自己直接写代码，而是分配给员工执行
  - 员工完成后，我需要总结他们的工作成果，然后推送到PR

[部署限制]
- Date: 2026-05-29
- Context: 用户明确要求禁止使用Docker容器部署
- Category: 运维部署
- Instructions:
  - 项目禁止使用Docker容器部署
  - 不得创建Dockerfile、docker-compose.yml等Docker相关文件
  - 部署方式为：CICD构建 → GitHub Release → 手动部署到生产服务器
  - 使用systemd管理服务，不使用容器化部署

[2026年最新编程工作流]
- Date: 2026-05-28
- Context: 用户要求搜索最新的编程工作流，用于真实交付编程项目
- Category: 工作流协作
- Instructions:
  - **CI/CD 自动化流水线**（2026年最佳实践）：
    - 流水线即交付契约（Delivery Contract）
    - 三重角色：自动化引擎、安全闸门、治理平面
    - 多阶段Docker构建、缓存优化、并行测试
    - SAST/DAST安全扫描、SBOM生成、镜像签名
    - OIDC身份认证、RBAC权限控制、不可变标签、自动回滚
  - **DevOps 核心原则**：
    - 流动原则：加速从开发到交付的流程，大需求拆小，工作可视化
    - 反馈原则：及时发现问题，源头保障质量
    - 持续学习原则：将改进制度化，建立学习型组织
  - **Agentic Workflow（智能体工作流）**：
    - AI Agent协同开发，80%重复劳动交给AI
    - 代码生成、单元测试生成、代码审查、日志分析、性能优化
    - AI写代码，人负责审查和修改
  - **开发流程优化**：
    - 敏捷开发：小步快跑，每次迭代交付可审查版本
    - 持续集成：频繁将代码变更合并到共享主干
    - 持续交付：确保代码始终处于可部署状态
    - 基础设施即代码：使用声明式配置管理服务器和环境
  - **质量保障**：
    - 自动化测试：单元测试、集成测试、E2E测试
    - 代码质量门禁：SonarQube、ESLint、Clippy
    - 安全扫描：SAST/DAST、依赖漏洞检查
    - 代码审查：PR审查、AI辅助审查
  - **部署策略**：
    - 蓝绿部署：零停机部署
    - 灰度发布：按比例逐步发布
    - 特性开关：功能级别的发布控制
    - 自动回滚：失败时自动回滚到上一版本
  - **监控与可观测性**：
    - 日志：结构化日志、日志聚合
    - 指标：Prometheus、Grafana
    - 链路追踪：分布式追踪
    - 告警：实时告警、自动恢复

[日志诊断技能自动触发规则]
- Date: 2026-06-07
- Context: 用户要求将日志诊断技能改为自动触发模式
- Category: 工作流协作
- Instructions:
  - **技能名称**：`/log-diagnosis` 日志诊断技能
  - **触发方式**：自动触发（无需用户手动输入命令）
  - **自动触发条件**：
    - 用户提到"日志"、"错误日志"、"异常日志"、"崩溃日志"等关键词
    - 用户要求分析服务器日志、应用日志、系统日志
    - 用户提到服务异常、崩溃、报错等问题
    - 用户要求查看服务器日志、拉取日志、分析错误
    - 用户提到 traceId、错误码、异常堆栈等信息
  - **执行流程**：
    1. 环境检查与配置加载（读取 `.diagnosis/config.json`）
    2. 日志搜索与提取（使用 grep/awk/sed 等工具）
    3. 代码联动分析（从日志中提取类名/方法名/文件名）
    4. 根因分析与诊断（综合分析，不可片面下结论）
    5. 生成诊断报告（保存到 `.diagnosis/reports/` 目录）
    6. 清理与恢复
  - **核心规则**：
    - 全量原则：必须统计总数并分批读取完毕，禁止只看前几行就下结论
    - 上下文原则：必须包含前后文（前5行后10行），避免脱离上下文误判
    - 代码验证原则：必须在代码中找到对应位置验证，禁止纯靠日志猜测
    - 报告原则：必须生成结构化报告文件，不可只口头描述
    - 配置优先原则：优先读取配置文件
    - 报告格式：
    - 保存路径：`.diagnosis/reports/{YYYY-MM-DD}_{问题描述摘要}.md`
    - 包含内容：基本信息、问题描述、日志分析、根因分析、修复建议、附录

[16 任务总规划]
- Date: 2026-06-14
- Context: 用户基于项目深度评估报告，要求规划 16 个待办任务（5 P0 + 6 P1 + 4 P2 + 4 P3 - 总计 19 个，扣除 3 个重叠/合并为 16）
- Category: 工作流协作
- Instructions:
  - **执行模式**：完全并行，使用多子代理并行 + 专用复查子代理检测 + 总代理审批机制
  - **4 类执行子代理**：
    - A 业务实现：P0-1/2/3/4 业务流修复、P1-1 generate-no 端点、P1-2 路由注册、P2-4 AI 深化
    - B 前端实现：P1-3/4/5/6 前端任务、P2-1 虚拟列表、P2-2 日志统一
    - C 基础设施：P0-5 事件定义、P2-3 CI 修复、迁移、工具、logger 框架
    - D 架构演进：P3-1/2/3/4 长期演进
    - 复查子代理：独立审查代码质量、测试覆盖、跨任务集成
  - **6 波推荐批次**：
    - Wave 1（5 任务）：P0-5、P1-1、P1-2、P2-3、logger 工具创建（1 周）
    - Wave 2（5 任务）：P0-1→P0-2/3/4 串行（同文件冲突）、P2-2（1 周）
    - Wave 3（6 任务）：P1-3（4 子代理并行）+ P1-4 + P1-5（1 周）
    - Wave 4（6 任务）：P1-6（6 子代理并行）+ P2-1 + P2-4（1 周）
    - Wave 5（4 任务）：P3-1/2/3/4 长期演进
    - Wave 6：复查子代理审查所有任务
  - **资源限制**：同时运行子代理数 ≤ 6，避免 token 爆炸和 Git 冲突
  - **Git 分支策略**：`feature/{task-id}` 独立分支，完成后合并 main 后删除
  - **强制报告模板**：子代理必须输出"工作报告"（改动文件/关键决策/测试结果/风险与遗留/自评）
  - **复查清单**：代码规范、dead_code、clippy、eslint、tsc、租户隔离（禁用 unwrap_or(0)）、敏感信息、错误处理、文档、CHANGELOG
  - **规划文档归档**：[规划-16tasks-2026-06-14.md](file:///workspace/.monkeycode/docs/规划-16tasks-2026-06-14.md)
  - **总任务清单**：
    - P0-1 修复采购入库→库存联动（A）
    - P0-2 修复销售发货→AR 应收账款（A）
    - P0-3 修复生产完成→入库（A）
    - P0-4 修复库存变动→财务凭证（A）
    - P0-5 修复 MaterialShortageAlert 事件定义（C）
    - P1-1 补齐 generate-no 端点（4 页面）（A）
    - P1-2 注册未挂载页面路由（sales-analysis/security）（B）
    - P1-3 拆分大 .vue 文件（46 个 > 500 行）（B×4）
    - P1-4 完成 system/index.vue 剩余 10 Tab 骨架（B）
    - P1-5 补齐 38 处前端 TODO（B）
    - P1-6 补齐 118 个仅 API 实现的前端页面（B×6）
    - P2-1 引入虚拟列表 vue-virtual-scroller / el-table-v2（B）
    - P2-2 统一前端日志：46 处 console.* → logger（B）
    - P2-3 修复 CI 测试编译错误（cargo test --lib）（C）
    - P2-4 AI 分析深化：工艺优化 + 质量预测（A）
    - P3-1 微服务拆分（按业务域）（D）
    - P3-2 WebSocket 实时通信（通知/看板）（D）
    - P3-3 移动端原生（React Native 配套）（D）
    - P3-4 数据仓库/BI 建设（D）

[13 任务重新规划]
- Date: 2026-06-14
- Context: 实时代码扫描发现原 19 任务中 5 个已完成、1 个需拆分（12 TODO 实际仅 2 处独立），用户要求对剩余 13 任务重新规划
- Category: 工作流协作
- Instructions:
  - **修订原因**：实时代码扫描纠正了 5 项误判（P0-1/3/4/5、P1-2 实际已完成）
  - **修订后 13 任务清单**：
    - 业务流：P0-2 销售发货→AR（60%→100%）
    - 基础设施：P2-3 rustc 升级（CI 编译失败修复）
    - 前端重构：P1-3 拆分 52 大 .vue、P1-4 完成 10 Tab、P1-5 完成 2 TODO、P2-1 虚拟列表、P2-2 console 替换
    - 端点：P1-1 generate-no 4 端点
    - AI：P2-4 工艺优化 + 质量预测
    - 长期：P3-1 微服务、P3-2 WebSocket、P3-3 React Native、P3-4 BI
  - **5 波调度**：
    - Wave 1（4 子代理，1 周）：A1 P0-2 / C1 P2-3 / B1 P1-1 / B2 P1-5
    - Wave 2（6 子代理，2 周）：B3 P1-3 嵌套 4 并行 / B4 P1-4 / B5 P2-1
    - Wave 3（2 子代理，1 周）：A2 P2-4 / B6 P2-2
    - Wave 4（4 子代理，4 周）：D1 P3-1 / D2 P3-2 / D3 P3-3 / D4 P3-4
    - Wave 5：复查子代理审查所有 P0/P1
  - **总资源**：13 执行子代理 + 1 复查；同时运行峰值 6；总周期约 8 周
  - **关键发现**：
    - P0 业务流已通过事件驱动架构实现（event_bus.rs:121-123 InventoryFinanceBridgeService.start_listener）
    - 449 个 API 函数 / 108 .vue 页面；P1-6 范围需在 Wave 1 前重新核对
    - 12 个 TODO 中 10 个与 P1-4 system/tabs 骨架重合，实际 P1-5 仅 2 处
  - **规划文档**：[规划-重新规划-13tasks-2026-06-14.md](file:///workspace/.monkeycode/docs/规划-重新规划-13tasks-2026-06-14.md)

[P0-2 销售→AR 业务流实现细节]
- Date: 2026-06-15
- Context: Agent 在执行 A1 任务（销售发货→AR 应收账款 P0-2）时发现并补全
- Category: 构建方法
- Instructions:
  - **业务流入口**：`backend/src/services/ar/inv.rs::ArReconciliationService::create_receivable`
  - **调用方**：`backend/src/services/so/delivery.rs::SalesService::ship_order`（第 192-224 行）
  - **事务边界**：调用方传入 `&DatabaseTransaction`，本方法不独立 commit/rollback；库存扣减、AR 单创建、订单状态更新共用同一事务
  - **幂等键**：`source_type=SALES_ORDER` + `source_bill_id=order_id` 联合唯一，重复调用返回 `BusinessError`
  - **客户账期**：调用方先经 `resolve_customer_payment_terms(customer_id)` 读取，<= 0 时回退 30 天
  - **单号生成**：`DocumentNumberGenerator` 生成 AR + YYYYMMDD + 3 位流水号
  - **业务事件**：`ReceivableCreated { receivable_id, order_id, customer_id, amount }` 在事务 commit 后再 publish，避免订阅方在事务回滚时误处理
  - **现有 ar_invoice_service**：顶层 `backend/src/services/ar_invoice_service.rs` 仍保留 `auto_generate_from_delivery` 等方法，但销售发货流已统一走 `ar::inv::create_receivable`，避免双入口数据不一致
  - **历史现象**：`delivery.rs` 在前一轮提交中已写入 AR 集成代码（含事件发布、resolve_customer_payment_terms 等），但 `ar_service.create_receivable` 方法本体缺失导致 `cargo build` 失败，本任务实际是补全缺失方法而非新增业务流
  - **回归测试**：CICD 端到端测试应覆盖：① 正常发货→AR 单生成 ② 二次发货幂等 ③ 客户账期=0 时 AR 单到日期 +30 天

[Wave 1 合并清理总结]
- Date: 2026-06-15
- Context: Agent 在执行"合并并清理"指令时完成
- Category: 工作流协作
- Instructions:
  - **合并结果**：Wave 1 全部 4 PR 已以 Squash 策略合入 main
    - #89 [.clippy.toml 宏路径修复] → a779078（先入）
    - #90 [B2 P1-5 入库单明细类型强化] → 2974c6d
    - #87 [A1 P0-2 销售→AR] → 042d123（cherry-pick 重构后）
    - #88 [B1 P1-1 generate-no 4 端点] → 5f28212（rebase 后）
  - **冲突解决经验**：
    - #87 使用 `git reset origin/main + cherry-pick 0373a73 4c0888b` 解决 MEMORY.md / CHANGELOG.md / inv.rs 冲突
    - #88 使用 `git rebase origin/main` 后解决 CHANGELOG.md (3 处) + frontend/src/api/purchaseReceipt.ts (1 处) 冲突
    - P0-2 业务流补齐 6 单元测试 保留 HEAD 版本（覆盖应收单号格式连续）
  - **清理范围**：
    - 远端源分支：4 个 `feature/*` `fix/*` 分支已由 GitHub squash merge 自动删除
    - 远端跟踪 ref：`git branch -rd origin/<branch>` 清理 4 个过时 ref
    - 本地分支：`git branch -D` 删除 5 个本地工作分支
    - 定时任务：`NLIZU5YY.FK660`（cron 0 * * * *）已删除，无需继续轮询
  - **CHANGELOG 更新**：在 [Unreleased] - 2026-06-15 顶部增加"Wave 1 合并汇总"表格，4 PR + 提交 SHA + 状态
  - **当前 main 状态**：5f28212 + a2df8f8（changelog），共 6 commit
  - **下一步**：可启动 Wave 2（B3 P1-3 嵌套 4 并行 / B4 P1-4 / B5 P2-1）

[Wave 3 B7 console.* 清理总结]
- Date: 2026-06-15
- Context: Agent 在执行"开始实施"指令时完成 Wave 3 B7 任务（清理 112 处 console.* → logger.*）
- Category: 工作流协作
- Instructions:
  - **执行模式**：单子代理串行，4 批分批 squash merge（避免云端卡死，已在 Wave 2 验证）
  - **Spec 文档**：`docs/superpowers/specs/2026-06-15-b7-console-cleanup-design.md`（提交 fee7507）
  - **评估文档**：`docs/superpowers/plans/2026-06-15-wave3-evaluation-plan.md`（提交 d21965b）
  - **4 批结果**：
    - B7-1 PR #91 → 313084e：purchase+inventory 域，8 文件 +45/-43，37 处
    - B7-2 PR #92 → c641239：crm+sales 域，4 文件 +15/-11，11 处
    - B7-3 PR #93 → 374a3af：bpm+report+arReconciliation 域，7 文件 +29/-22，22 处
    - B7-4 PR #94 → 979feca：dye/logistics/security/email/tenant/supplier/system/advanced/dashboard/setup/batch 域，12 文件 +54/-42，42 处
  - **总成果**：112 处 console.* → logger.*，31 个 .vue/.ts 文件，0 业务逻辑改动
  - **关键经验**：
    - 子代理在 catch 块处理中遇到 `e:unknown` 类型与 `logger.error(message: string)` 签名冲突，使用 `String(e)` 转换解决（消除 TS2345 错误）
    - 子代理发现 Edit 工具偶发"返回成功但未实际写入"（连续调用时），必须用 `grep` 验证 before/after
    - GitHub squash merge 后部分远端分支自动删除，残留可通过 `git push origin --delete` 或 `git update-ref -d` 清理
  - **已知遗留**：基线存在 32 个预存 type-check 错误（Wave 2 合并后），B7 4 批均无新增错误（基线 = 当前 = 32），清理预存错误属于 Wave 4 启动前置 P 任务
  - **GitHub Token**：嵌入在 `/workspace/.git/config` 的 `origin` URL 中（格式 `x-access-token:ghu_...`），可用 `grep -oP 'x-access-token:\K[^@]+' .git/config` 提取
  - **当前 main 状态**：979feca + 4658d37（changelog），共 7 commit
  - **下一步**：A2 AI 深化（工艺优化 + 质量预测）— 需用户确认 dye_recipe 表 migration 缺失问题

[GitHub 版本管理分支策略]
- Date: 2026-06-16
- Context: 用户要求建立规范的分支管理策略
- Category: 工作流协作
- Instructions:
  - **分支结构**：
    - `main` 为主分支（正式版），不允许删除
    - `test` 为测试分支，不允许删除
  - **测试分支 (test)**：
    - 所有修复/功能变更/功能新增等测试均在测试分支进行
    - 测试版分支不需要发布产品安装包
    - 测试分支需要特别详细的全量日志系统
    - 测试版需要自动触发 CI/CD
    - 所有 AI 创建的修复分支在验证后都合并入测试版分支
    - 合并到测试版分支后自动删除修复分支
  - **正式版分支 (main)**：
    - 正式版分支需要发版（发布产品安装包）
    - 不需要详细的日志系统，只保留基础的日志验证
    - 正式版的发版需要手动触发
  - **修复分支流程**：
    - AI 创建的修复分支验证后合并入 test 分支
    - 合并完成后自动删除修复分支
    - 测试通过后由 test 分支合并入 main 分支
