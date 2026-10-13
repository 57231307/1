import { logger } from '@/utils/logger';

/**
 * 敏感导出审批的状态与资源类型单一映射源（申请侧与审批侧共用一份）。
 *
 * 状态取值与后端 models/export_approval_request.rs 的 ApprovalStatus.as_str 出参
 * 逐字符对应（小写 token），资源类型取值与同文件 sensitive_resources 注册表逐一对应。
 * labelKey 一律写完整 i18n 键路径，由 check-i18n 校验中英双侧存在（范式同
 * constants/return-reason.ts 与 utils/sales-status.ts）；词表外 token 记错误日志并
 * 抛错，禁止把裸枚举回显给用户，也禁止用兜底值掩盖后端新增取值未同步前端的事实。
 */

export const EXPORT_APPROVAL_STATUSES = [
  'pending',
  'pending_l2',
  'approved',
  'rejected',
  'expired',
  'cancelled',
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
