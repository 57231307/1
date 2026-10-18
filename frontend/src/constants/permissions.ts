/**
 * 前端权限码常量定义（单一真相源）
 *
 * Batch 462 P0-S24 修复：前后端权限边界一致性
 * - 命名风格与后端 init_service.rs 权限矩阵对齐：连字符复数（kebab-case + 复数）
 * - 后端权限校验通过 permission_middleware 基于 URL path 提取 resource_type
 * - 前端 v-permission 指令使用的权限码必须与此处常量完全一致
 *
 * 使用方式：
 *   import { PERMISSIONS } from '@/constants/permissions'
 *   <el-button v-permission="PERMISSIONS.USER_UPDATE">编辑</el-button>
 *
 * 命名规范：
 *   - 资源类型：连字符复数（如 users / sales-prices / sales-returns）
 *   - 动作：read / create / update / delete / approve / reject / cancel / export / print
 *     （取值与后端 middleware/permission.rs:227 PATH_ACTION_KEYWORDS 同源）
 *   - 常量名：大写下划线（如 USER_UPDATE 对应 'users:update'）
 */

// ============================================================================
// 通用动作常量
// ============================================================================
export const ACTION = {
  READ: 'read',
  CREATE: 'create',
  UPDATE: 'update',
  DELETE: 'delete',
  APPROVE: 'approve',
  REJECT: 'reject',
  CANCEL: 'cancel',
  EXPORT: 'export',
  IMPORT: 'import',
  PRINT: 'print',
} as const;

// ============================================================================
// 权限码常量（与后端 init_service.rs 权限矩阵对齐）
// ============================================================================
export const PERMISSIONS = {
  // 用户管理（后端资源：users）
  USER_READ: 'users:read',
  USER_CREATE: 'users:create',
  USER_UPDATE: 'users:update',
  USER_DELETE: 'users:delete',

  // 角色管理（后端资源：roles）
  ROLE_READ: 'roles:read',
  ROLE_CREATE: 'roles:create',
  ROLE_UPDATE: 'roles:update',
  ROLE_DELETE: 'roles:delete',

  // 部门管理（后端资源：departments）
  DEPARTMENT_READ: 'departments:read',
  DEPARTMENT_CREATE: 'departments:create',
  DEPARTMENT_UPDATE: 'departments:update',
  DEPARTMENT_DELETE: 'departments:delete',

  // 仓库管理（后端资源：warehouses）
  WAREHOUSE_READ: 'warehouses:read',
  WAREHOUSE_CREATE: 'warehouses:create',
  WAREHOUSE_UPDATE: 'warehouses:update',
  WAREHOUSE_DELETE: 'warehouses:delete',

  // 客户管理（后端资源：customers）
  CUSTOMER_READ: 'customers:read',
  CUSTOMER_CREATE: 'customers:create',
  CUSTOMER_UPDATE: 'customers:update',
  CUSTOMER_DELETE: 'customers:delete',
  // PII 按需揭示（POST /crm/customers/{id}/pii/reveal 的运行时键；
  // 每次成功揭示服务端强制留痕 pii_reveal_audit）
  CUSTOMER_PII_REVEAL: 'customers:reveal',

  // 供应商管理（后端资源：suppliers）
  SUPPLIER_READ: 'suppliers:read',
  SUPPLIER_CREATE: 'suppliers:create',
  SUPPLIER_UPDATE: 'suppliers:update',
  SUPPLIER_DELETE: 'suppliers:delete',

  // 产品管理（后端资源：products）
  PRODUCT_READ: 'products:read',
  PRODUCT_CREATE: 'products:create',
  PRODUCT_UPDATE: 'products:update',
  PRODUCT_DELETE: 'products:delete',

  // 销售报价（后端资源：quotations）
  QUOTATION_READ: 'quotations:read',
  QUOTATION_CREATE: 'quotations:create',
  QUOTATION_UPDATE: 'quotations:update',
  QUOTATION_DELETE: 'quotations:delete',
  QUOTATION_CANCEL: 'quotations:cancel',

  // 销售价格（后端资源：sales-prices）
  SALES_PRICE_READ: 'sales-prices:read',
  SALES_PRICE_CREATE: 'sales-prices:create',
  SALES_PRICE_UPDATE: 'sales-prices:update',
  SALES_PRICE_DELETE: 'sales-prices:delete',
  // 审批双动作：approve/reject 是两条独立端点（routes/sales.rs:196-203），权限键由 URL 段 +
  // 末段动作推导（middleware/permission.rs:227 PATH_ACTION_KEYWORDS 含 approve/reject），
  // 故拒绝入口必须用 reject 键，禁止拿 approve 键兼职放行。
  SALES_PRICE_APPROVE: 'sales-prices:approve',
  SALES_PRICE_REJECT: 'sales-prices:reject',

  // 销售退货（后端资源：sales-returns）
  SALES_RETURN_READ: 'sales-returns:read',
  SALES_RETURN_CREATE: 'sales-returns:create',
  SALES_RETURN_UPDATE: 'sales-returns:update',
  SALES_RETURN_DELETE: 'sales-returns:delete',
  SALES_RETURN_APPROVE: 'sales-returns:approve',

  // 销售合同（后端资源：sales-contracts）
  SALES_CONTRACT_READ: 'sales-contracts:read',
  SALES_CONTRACT_CREATE: 'sales-contracts:create',
  SALES_CONTRACT_UPDATE: 'sales-contracts:update',
  SALES_CONTRACT_DELETE: 'sales-contracts:delete',
  // 审批双动作端点：routes/sales.rs:154-161（approve / reject 各一条），两键不互替
  SALES_CONTRACT_APPROVE: 'sales-contracts:approve',
  SALES_CONTRACT_REJECT: 'sales-contracts:reject',

  // 采购收货（后端资源：purchase-receipts）的两个处置动作键：由 URL 末段派生
  // （POST /purchase/receipts/{id}/concession|rejudge），与建单/审批权互不覆盖；
  // 受授岗位集合见迁移 business/m0090_grant_purchase_receipt_concession_rejudge.rs
  PURCHASE_RECEIPT_CONCESSION: 'purchase-receipts:concession',
  PURCHASE_RECEIPT_REJUDGE: 'purchase-receipts:rejudge',

  // 收货确认键：真实端点是 POST /purchase/receipts/{id}/confirm（URL 末段派生 confirm），
  // 与建单权 create 分键；原先矩阵授的是 approve——该端点不存在，授权永不命中。
  // 受授岗位集合见迁移 business/m0091_realign_purchase_receipt_confirm.rs
  PURCHASE_RECEIPT_CONFIRM: 'purchase-receipts:confirm',

  // 采购合同（后端资源：purchase-contracts）
  PURCHASE_CONTRACT_READ: 'purchase-contracts:read',
  PURCHASE_CONTRACT_CREATE: 'purchase-contracts:create',
  PURCHASE_CONTRACT_UPDATE: 'purchase-contracts:update',
  PURCHASE_CONTRACT_DELETE: 'purchase-contracts:delete',
  // 审批双动作端点：routes/purchase.rs:252-259（approve / reject 各一条），两键不互替
  PURCHASE_CONTRACT_APPROVE: 'purchase-contracts:approve',
  PURCHASE_CONTRACT_REJECT: 'purchase-contracts:reject',

  // 采购价格（后端资源：purchase-prices）
  PURCHASE_PRICE_READ: 'purchase-prices:read',
  PURCHASE_PRICE_CREATE: 'purchase-prices:create',
  PURCHASE_PRICE_UPDATE: 'purchase-prices:update',
  PURCHASE_PRICE_DELETE: 'purchase-prices:delete',
  // 审批双动作端点：routes/purchase.rs:298-305（approve / reject 各一条），两键不互替
  PURCHASE_PRICE_APPROVE: 'purchase-prices:approve',
  PURCHASE_PRICE_REJECT: 'purchase-prices:reject',

  // 采购退货（后端资源：purchase-returns）
  PURCHASE_RETURN_READ: 'purchase-returns:read',
  PURCHASE_RETURN_CREATE: 'purchase-returns:create',
  PURCHASE_RETURN_UPDATE: 'purchase-returns:update',
  PURCHASE_RETURN_DELETE: 'purchase-returns:delete',

  // CRM 线索（后端资源：crm-leads）
  CRM_LEAD_READ: 'crm-leads:read',
  CRM_LEAD_CREATE: 'crm-leads:create',
  CRM_LEAD_UPDATE: 'crm-leads:update',
  CRM_LEAD_DELETE: 'crm-leads:delete',

  // CRM 商机（后端资源：crm-opportunities）
  CRM_OPPORTUNITY_READ: 'crm-opportunities:read',
  CRM_OPPORTUNITY_CREATE: 'crm-opportunities:create',
  CRM_OPPORTUNITY_UPDATE: 'crm-opportunities:update',
  CRM_OPPORTUNITY_DELETE: 'crm-opportunities:delete',

  // CRM 客户（后端资源：crm-customers）
  CRM_CUSTOMER_READ: 'crm-customers:read',
  CRM_CUSTOMER_CREATE: 'crm-customers:create',
  CRM_CUSTOMER_UPDATE: 'crm-customers:update',
  CRM_CUSTOMER_DELETE: 'crm-customers:delete',

  // 成本归集（后端资源：cost-collections）
  COST_COLLECTION_READ: 'cost-collections:read',
  COST_COLLECTION_CREATE: 'cost-collections:create',
  COST_COLLECTION_UPDATE: 'cost-collections:update',
  COST_COLLECTION_DELETE: 'cost-collections:delete',
  // 审核面的动作键由端点末段派生：挂载是 POST /production/cost-collections/{id}/audit，
  // 末段 audit 属后端 PATH_ACTION_KEYWORDS，通过或拒绝由请求体 approved 承载，
  // 因此两个入口共用 audit 键（没有 approve 端点，也就不存在 approve 键）。
  COST_COLLECTION_AUDIT: 'cost-collections:audit',
  // 导出面键由端点路径末段派生（GET /production/cost-collections/export），与读权限分键
  COST_COLLECTION_EXPORT: 'cost-collections:export',

  // 质量标准（后端资源：quality-standards）
  QUALITY_STANDARD_READ: 'quality-standards:read',
  QUALITY_STANDARD_CREATE: 'quality-standards:create',
  QUALITY_STANDARD_UPDATE: 'quality-standards:update',
  QUALITY_STANDARD_DELETE: 'quality-standards:delete',

  // 打印模板（后端资源：print-templates）
  PRINT_TEMPLATE_READ: 'print-templates:read',
  PRINT_TEMPLATE_CREATE: 'print-templates:create',
  PRINT_TEMPLATE_UPDATE: 'print-templates:update',
  PRINT_TEMPLATE_DELETE: 'print-templates:delete',

  // 打印目标（后端资源：print-targets）
  PRINT_TARGET_READ: 'print-targets:read',
  PRINT_TARGET_CREATE: 'print-targets:create',
  PRINT_TARGET_UPDATE: 'print-targets:update',
  PRINT_TARGET_DELETE: 'print-targets:delete',

  // 打印任务（后端资源：print-tasks）：发起一次真实打印与回查任务终态
  PRINT_TASK_READ: 'print-tasks:read',
  PRINT_TASK_CREATE: 'print-tasks:create',

  // 库存（后端资源：inventory）
  INVENTORY_READ: 'inventory:read',
  INVENTORY_CREATE: 'inventory:create',
  INVENTORY_UPDATE: 'inventory:update',
  INVENTORY_DELETE: 'inventory:delete',
  // 库存调拨动作（后端 PATH_ACTION_KEYWORDS 含 transfer）
  INVENTORY_TRANSFER: 'inventory:transfer',
  // 成品布入库标签打印（GET /inventory/pieces/{id}/print）。权限键非自造
  // resource_type 由 URL 段推导——middleware/permission.rs::extract_resource_info 对
  // 模块前缀 inventory（utils/path_utils.rs::is_business_module_prefix）取 segment4
  // "pieces"（path_utils.rs:102-117 resolve_module_prefixed_resource 默认分支，
  // 与 /inventory/transfers 取 "transfers" 同构）；action="print" 来自末段
  // PATH_ACTION_KEYWORDS（permission.rs:218-221）。登录出参权限码即
  // "{resource_type}:{action}"（handlers/auth_handler.rs:176）。
  INVENTORY_PIECE_PRINT: 'pieces:print',

  // 销售订单（后端资源：sales-orders）
  SALES_ORDER_READ: 'sales-orders:read',
  SALES_ORDER_CREATE: 'sales-orders:create',
  SALES_ORDER_UPDATE: 'sales-orders:update',
  SALES_ORDER_DELETE: 'sales-orders:delete',
  SALES_ORDER_APPROVE: 'sales-orders:approve',
  SALES_ORDER_CANCEL: 'sales-orders:cancel',

  // 采购订单（后端资源：purchase-orders）
  PURCHASE_ORDER_READ: 'purchase-orders:read',
  PURCHASE_ORDER_CREATE: 'purchase-orders:create',
  PURCHASE_ORDER_UPDATE: 'purchase-orders:update',
  PURCHASE_ORDER_DELETE: 'purchase-orders:delete',
  PURCHASE_ORDER_APPROVE: 'purchase-orders:approve',
  PURCHASE_ORDER_RECEIVE: 'purchase-orders:receive',

  // 凭证（后端资源：vouchers）
  VOUCHER_READ: 'vouchers:read',
  VOUCHER_CREATE: 'vouchers:create',
  VOUCHER_UPDATE: 'vouchers:update',
  VOUCHER_DELETE: 'vouchers:delete',
  VOUCHER_APPROVE: 'vouchers:approve',
  // 导出面键由端点路径末段派生（GET /vouchers/export）；列表页的打印入口只把已读到的行交给
  // 浏览器本地打印，不落后端打印端点，故该入口的门控取读面键 VOUCHER_READ。
  VOUCHER_EXPORT: 'vouchers:export',

  // 预算（后端资源：budgets）
  BUDGET_READ: 'budgets:read',
  BUDGET_CREATE: 'budgets:create',
  BUDGET_UPDATE: 'budgets:update',
  BUDGET_DELETE: 'budgets:delete',
  BUDGET_APPROVE: 'budgets:approve',

  // 仪表盘（后端资源：dashboard）
  DASHBOARD_READ: 'dashboard:read',

  // 审计日志（后端资源：audit-logs）
  AUDIT_READ: 'audit-logs:read',

  // 财务域粗域键：后端没有名为 finance 的 resource 段（finance 只是 URL 模块前缀），
  // finance:read 的可放行集合由 router 的 COARSE_DOMAIN_RESOURCES.finance 细资源并集解释，
  // 供财务各功能页的页面级门控使用；页面内按钮一律用下面各资源的派生真名键，不得用本粗键兼职。
  FINANCE_READ: 'finance:read',

  // 应付发票（后端资源：ap-invoices；URL 段 ap/invoices 经 path_utils 的 RESOURCE_NAME_MAP
  // 消歧后与 PERMISSION_RESOURCES 在册名同词）。列表页打印入口只做浏览器本地打印、
  // 不落后端打印端点，故门控取读面键 AP_INVOICE_READ。
  AP_INVOICE_READ: 'ap-invoices:read',
  AP_INVOICE_EXPORT: 'ap-invoices:export',

  // 应收发票（后端资源：ar-invoices，与应付同型随域改名）
  AR_INVOICE_READ: 'ar-invoices:read',
  AR_INVOICE_EXPORT: 'ar-invoices:export',

  // 财务报表（后端资源：finance 前缀加报表段；URL 形如 /finance/reports/<报表段>，
  // 跳段后仍由 RESOURCE_NAME_MAP 消歧，六个报表面各自一个资源名，互不代偿）
  FINANCE_BALANCE_SHEET_READ: 'finance-balance-sheet:read',
  FINANCE_INCOME_STATEMENT_READ: 'finance-income-statement:read',
  FINANCE_CASH_FLOW_READ: 'finance-cash-flow:read',
  FINANCE_TRIAL_BALANCE_READ: 'finance-trial-balance:read',
  FINANCE_GENERAL_LEDGER_READ: 'finance-general-ledger:read',
  FINANCE_SUBSIDIARY_LEDGER_READ: 'finance-subsidiary-ledger:read',

  // 会计科目（后端资源：subjects，根级直接资源；导出面 GET /subjects/export 末段派生）
  SUBJECT_READ: 'subjects:read',
  SUBJECT_UPDATE: 'subjects:update',
  SUBJECT_DELETE: 'subjects:delete',
  SUBJECT_EXPORT: 'subjects:export',

  // 财务分析（后端资源：financial-analysis，根级直接资源；读写面走 /financial-analysis/reports）
  FINANCIAL_ANALYSIS_UPDATE: 'financial-analysis:update',
  FINANCIAL_ANALYSIS_DELETE: 'financial-analysis:delete',

  // 资金管理（后端资源：fund-management，根级直接资源；建户面 POST /fund-management/accounts）
  FUND_MANAGEMENT_CREATE: 'fund-management:create',

  // 固定资产（后端资源：fixed-assets，根级直接资源；导出面 GET /fixed-assets/export）
  FIXED_ASSET_EXPORT: 'fixed-assets:export',

  // BPM 工作流（后端资源：bpm-*，`bpm` 是 URL 模块前缀，第四段经 path_utils 的
  // RESOURCE_NAME_MAP 消歧为 bpm-tasks/bpm-approvals/… 后与 PERMISSION_RESOURCES 同词）。
  // 动作段派生依据：GET→read、POST→create、PUT→update、DELETE→delete，末段命中
  // 路径动作关键字（approve/cancel…）时以该关键字为动作。
  BPM_TASK_READ: 'bpm-tasks:read',
  // 审批提交端点是 POST /bpm/tasks/approve，末段 approve 属路径动作关键字
  BPM_TASK_APPROVE: 'bpm-tasks:approve',
  // 转办与催办（POST /bpm/tasks/{id}/transfer、/urge）末段不是动作关键字，按方法派生落 create
  BPM_TASK_CREATE: 'bpm-tasks:create',
  // 审批中心执行入口只有 POST /bpm/approval/execute，execute 不是动作关键字，
  // 因此该面不存在 read/approve 形态的派生键，只有 create
  BPM_APPROVAL_EXECUTE: 'bpm-approvals:create',
  // 实例详情与审批链（GET /bpm/instances/{id}/detail、/approval-chain）走同一资源段
  BPM_INSTANCE_READ: 'bpm-instances:read',
  // 撤销实例端点是 POST /bpm/instances/{id}/cancel，末段 cancel 属路径动作关键字
  BPM_INSTANCE_CANCEL: 'bpm-instances:cancel',
  // 全局流程监控面（GET /bpm/monitor/*）未按发起人收窄，持该键可读全系统实例与待办
  BPM_MONITOR_READ: 'bpm-monitor:read',
  // 发起流程端点是 POST /bpm/process/start，start 不是动作关键字，按方法派生落 create
  BPM_PROCESS_CREATE: 'bpm-process:create',
  // 业务关联查询（GET /bpm/business-relation）与流程图（GET /bpm/visualization/{id}）
  BPM_BUSINESS_RELATION_READ: 'bpm-business-relation:read',
  BPM_VISUALIZATION_READ: 'bpm-visualization:read',
  // 流程定义面（GET/POST /bpm/definitions、PUT/DELETE /bpm/definitions/{id}）
  BPM_DEFINITION_READ: 'bpm-definitions:read',
  BPM_DEFINITION_UPDATE: 'bpm-definitions:update',
  BPM_DEFINITION_DELETE: 'bpm-definitions:delete',
  // 流程模板面（GET /bpm/templates）
  BPM_TEMPLATE_READ: 'bpm-templates:read',

  // 消息通知（后端资源：notifications）
  NOTIFICATION_READ: 'notifications:read',
  NOTIFICATION_CREATE: 'notifications:create',

  // OA 公告（后端资源：oa-announcements）
  OA_ANNOUNCEMENT_READ: 'oa-announcements:read',
  OA_ANNOUNCEMENT_CREATE: 'oa-announcements:create',
  OA_ANNOUNCEMENT_UPDATE: 'oa-announcements:update',
  OA_ANNOUNCEMENT_DELETE: 'oa-announcements:delete',

  // SKU 对照表（后端资源：sku-mappings，采购域）
  SKU_MAPPING_READ: 'sku-mappings:read',
  SKU_MAPPING_CREATE: 'sku-mappings:create',
  SKU_MAPPING_IMPORT: 'sku-mappings:import',
  SKU_MAPPING_UPDATE: 'sku-mappings:update',
  SKU_MAPPING_DELETE: 'sku-mappings:delete',

  // 供应商商品目录（后端资源：supplier-products，采购域）
  SUPPLIER_PRODUCT_READ: 'supplier-products:read',
  SUPPLIER_PRODUCT_CREATE: 'supplier-products:create',
  SUPPLIER_PRODUCT_UPDATE: 'supplier-products:update',

  // 供应商商品色号目录（后端资源：supplier-product-colors，采购域）
  SUPPLIER_PRODUCT_COLOR_READ: 'supplier-product-colors:read',
  SUPPLIER_PRODUCT_COLOR_CREATE: 'supplier-product-colors:create',
  SUPPLIER_PRODUCT_COLOR_UPDATE: 'supplier-product-colors:update',

  // 系统更新（高危敏感面：二进制替换 / 版本回滚 / 上传安装 / 备份 / 任务）
  // 权限键与后端 utils/path_utils.rs 消歧（("system-update", "<seg>") →
  // "system-update-<seg>"）及 init_service.rs 注册逐字符一致；读取面统一 :read，
  // 写入面按派生规则取 :create（versions 的 download/install 均落在 versions:create，
  // 任务取消是末段动作 → tasks:cancel）。后端独立播种角色 code=ops 持本族全部读写键，
  // admin 经 *:* 通配覆盖。任一常量与后端不同名即令 check-contract 与 v-permission 双失效。
  SYSTEM_UPDATE_CHECK_READ: 'system-update-check:read',
  SYSTEM_UPDATE_VERSION_READ: 'system-update-version:read',
  SYSTEM_UPDATE_CURRENT_VERSION_READ: 'system-update-current-version:read',
  SYSTEM_UPDATE_UPDATE_STATUS_READ: 'system-update-update-status:read',
  SYSTEM_UPDATE_LOCAL_CHECK_READ: 'system-update-local-check:read',
  SYSTEM_UPDATE_LOCAL_RELEASES_READ: 'system-update-local-releases:read',
  SYSTEM_UPDATE_TASKS_READ: 'system-update-tasks:read',
  SYSTEM_UPDATE_VERSIONS_READ: 'system-update-versions:read',
  SYSTEM_UPDATE_BACKUPS_READ: 'system-update-backups:read',
  SYSTEM_UPDATE_UPDATE_CREATE: 'system-update-update:create',
  SYSTEM_UPDATE_UPLOAD_CREATE: 'system-update-upload:create',
  SYSTEM_UPDATE_ROLLBACK_CREATE: 'system-update-rollback:create',
  SYSTEM_UPDATE_LOCAL_UPDATE_CREATE: 'system-update-local-update:create',
  SYSTEM_UPDATE_BACKUPS_CREATE: 'system-update-backups:create',
  SYSTEM_UPDATE_VERSIONS_CREATE: 'system-update-versions:create',
  SYSTEM_UPDATE_TASKS_CANCEL: 'system-update-tasks:cancel',
} as const;

// ============================================================================
// 类型定义
// ============================================================================
export type PermissionCode = (typeof PERMISSIONS)[keyof typeof PERMISSIONS];
