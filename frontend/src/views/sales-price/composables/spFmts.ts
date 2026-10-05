/**
 * spFmts.ts - 销售价格格式化工具
 * 任务编号: P14 批 2 I-3 第 3 批（拆分原 sales-price/index.vue）；按价格域权威词表整体重写
 *
 * 价格状态词表以销售侧写入方全集为准：建单写 price_approval::PENDING
 * （backend/src/services/sales_price_service.rs::create_price）、审批通过写 APPROVED（::approve_price）、
 * 审批拒绝写 REJECTED（::reject_price）。
 * inactive 无销售侧生产者（采购侧写入方见 purchase_price_service.rs，inactive 由 ::update_price 按
 * price_approval::ALL 白名单透传写入），
 * active/expired 无服务层写入方（建表 DEFAULT 'ACTIVE' 已由 migration/src/domain/price_vocab_check
 * 收敛为 'pending'）—— 旧前端以它们做筛选与标签 ⇒ 筛选恒 0 行、标签回显裸 token。
 * 词表权威 backend/src/models/status/sales.rs:175-190::price_approval（小写
 * pending/approved/rejected/inactive，销售侧取值域为其去掉 inactive 的子集）；契约锁
 * backend/tests/contract_wave8_price_status_parity_test.rs 钉 词表常量==DB CHECK（含旁路写
 * inactive 被拒负例）；该测试不读前端文件，本模块数组与销售侧取值集的一致性
 * 由下方 SALES_PRICE_STATUS 常量与本注释同文维持。
 * 本模块以 pending/approved/rejected 原值为比较对象，文案走 i18n 键（salesPrice.statusLabels.*），
 * 未知 token 记日志后抛错（fail-visible），禁止 `|| status` 兜底外显。
 * 注：本文件字符串字面量（含诊断文案）不得出现裸中文（可开判据：对本文件字符串字面量做
 * CJK 字符 grep 应为空），开发者诊断信息用英文，用户可见文案一律经 i18n 键取。
 */
import { logger } from '@/utils/logger';
import { i18n } from '@/i18n';

export const SALES_PRICE_STATUS = {
  PENDING: 'pending',
  APPROVED: 'approved',
  REJECTED: 'rejected',
} as const;

export type SalesPriceStatus = (typeof SALES_PRICE_STATUS)[keyof typeof SALES_PRICE_STATUS];

export const SALES_PRICE_STATUSES: SalesPriceStatus[] = Object.values(SALES_PRICE_STATUS);

export type SalesPriceTagType = '' | 'success' | 'warning' | 'info' | 'danger';

export const SALES_PRICE_STATUS_LABEL_KEYS: Record<SalesPriceStatus, string> = {
  pending: 'salesPrice.statusLabels.pending',
  approved: 'salesPrice.statusLabels.approved',
  rejected: 'salesPrice.statusLabels.rejected',
};

export const SALES_PRICE_STATUS_TAG_TYPES: Record<SalesPriceStatus, SalesPriceTagType> = {
  pending: 'warning',
  approved: 'success',
  rejected: 'danger',
};

/**
 * 把后端返回的价格状态归一到已知枚举。
 * 状态缺失（行尚未加载）返回 undefined，由调用方渲染“未知”标签；
 * 取值在词表外（拼错的大小写、已废弃的 active/expired/inactive）不被静默当作合法状态：
 * 记错误日志并抛错，禁止调用方用 `|| status` 把裸枚举回显给用户。
 */
export function normalizeSalesPriceStatus(
  status: string | undefined | null
): SalesPriceStatus | undefined {
  if (status == null || status === '') return undefined;
  if ((SALES_PRICE_STATUSES as readonly string[]).includes(status)) {
    return status as SalesPriceStatus;
  }
  const message =
    `Unknown sales price status "${status}". ` +
    'Legal values (sales-side writers): pending/approved/rejected — ' +
    'see backend models/status/sales.rs::price_approval';
  logger.error(message, { status });
  throw new Error(message);
}

/** 获取价格状态 el-tag 类型 */
export const getStatusType = (status: string | undefined | null): SalesPriceTagType => {
  const normalized = normalizeSalesPriceStatus(status);
  return normalized ? SALES_PRICE_STATUS_TAG_TYPES[normalized] : 'info';
};

/** 获取销售价格状态显示文案（i18n） */
export const getStatusLabel = (status: string | undefined | null): string => {
  const normalized = normalizeSalesPriceStatus(status);
  const key = normalized ? SALES_PRICE_STATUS_LABEL_KEYS[normalized] : 'common.statusUnknown';
  return i18n.global.t(key);
};

/** 获取价格类型标签（文案走 i18n，键 salesPrice.form.priceType.*） */
export const getPriceTypeLabel = (type: string): string =>
  i18n.global.t(`salesPrice.form.priceType.${type}`);

/** rust_decimal（backend/Cargo.toml 仅启用 serde feature，未启 serde-float）序列化的 JSON 十进制字符串形态 */
const DECIMAL_STRING_RE = /^-?\d+(\.\d+)?$/;

/**
 * 以纯字符串补齐为 6 位定点（DB 列 price DECIMAL(18,6)，
 * migration/src/domain/business/m0011_add_sales_and_logistics_extensions.rs 建表 sales_prices 列定义）。
 * 不经过 parseFloat/toFixed，避免二进制浮点丢精度；小数位超过列标度即契约违背，抛错。
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
 * 格式化货币（人民币 6 位定点），入参为后端 Decimal 序列化字符串。
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
