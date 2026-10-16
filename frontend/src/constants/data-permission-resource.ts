/**
 * 数据权限资源类型（resource_type）候选词表与 i18n 文案键映射。
 *
 * 真相源为后端资源类型端点，前端下拉候选从接口拉取。本文件仅承担两件事：
 * 一、把资源 token 映射到中英齐全的展示文案键，二、在接口不可用时提供兜底候选。
 * token 集合与后端合法资源词表逐字一致（含 inventory_stock、不含旧写的 inventory
 * 与 finance），新增或删减资源必须三处（后端词表、本兜底、语言包）同步。
 *
 * labelKey 写完整 i18n 路径：check-i18n 把 labelKey 字面量当引用校验存在性，
 * 半截路径会落在动态拼接不判定里、拼错就直接把原始键显示到界面。
 */
export interface DataPermissionResourceTypeOption {
  value: string;
  labelKey: string;
}

/** 接口不可用时的兜底候选，与后端合法资源 token 一一对应 */
export const DATA_PERMISSION_RESOURCE_TYPE_OPTIONS: DataPermissionResourceTypeOption[] = [
  { value: 'customer', labelKey: 'dataPermission.resourceType.customer' },
  { value: 'supplier', labelKey: 'dataPermission.resourceType.supplier' },
  { value: 'sales_order', labelKey: 'dataPermission.resourceType.salesOrder' },
  { value: 'purchase_order', labelKey: 'dataPermission.resourceType.purchaseOrder' },
  { value: 'purchase_receipt', labelKey: 'dataPermission.resourceType.purchaseReceipt' },
  { value: 'purchase_receipt_item', labelKey: 'dataPermission.resourceType.purchaseReceiptItem' },
  { value: 'inventory_stock', labelKey: 'dataPermission.resourceType.inventoryStock' },
  { value: 'product', labelKey: 'dataPermission.resourceType.product' },
  { value: 'ap_payment_request', labelKey: 'dataPermission.resourceType.apPaymentRequest' },
  { value: 'crm_lead', labelKey: 'dataPermission.resourceType.crmLead' },
  { value: 'crm_opportunity', labelKey: 'dataPermission.resourceType.crmOpportunity' },
];

/** 兜底候选的纯 token 列表，供接口失败时填充下拉 */
export const FALLBACK_DATA_PERMISSION_RESOURCE_TYPES: string[] =
  DATA_PERMISSION_RESOURCE_TYPE_OPTIONS.map(item => item.value);

const LABEL_KEY_BY_VALUE: Record<string, string> = Object.fromEntries(
  DATA_PERMISSION_RESOURCE_TYPE_OPTIONS.map(item => [item.value, item.labelKey])
);

/**
 * 资源 token 到 i18n 文案键的映射，未知 token 返回 undefined 交由调用方原样显示并告警，
 * 不在此处猜含义、也不静默吞掉后端词表漂移。
 */
export function dataPermissionResourceTypeLabelKey(value: string): string | undefined {
  return LABEL_KEY_BY_VALUE[value];
}
