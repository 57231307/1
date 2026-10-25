import { i18n } from '@/i18n';
import { logger } from '@/utils/logger';

/**
 * 敏感导出审批的单一映射源：状态词表、资源类型词表、各资源的导出端点与
 * 申请参数形状（申请侧与审批侧共用一份，禁止各页再复制）。
 *
 * 状态取值与后端 models/export_approval_request.rs 的 ApprovalStatus.as_str 出参
 * 逐字符对应（小写 token），资源类型取值与同文件 sensitive_resources 注册表逐一对应。
 * labelKey 一律写完整 i18n 键路径，由 check-i18n 校验中英双侧存在（范式同
 * constants/return-reason.ts 与 utils/sales-status.ts）；词表外 token 记错误日志并
 * 抛错，禁止把裸枚举回显给用户，也禁止用兜底值掩盖后端新增取值未同步前端的事实。
 *
 * 端点形状与权限键依据：后端各资源导出 handler 以 Query 参数 download_token 承载
 * 审批令牌（services/export_approval_service.rs 的 enforce_export_download fail-closed
 * 校验），权限键按 middleware/permission.rs 的 URL 段派生规则逐端点核实后登记。
 */

export const EXPORT_APPROVAL_STATUSES = [
  'pending',
  'pending_l2',
  'approved',
  'rejected',
  'expired',
  'cancelled',
  'consumed',
  'exhausted',
] as const;

export type ExportApprovalStatus = (typeof EXPORT_APPROVAL_STATUSES)[number];

/** el-tag 配色类型（与 element-plus Tag 的 type 属性对齐） */
export type ExportApprovalTagType = 'success' | 'warning' | 'info' | 'danger';

/** 状态 i18n 键（locales 的 exportApprovals 命名空间） */
export const EXPORT_APPROVAL_STATUS_LABEL_KEYS: Record<ExportApprovalStatus, string> = {
  pending: 'exportApprovals.statusPending',
  pending_l2: 'exportApprovals.statusPendingL2',
  approved: 'exportApprovals.approved',
  rejected: 'exportApprovals.rejected',
  expired: 'exportApprovals.statusExpired',
  cancelled: 'exportApprovals.cancelled',
  consumed: 'exportApprovals.statusConsumed',
  exhausted: 'exportApprovals.statusExhausted',
};

/** Element Plus el-tag 类型 */
export const EXPORT_APPROVAL_STATUS_TAG_TYPES: Record<ExportApprovalStatus, ExportApprovalTagType> =
  {
    pending: 'warning',
    pending_l2: 'warning',
    approved: 'success',
    rejected: 'danger',
    expired: 'info',
    cancelled: 'info',
    consumed: 'info',
    exhausted: 'danger',
  };

/** 归一到已知状态词表；词表外取值记错误日志并抛错（禁止静默回显裸枚举） */
export function normalizeExportApprovalStatus(status: string): ExportApprovalStatus {
  if ((EXPORT_APPROVAL_STATUSES as readonly string[]).includes(status)) {
    return status as ExportApprovalStatus;
  }
  const message =
    '未识别的导出审批状态 ' +
    status +
    '，合法取值见后端 models/export_approval_request.rs 的 ApprovalStatus';
  logger.error(message, { status });
  throw new Error(message);
}

export function exportApprovalStatusLabelKey(status: string): string {
  return EXPORT_APPROVAL_STATUS_LABEL_KEYS[normalizeExportApprovalStatus(status)];
}

export function exportApprovalStatusTagType(status: string): ExportApprovalTagType {
  return EXPORT_APPROVAL_STATUS_TAG_TYPES[normalizeExportApprovalStatus(status)];
}

export const EXPORT_APPROVAL_RESOURCE_TYPES = [
  'customer',
  'supplier',
  'dye_recipe',
  'price_list',
  'finance_report',
  'audit_log',
  'product',
] as const;

export type ExportApprovalResourceType = (typeof EXPORT_APPROVAL_RESOURCE_TYPES)[number];

/** 资源类型 i18n 键（locales 的 exportApprovals.resourceType 命名空间） */
export const EXPORT_APPROVAL_RESOURCE_LABEL_KEYS: Record<ExportApprovalResourceType, string> = {
  customer: 'exportApprovals.resourceType.customer',
  supplier: 'exportApprovals.resourceType.supplier',
  dye_recipe: 'exportApprovals.resourceType.dye_recipe',
  price_list: 'exportApprovals.resourceType.price_list',
  finance_report: 'exportApprovals.resourceType.finance_report',
  audit_log: 'exportApprovals.resourceType.audit_log',
  product: 'exportApprovals.resourceType.product',
};

/** 归一到后端 sensitive_resources 注册表；词表外取值记错误日志并抛错 */
export function normalizeExportApprovalResourceType(
  resourceType: string
): ExportApprovalResourceType {
  if ((EXPORT_APPROVAL_RESOURCE_TYPES as readonly string[]).includes(resourceType)) {
    return resourceType as ExportApprovalResourceType;
  }
  const message =
    '未识别的导出审批资源类型 ' +
    resourceType +
    '，合法取值见后端 models/export_approval_request.rs 的 sensitive_resources 注册表';
  logger.error(message, { resourceType });
  throw new Error(message);
}

export function exportApprovalResourceLabelKey(resourceType: string): string {
  return EXPORT_APPROVAL_RESOURCE_LABEL_KEYS[normalizeExportApprovalResourceType(resourceType)];
}

/** 发起导出审批申请的权限键（后端 POST /export-approvals，URL 段派生 export-approvals:create） */
export const EXPORT_APPROVAL_REQUEST_PERMISSION_KEY = 'export-approvals:create';

/** 审批页批准/驳回/撤销的权限键（后端按末段动作关键字派生，见 middleware/permission.rs） */
export const EXPORT_APPROVAL_APPROVE_PERMISSION_KEY = 'export-approvals:approve';
export const EXPORT_APPROVAL_REJECT_PERMISSION_KEY = 'export-approvals:reject';
export const EXPORT_APPROVAL_CANCEL_PERMISSION_KEY = 'export-approvals:cancel';

/** 单个敏感资源的导出端点契约（申请参数形状 + 令牌下载端点 + 该端点权限键） */
export interface ExportApprovalEndpointConfig {
  /** 该资源真实导出端点（相对 baseURL /api/v1/erp；审批令牌经 download_token 查询参数携带） */
  readonly exportPath: string;
  /** 申请 export_params 的键白名单，与后端该导出端点 Query 结构真实读取的筛选字段对齐 */
  readonly paramKeys: readonly string[];
  /** 下载端点被权限中间件强制校验的键（按后端 URL 段派生规则逐端点核实） */
  readonly downloadPermissionKey: string;
  /** 下载文件名前缀（ASCII；后端 Content-Disposition 存在时以其为准） */
  readonly filenamePrefix: string;
}

/**
 * 资源类型 → 导出端点契约注册表。
 *
 * 权限键派生依据（middleware/permission.rs + utils/path_utils.rs）：资源名取
 * /api/v1/erp 后的模块/资源段（模块前缀经消歧表映射），动作取 URL 末段关键字
 * （export）。逐端点核实：
 * - /crm/customers/export → crm 前缀下 customers 无消歧臂 → customers:export
 * - /purchase/suppliers/export → suppliers 无 purchase- 前缀消歧臂 → suppliers:export
 * - /products/export → 根级直接资源 → products:export
 * - /production/dye-recipes/export → dye-recipes:export
 * - /sales/sales-prices/export → sales-prices:export
 * - /audit-logs/export → audit-logs 在系统模块前缀表内，URL 段派生把资源名落到了
 *   末段 "export" 上（派生键 export:export）。此为后端派生的既存异常而非前端可修正
 *   项，如实登记；申请入口一律以 export-approvals:create 门控，不依赖该键。
 * finance_report 的端点按申请参数 report_type 分流（六个报表各自一个导出端点），
 * 见 FINANCE_REPORT_EXPORT_SEGMENTS 与 resolveExportDownloadTarget。
 */
export const EXPORT_APPROVAL_ENDPOINTS: Record<
  ExportApprovalResourceType,
  ExportApprovalEndpointConfig
> = {
  customer: {
    exportPath: '/crm/customers/export',
    paramKeys: ['keyword', 'customer_type', 'status'],
    downloadPermissionKey: 'customers:export',
    filenamePrefix: 'customers_export',
  },
  supplier: {
    exportPath: '/purchase/suppliers/export',
    paramKeys: ['keyword', 'grade', 'status', 'is_processor'],
    downloadPermissionKey: 'suppliers:export',
    filenamePrefix: 'suppliers_export',
  },
  product: {
    exportPath: '/products/export',
    paramKeys: ['category_id', 'status', 'search'],
    downloadPermissionKey: 'products:export',
    filenamePrefix: 'products_export',
  },
  dye_recipe: {
    exportPath: '/production/dye-recipes/export',
    paramKeys: ['recipe_no', 'color_code', 'color_name', 'dye_type', 'status'],
    downloadPermissionKey: 'dye-recipes:export',
    filenamePrefix: 'dye_recipes_export',
  },
  price_list: {
    exportPath: '/sales/sales-prices/export',
    paramKeys: ['product_id', 'customer_id', 'keyword', 'status'],
    downloadPermissionKey: 'sales-prices:export',
    filenamePrefix: 'sales_prices_export',
  },
  audit_log: {
    exportPath: '/audit-logs/export',
    paramKeys: [
      'operation_type',
      'severity',
      'resource_type',
      'request_id',
      'keyword',
      'start_time',
      'end_time',
    ],
    downloadPermissionKey: 'export:export',
    filenamePrefix: 'audit_logs_export',
  },
  finance_report: {
    // 六报表各有一个导出端点，静态路径不适用：令牌下载一律经
    // resolveExportDownloadTarget 按 report_type 解析（见文件头注释）
    exportPath: '',
    paramKeys: [
      'report_type',
      'period',
      'start_date',
      'end_date',
      'subject_code',
      'dimension_type',
      'dimension_value',
    ],
    downloadPermissionKey: '',
    filenamePrefix: 'finance_report_export',
  },
};

/** finance_report 申请参数 report_type → 报表导出端点段（后端 routes/finance.rs 六端点） */
export const FINANCE_REPORT_EXPORT_SEGMENTS: Record<string, string> = {
  trial_balance: 'trial-balance',
  balance_sheet: 'balance-sheet',
  income_statement: 'income-statement',
  cash_flow: 'cash-flow',
  general_ledger: 'general-ledger',
  subsidiary_ledger: 'subsidiary-ledger',
};

/** 令牌下载目标：端点相对路径 + 文件名前缀 + 该端点权限键 */
export interface ExportDownloadTarget {
  readonly apiPath: string;
  readonly permissionKey: string;
  readonly filenamePrefix: string;
}

/**
 * 按资源类型与申请时刻落库的 export_params 解析令牌下载端点。
 * finance_report 依 report_type 分流到六个报表导出端点之一；report_type 缺失或
 * 不在词表内属于申请数据本身不合法（后端也必然 404/422），记错误日志并抛错外显，
 * 禁止兜底猜一个端点把用户导错数据。
 */
export function resolveExportDownloadTarget(
  resourceType: string,
  exportParams: Record<string, unknown> | null
): ExportDownloadTarget {
  const rt = normalizeExportApprovalResourceType(resourceType);
  if (rt === 'finance_report') {
    const reportType = exportParams?.report_type;
    const segment =
      typeof reportType === 'string' ? FINANCE_REPORT_EXPORT_SEGMENTS[reportType] : undefined;
    if (!segment) {
      const message = i18n.global.t('exportApprovals.unknownReportType');
      logger.error('resolveExportDownloadTarget: finance_report 申请缺少有效 report_type', {
        reportType,
      });
      throw new Error(message);
    }
    return {
      apiPath: `/finance/reports/${segment}/export`,
      permissionKey: `${segment}:export`,
      filenamePrefix: `finance_report_${segment}`,
    };
  }
  const cfg = EXPORT_APPROVAL_ENDPOINTS[rt];
  return {
    apiPath: cfg.exportPath,
    permissionKey: cfg.downloadPermissionKey,
    filenamePrefix: cfg.filenamePrefix,
  };
}

/**
 * 资源类型 → 应用内导出入口路由（审批通过后“去导出”的跳转目标）。
 * 唯一事实来源为承载就地下载入口（useExportApproval.downloadApprovedExport）
 * 的页面在 router/index.ts 的注册路径；新增资源接入时必须同步登记。
 */
export const EXPORT_APPROVAL_TARGET_ROUTES: Record<ExportApprovalResourceType, string> = {
  customer: '/customer',
  supplier: '/supplier',
  product: '/product',
  dye_recipe: '/dye-recipe',
  price_list: '/sales-price',
  finance_report: '/finance-report',
  audit_log: '/system/audit-log',
};

/** 归一到目标路由；词外资源沿 normalize 的记日志并抛错口径，不兜底跳首页 */
export function exportApprovalTargetRoute(resourceType: string): string {
  return EXPORT_APPROVAL_TARGET_ROUTES[normalizeExportApprovalResourceType(resourceType)];
}

/** 审批中心（BPM 待办列表）路由，导出审批终审进度回看的正规入口 */
export const BPM_APPROVAL_CENTER_ROUTE = '/bpm/approval';

/**
 * 按注册表 paramKeys 白名单收敛申请/下载参数：只透传该资源导出端点真实读取的键，
 * 空串/纯空白/undefined/null 不进入集合（与 request.ts serializeParams 和后端空参
 * 剔除是同一契约）。数字 0 与布尔 false 是有效取值，不丢弃。
 * exportParams 为 null 对应后端 Option<export_params> 列，按空参数处理。
 */
export function pickExportQueryParams(
  resourceType: string,
  exportParams: Record<string, unknown> | null
): Record<string, unknown> {
  const cfg = EXPORT_APPROVAL_ENDPOINTS[normalizeExportApprovalResourceType(resourceType)];
  const picked: Record<string, unknown> = {};
  if (!exportParams) return picked;
  for (const key of cfg.paramKeys) {
    const value = exportParams[key];
    if (value === undefined || value === null) continue;
    if (typeof value === 'string' && value.trim() === '') continue;
    picked[key] = value;
  }
  return picked;
}
