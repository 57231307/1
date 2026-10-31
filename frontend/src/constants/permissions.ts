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

  // 字段权限（后端资源：field-permissions，routes/iam.rs field_permissions() 派生键）
  FIELD_PERMISSION_READ: 'field-permissions:read',
  FIELD_PERMISSION_CREATE: 'field-permissions:create',
  FIELD_PERMISSION_UPDATE: 'field-permissions:update',
  FIELD_PERMISSION_DELETE: 'field-permissions:delete',

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
  // 仓库导表面键由端点路径末段派生（GET /warehouses/export），与读权限分键
  WAREHOUSE_EXPORT: 'warehouses:export',

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

  // 客户信用管理（后端资源段 customer-credits，路由挂在模块前缀 crm 下，
  // 不在 path_utils 的消歧映射表内，故资源名保留 URL 段原名）。
  // 动作段取值逐条取自后端权限中间件的键派生规则，路径末段命中
  // PATH_ACTION_KEYWORDS 时取末段，否则按方法派生，GET 走 read，POST 走 create，
  // PUT 走 update，DELETE 走 delete。
  // rating occupy adjust evaluate deactivate 五个末段都不在该关键字表内，
  // 全部派生成 create 一枚键；release 命中关键字表，是独立键；
  // 详情面走同资源 read 键。这四枚键与该页按钮的取值由 tests unit 下的
  // 客户信用动作权限与信封锁三向互锁。
  CUSTOMER_CREDIT_READ: 'customer-credits:read',
  CUSTOMER_CREDIT_CREATE: 'customer-credits:create',
  CUSTOMER_CREDIT_UPDATE: 'customer-credits:update',
  CUSTOMER_CREDIT_DELETE: 'customer-credits:delete',
  CUSTOMER_CREDIT_RELEASE: 'customer-credits:release',

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
  // 提交/通过/拒绝是三条独立端点，运行时键由 URL 末段动作派生，各用各的键，禁止互相兼职放行。
  QUOTATION_SUBMIT: 'quotations:submit',
  QUOTATION_APPROVE: 'quotations:approve',
  QUOTATION_REJECT: 'quotations:reject',

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
  // 导表面键由端点路径末段派生（GET /sales/sales-contracts/export），与读权限分键；
  // 列表页打印入口只做浏览器本地打印、不落后端端点，门控取读面键 SALES_CONTRACT_READ
  SALES_CONTRACT_EXPORT: 'sales-contracts:export',

  // 生产工单（后端资源：production-orders；URL /production/production-orders/orders/{id}，
  // production 与 production-orders 是双层模块前缀、资源名落第五段 orders 再经消歧映射
  // 回权威名 production-orders，动作按方法派生）
  PRODUCTION_ORDER_UPDATE: 'production-orders:update',
  PRODUCTION_ORDER_DELETE: 'production-orders:delete',

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
  // 质量标准导表面键由端点路径末段派生（GET /quality-standards/export），与读权限分键；
  // 列表页打印入口只做浏览器本地打印、不落后端端点，门控取读面键 QUALITY_STANDARD_READ
  QUALITY_STANDARD_EXPORT: 'quality-standards:export',

  // 质检记录面（后端资源：quality-inspections；URL /production/quality-inspection/records，
  // production 是模块前缀、第四段 quality-inspection 经消歧映射为权威名 quality-inspections）。
  // 导表面 GET /production/quality-inspection/records/export 末段派生 export；
  // 列表打印入口只做浏览器本地打印，门控取读面键 QUALITY_INSPECTION_READ
  QUALITY_INSPECTION_READ: 'quality-inspections:read',
  QUALITY_INSPECTION_EXPORT: 'quality-inspections:export',

  // 邮件模板（后端资源：email-templates，根级直接资源；读写删面 /email-templates 与 /email-templates/{id}）
  EMAIL_TEMPLATE_READ: 'email-templates:read',
  EMAIL_TEMPLATE_UPDATE: 'email-templates:update',
  EMAIL_TEMPLATE_DELETE: 'email-templates:delete',

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
  // 库存调拨页入口（打开调拨页即 GET /inventory/transfers 取列表）。
  // inventory 是模块前缀，调拨自身资源段是 URL 第四段 transfers（后端注册表在册名）。
  // transfer 不在后端 PATH_ACTION_KEYWORDS 动作关键字内，该域端点动作只由方法派生
  // （GET→read、POST→create、PUT/PATCH→update、DELETE→delete），故 inventory:transfer
  // 是后端永远派生不出来的幻影键。
  INVENTORY_TRANSFER_READ: 'transfers:read',
  // 库存子资源各自的建/改/删面键（inventory 是模块前缀，权威资源名落 URL 第四段）：
  // 台账面 GET /inventory/stock（列表打印入口只做浏览器本地打印，门控取读面键 STOCK_READ）；
  // 导表面 GET /inventory/stock/export 末段派生 export
  STOCK_READ: 'stock:read',
  STOCK_EXPORT: 'stock:export',
  // 调整单建单面 POST /inventory/adjustments，与库存台账读写互不代偿
  ADJUSTMENT_CREATE: 'adjustments:create',
  // 批次面（/inventory/batches）：批次调拨提交 POST /inventory/batches/{id}/transfer，
  // transfer 不是路径动作关键字，按方法派生落 create，与建单面同键
  BATCH_CREATE: 'batches:create',
  BATCH_UPDATE: 'batches:update',
  BATCH_DELETE: 'batches:delete',
  // 盘点单建单面 POST /inventory/counts
  COUNT_CREATE: 'counts:create',
  // 调拨单建单面 POST /inventory/transfers（读面见 INVENTORY_TRANSFER_READ）
  TRANSFER_CREATE: 'transfers:create',
  // 成品布入库标签打印（GET /inventory/pieces/{id}/print）。权限键非自造
  // resource_type 由 URL 段推导——middleware/permission.rs::extract_resource_info 对
  // 模块前缀 inventory（utils/path_utils.rs::is_business_module_prefix）取 segment4
  // "pieces"（path_utils.rs:102-117 resolve_module_prefixed_resource 默认分支，
  // 与 /inventory/transfers 取 "transfers" 同构）；action="print" 来自末段
  // PATH_ACTION_KEYWORDS（permission.rs:218-221）。登录出参权限码即
  // "{resource_type}:{action}"（handlers/auth_handler.rs:176）。
  INVENTORY_PIECE_PRINT: 'pieces:print',

  // 销售订单（后端资源：orders——`/sales/orders` 的 sales 是模块前缀，消歧映射未登记
  // sales+orders 组合，资源段保留 URL 第四段原名 orders；sales-orders 不是后端派生名）
  SALES_ORDER_CREATE: 'orders:create',
  SALES_ORDER_SUBMIT: 'orders:submit',
  SALES_ORDER_CANCEL: 'orders:cancel',
  SALES_ORDER_DELETE: 'orders:delete',
  SALES_ORDER_APPROVE: 'orders:approve',
  SALES_ORDER_REJECT: 'orders:reject',

  // 采购订单（后端资源：purchase-orders）
  PURCHASE_ORDER_READ: 'purchase-orders:read',
  PURCHASE_ORDER_CREATE: 'purchase-orders:create',
  PURCHASE_ORDER_UPDATE: 'purchase-orders:update',
  PURCHASE_ORDER_DELETE: 'purchase-orders:delete',
  PURCHASE_ORDER_APPROVE: 'purchase-orders:approve',
  // 采购订单导表面键由端点路径末段派生（GET /purchase/orders/export，purchase 是模块
  // 前缀、资源名落消歧后的第四段 orders→purchase-orders），与读权限分键
  PURCHASE_ORDER_EXPORT: 'purchase-orders:export',
  // "收货"入口的真实动作是建入库单：对话框提交 POST /purchase/receipts
  // （purchase 是模块前缀，消歧映射把 receipts 对齐到注册表权威名 purchase-receipts，
  // 动作由方法派生为 create）。后端既无 receive 这个动作词，也没有 /purchase/orders/{id}/receive
  // 或 /purchase/receipts/{id}/receive 路由，故不存在 purchase-orders:receive 这个可授予键。
  PURCHASE_RECEIPT_CREATE: 'purchase-receipts:create',

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

  // 应付域其余页签的入口读面键（后端资源名逐枚对齐 PERMISSION_RESOURCES 与
  // path_utils 消歧目标；派生依据 = GET /ap/<段>，动作段按 GET→read）：
  // /ap/payments、/ap/payment-requests、/ap/verifications、/ap/reconciliations。
  AP_PAYMENT_READ: 'ap-payments:read',
  AP_PAYMENT_REQUEST_READ: 'ap-payment-requests:read',
  AP_VERIFICATION_READ: 'ap-verifications:read',
  AP_RECONCILIATION_READ: 'ap-reconciliations:read',

  // 应付报表中心四本账（GET /ap/reports/<段>：ap 与 reports 同为模块前缀，跳段后资源名落在
  // 第五段并经消歧映射为 ap-aging / ap-daily / ap-monthly / ap-statistics，四本彼此独立，
  // 任一枚都不代偿另一枚）。
  AP_AGING_READ: 'ap-aging:read',
  AP_DAILY_READ: 'ap-daily:read',
  AP_MONTHLY_READ: 'ap-monthly:read',
  AP_STATISTICS_READ: 'ap-statistics:read',

  // 应收发票（后端资源：ar-invoices，与应付同型随域改名）
  AR_INVOICE_READ: 'ar-invoices:read',
  AR_INVOICE_EXPORT: 'ar-invoices:export',
  // 应收发票的制单面与审核面（后端资源 ar-invoices）：建单面 POST /ar/invoices 按方法派生
  // create，改单面 PUT /ar/invoices/{id} 派生 update（两枚都授记账岗）；审核面与作废面的
  // 末段 approve 与 cancel 命中 PATH_ACTION_KEYWORDS 派生同名动作（两枚都授审核岗）。
  // 制单与审核互斥，五个键的持有者集合分属两岗，所以页签内按钮必须逐枚守卫。
  AR_INVOICE_CREATE: 'ar-invoices:create',
  AR_INVOICE_UPDATE: 'ar-invoices:update',
  AR_INVOICE_APPROVE: 'ar-invoices:approve',
  AR_INVOICE_CANCEL: 'ar-invoices:cancel',

  // 应收族其余页签的入口读面键（后端资源名逐枚对齐 PERMISSION_RESOURCES 与 path_utils
  // 的 ar 前缀消歧目标；派生依据 = GET /ar/<段>，动作段按 GET 到 read）：
  // /ar/payments、/ar/verifications、/ar-reconciliations（对账族挂根级第四段，不经 ar 消歧）。
  AR_PAYMENT_READ: 'ar-payments:read',
  // 收款面的三个写动作（后端资源 ar-payments，动作按方法派生：POST 到 create、
  // PUT 到 update，末段 confirm 命中 PATH_ACTION_KEYWORDS 派生 confirm）。
  // 建单与改单授记账岗，确认授审核岗，两岗互斥，所以三个键的持有者集合不同，
  // 收款页签的行内按钮必须逐枚守卫，否则总有一岗看得见点不得（点了必 403）。
  AR_PAYMENT_CREATE: 'ar-payments:create',
  AR_PAYMENT_UPDATE: 'ar-payments:update',
  AR_PAYMENT_CONFIRM: 'ar-payments:confirm',
  AR_VERIFICATION_READ: 'ar-verifications:read',
  // 核销面的建单与撤核（后端资源 ar-verifications）：自动与手工两条 POST 端点的末段
  // auto 与 manual 都不在 PATH_ACTION_KEYWORDS 内，一律按方法派生成 create，
  // 所以一枚 create 同时放出两条核销腿（已知捆绑，页面两把按钮共用同一判据）；
  // 撤核面 POST /ar/verifications/{id}/cancel 末段命中关键字派生 cancel。两枚都只授记账岗，
  // 审核岗在本页签只有读权，按钮不守卫就是点了必 403。
  AR_VERIFICATION_CREATE: 'ar-verifications:create',
  AR_VERIFICATION_CANCEL: 'ar-verifications:cancel',
  AR_RECONCILIATION_READ: 'ar-reconciliations:read',

  // 收款单 DOCX 打印（后端资源：ar-collections，URL 段 ar/collections 消歧得名）。
  // 本资源只有一个动作可派生：GET /ar/collections/{id}/print 末段命中路径动作关键字表
  // 派生 print，该资源没有任何 read 端点，所以读面键 ar-collections:read 是运行时
  // 永不命中的悬空授权，不许写。前端收款页签的行内打印按钮按本键守卫，缺键即按钮隐去。
  AR_COLLECTION_PRINT: 'ar-collections:print',

  // 应收报表中心四本账（GET /ar/reports/<段>：ar 与 reports 同为模块前缀，跳段后资源名
  // 落在第五段并经消歧映射为 ar-aging / ar-daily / ar-monthly / ar-statistics，
  // 四本彼此独立，任一枚都不代偿另一枚）。
  AR_AGING_READ: 'ar-aging:read',
  AR_DAILY_READ: 'ar-daily:read',
  AR_MONTHLY_READ: 'ar-monthly:read',
  AR_STATISTICS_READ: 'ar-statistics:read',

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
  // READ 是应收壳页「资金账户」页签的入口判据（该页签挂载即拉 GET /fund-management/accounts，
  // 资源段取 URL 第四段原名，不经任何前缀消歧）。
  FUND_MANAGEMENT_READ: 'fund-management:read',
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

// ============================================================================
// 应付域（/ap）门控集合
// ============================================================================

/**
 * 应付报表中心页签的入口判据：四本账的读面键，任一命中即该页签可达。
 * 页签内部用 el-radio-group 在四本账之间切换，本集合只决定"页签是否出现"。
 */
export const AP_REPORT_TAB_PERMISSIONS: readonly PermissionCode[] = [
  PERMISSIONS.AP_AGING_READ,
  PERMISSIONS.AP_DAILY_READ,
  PERMISSIONS.AP_MONTHLY_READ,
  PERMISSIONS.AP_STATISTICS_READ,
];

/**
 * 应付域壳页（路由 /ap）的入口判据：六个页签各自入口判据的并集。
 *
 * 取并集而非任一单枚，是为了钉住一条不变式："壳页可达 ⇔ 至少一个页签可见"——
 * 用单枚（如仅 ap-invoices:read）会把只持付款或只持报表读权的岗位关在门外，
 * 用粗域键 finance:read 则反向放行整个财务域（只持工资/汇率/凭证读权的岗位
 * 也能打开应付壳页），两端都是错的。
 *
 * 本集合与 router 里 /ap 的 meta.permission 字面量数组由静态锁
 * tests/unit/ap-shell-permission-lock.test.ts 逐字互锁：router 的文本解析口径
 * 只认单引号字面量（不解析标识符），故 router 侧必须写 literals，二者一致性由锁承担。
 */
export const AP_SHELL_ENTRY_PERMISSIONS: readonly PermissionCode[] = [
  PERMISSIONS.AP_INVOICE_READ,
  PERMISSIONS.AP_PAYMENT_READ,
  PERMISSIONS.AP_PAYMENT_REQUEST_READ,
  PERMISSIONS.AP_VERIFICATION_READ,
  PERMISSIONS.AP_RECONCILIATION_READ,
  ...AP_REPORT_TAB_PERMISSIONS,
];

// ============================================================================
// 应收域（/ar）门控集合
// ============================================================================

/**
 * 应收报表中心页签的入口判据：四本账的读面键，任一命中即该页签可达。
 * 页签内部用 el-radio-group 在四本账之间切换，本集合只决定「页签是否出现」。
 * 四枚必须逐枚列：后端 matcher 是资源段等值的精确匹配，报表四本各自一个资源名，
 * 给其中一本的读权放行不了另一本。
 */
export const AR_REPORT_TAB_PERMISSIONS: readonly PermissionCode[] = [
  PERMISSIONS.AR_AGING_READ,
  PERMISSIONS.AR_DAILY_READ,
  PERMISSIONS.AR_MONTHLY_READ,
  PERMISSIONS.AR_STATISTICS_READ,
];

/**
 * 应收域壳页（路由 /ar）的入口判据：六个页签各自入口判据的并集。
 *
 * 与应付域同形，同样取并集钉住「壳页可达 ⇔ 至少一个页签可见」这条不变式，
 * 同样不许用粗域键 finance:read 兼职（后者经 router 的 COARSE_DOMAIN_RESOURCES
 * 展开为整个财务域五十余枚细资源的并集，只持凭证/工资/汇率/成本读权的岗位
 * 也能打开应收壳页，属跨域越权入口；而后端 matcher 从不认粗键，所以粗键给出的
 * 展示面必然比后端能力面宽，点进去就是 403）。
 *
 * 与应付域的一处结构差需要点名：本域的「资金账户」页签走 GET /fund-management/accounts，
 * 后端资源段是 fund-management（根级直接资源，不经 ar 前缀消歧），不属应收族八枚细键。
 * 按「壳页可达当且仅当至少一个页签可见」的不变式，它的读面键必须留在本并集里，
 * 否则出纳（持 fund-management 读与建户、不持任何 ar-* 键）能进壳页却一个页签都看不见。
 *
 * 本集合与 router 里 /ar 的 meta.permission 字面量数组、e2e 权限模型的
 * ROUTE_PERMISSIONS.ar 三处由静态锁 tests/unit/ar-shell-permission-lock.test.ts
 * 逐字互锁；router 侧只能写单引号字面量，因为该口径的文本解析不识别标识符。
 */
export const AR_SHELL_ENTRY_PERMISSIONS: readonly PermissionCode[] = [
  PERMISSIONS.AR_INVOICE_READ,
  PERMISSIONS.AR_PAYMENT_READ,
  PERMISSIONS.AR_VERIFICATION_READ,
  PERMISSIONS.AR_RECONCILIATION_READ,
  PERMISSIONS.FUND_MANAGEMENT_READ,
  ...AR_REPORT_TAB_PERMISSIONS,
];
