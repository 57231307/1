/**
 * ppFmts.ts - 采购价格格式化工具
 * 任务编号: P14 批 2 I-3 第 3 批（拆分原 purchase-price/index.vue）；对齐销售侧 spFmts 范式收口
 *
 * 价格状态词表权威 = backend/src/models/status/sales 的 price_approval 模块（全小写），
 * 采购侧四值均有写入方：建单写 pending（purchase_price_service.rs::create_price）、
 * 审批通过写 approved（::approve_price，仅 pending 可批的状态门在同函数）、
 * 审批拒绝写 rejected（::reject_price，仅 pending 起拒的状态门在同函数）、
 * update_price 按 price_approval::ALL 白名单校验后透传写入（::update_price，非法值 400+VALIDATION_ERROR）。
 * DB CHECK chk_purchase_price_status 与 {pending,approved,rejected,inactive} 恰等
 * （migration/src/domain/price_vocab_check 的约束施加段 + 后继
 * migration/src/domain/price_vocab_extend/m0080_extend_price_status_check_rejected.rs 扩 rejected；
 * 契约锁 backend/tests/contract_wave8_price_status_parity_test.rs 钉 词表常量==DB CHECK，含销售侧
 * 旁路写 inactive 被拒、采购侧 inactive 必须可写的分表负例；该测试不读前端文件，本数组与
 * 词表一致性由 PURCHASE_PRICE_STATUS 常量与本注释同文维持）。
 * 历史纠偏：本模块旧注释与诊断文案曾把权威指向 models/status/general.rs::master_data
 * （那是 supplier/customer 等启用/停用词表，取值域 {active,inactive}，与价格词表仅
 * "inactive" 一字面重合，指向错），连同 api/purchase-price.ts 头注一并纠正为 price_approval。
 * 本模块以 pending/approved/rejected/inactive 原值为比较对象，文案走 i18n 键（purchasePrice.statusLabels.*），
 * 未知 token 记日志后抛错（fail-visible），禁止 `|| status` 兜底外显。
 * 注：本文件字符串字面量（含诊断文案）不得出现裸中文（同销售侧 spFmts 判据：对字符串字面量
 * 做 CJK grep 应为空），开发者诊断信息用英文，用户可见文案一律经 i18n 键取。
 */
import { logger } from '@/utils/logger';
import { i18n } from '@/i18n';

export const PURCHASE_PRICE_STATUS = {
  PENDING: 'pending',
  APPROVED: 'approved',
  REJECTED: 'rejected',
  INACTIVE: 'inactive',
} as const;

export type PurchasePriceStatus =
  (typeof PURCHASE_PRICE_STATUS)[keyof typeof PURCHASE_PRICE_STATUS];

export const PURCHASE_PRICE_STATUSES: PurchasePriceStatus[] = Object.values(PURCHASE_PRICE_STATUS);

export type PurchasePriceTagType = '' | 'success' | 'warning' | 'info' | 'danger';

export const PURCHASE_PRICE_STATUS_LABEL_KEYS: Record<PurchasePriceStatus, string> = {
  pending: 'purchasePrice.statusLabels.pending',
  approved: 'purchasePrice.statusLabels.approved',
  rejected: 'purchasePrice.statusLabels.rejected',
  inactive: 'purchasePrice.statusLabels.inactive',
};

export const PURCHASE_PRICE_STATUS_TAG_TYPES: Record<PurchasePriceStatus, PurchasePriceTagType> = {
  pending: 'warning',
  approved: 'success',
  rejected: 'danger',
  inactive: 'info',
};

export function normalizePurchasePriceStatus(
  status: string | undefined | null
): PurchasePriceStatus | undefined {
  if (status == null || status === '') return undefined;
  if ((PURCHASE_PRICE_STATUSES as readonly string[]).includes(status)) {
    return status as PurchasePriceStatus;
  }
  const message =
    `Unknown purchase price status "${status}". ` +
    'Legal values (purchase-side writers): pending/approved/rejected/inactive — ' +
    'see backend models/status/sales.rs::price_approval';
  logger.error(message, { status });
  throw new Error(message);
}

/** 获取价格状态 el-tag 类型 */
export const getStatusType = (status: string | undefined | null): PurchasePriceTagType => {
  const normalized = normalizePurchasePriceStatus(status);
  return normalized ? PURCHASE_PRICE_STATUS_TAG_TYPES[normalized] : 'info';
};

/** 获取采购价格状态显示文案（i18n） */
export const getStatusLabel = (status: string | undefined | null): string => {
  const normalized = normalizePurchasePriceStatus(status);
  const key = normalized ? PURCHASE_PRICE_STATUS_LABEL_KEYS[normalized] : 'common.statusUnknown';
  return i18n.global.t(key);
};

/** 获取价格类型标签（文案走 i18n，键 purchasePrice.form.priceType.*） */
export const getPriceTypeLabel = (type: string): string =>
  i18n.global.t(`purchasePrice.form.priceType.${type}`);

/** rust_decimal（backend/Cargo.toml 仅启用 serde feature，未启 serde-float）序列化的 JSON 十进制字符串形态 */
const DECIMAL_STRING_RE = /^-?\d+(\.\d+)?$/;

/**
 * 以纯字符串补齐为 6 位定点（DB 列 price DECIMAL(18,6) NOT NULL，
 * backend/migration/src/domain/business/m0009_add_purchase_extensions.rs 建表 purchase_prices 列定义）。
 * 不经过 parseFloat/toFixed，避免二进制浮点丢精度；小数位超过列标度即契约违背，抛错。
 * 与销售侧 spFmts.toFixed6 同形但刻意不跨域 import：两域词表与列标度证据各自独立，保持模块自治。
 */
function toFixed6(value: string): string {
  const negative = value.startsWith('-');
  const unsigned = negative ? value.slice(1) : value;
  const [intPart, fracPart = ''] = unsigned.split('.');
  if (fracPart.length > 6) {
    const message =
      `Price value "${value}" has a scale greater than the DB column DECIMAL(18,6). ` +
      'Backend contract violated.';
    logger.error(message, { value });
    throw new Error(message);
  }
  const frac = fracPart.padEnd(6, '0');
  return `${negative ? '-' : ''}${intPart}.${frac}`;
}

/**
 * 格式化货币（人民币 6 位定点），入参为后端 Decimal 序列化字符串
 * （backend/src/models/purchase_price.rs::Model.price: Decimal 非 Option ⇒ 线格式恒在）。
 * null/undefined/'' 为“缺值”⇒ 显示 common.valueMissing，与真实 0 可区分
 * （禁止 `value ? … : '¥0.000000'` 这类把缺值当 0 的掩盖写法）；
 * 非数值字符串是契约违背 ⇒ 记日志后抛错，不静默兜底。
 */
export const formatCurrency = (value: string | null | undefined): string => {
  if (value === null || value === undefined || value === '') {
    return i18n.global.t('common.valueMissing');
  }
  if (!DECIMAL_STRING_RE.test(value)) {
    const message = `Invalid price value "${value}". Expected a backend-serialized Decimal string like "12.340000".`;
    logger.error(message, { value });
    throw new Error(message);
  }
  return `¥${toFixed6(value)}`;
};
