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
 *   - 动作：read / create / update / delete / approve / cancel / export / print
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
} as const

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
  SALES_PRICE_APPROVE: 'sales-prices:approve',

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

  // 采购合同（后端资源：purchase-contracts）
  PURCHASE_CONTRACT_READ: 'purchase-contracts:read',
  PURCHASE_CONTRACT_CREATE: 'purchase-contracts:create',
  PURCHASE_CONTRACT_UPDATE: 'purchase-contracts:update',
  PURCHASE_CONTRACT_DELETE: 'purchase-contracts:delete',

  // 采购价格（后端资源：purchase-prices）
  PURCHASE_PRICE_READ: 'purchase-prices:read',
  PURCHASE_PRICE_CREATE: 'purchase-prices:create',
  PURCHASE_PRICE_UPDATE: 'purchase-prices:update',
  PURCHASE_PRICE_DELETE: 'purchase-prices:delete',

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
  COST_COLLECTION_APPROVE: 'cost-collections:approve',

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

  // 库存（后端资源：inventory）
  INVENTORY_READ: 'inventory:read',
  INVENTORY_CREATE: 'inventory:create',
  INVENTORY_UPDATE: 'inventory:update',
  INVENTORY_DELETE: 'inventory:delete',
  // Batch 468 P0-S28：库存调拨动作（后端 PATH_ACTION_KEYWORDS 含 transfer）
  INVENTORY_TRANSFER: 'inventory:transfer',

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

  // 财务（后端资源：finance）
  FINANCE_READ: 'finance:read',
  FINANCE_CREATE: 'finance:create',
  FINANCE_UPDATE: 'finance:update',
  FINANCE_DELETE: 'finance:delete',
} as const

// ============================================================================
// 类型定义
// ============================================================================
export type PermissionCode = typeof PERMISSIONS[keyof typeof PERMISSIONS]
