// sales-analysis 格式化工具集合
// 拆分自 sales-analysis/index.vue（P14 批 2 I-3 第 6 批）

import { logger } from '@/utils/logger';
import { i18n } from '@/i18n';

/** 后端 Decimal 序列化字符串形态，可带负号，用于契约违背判定 */
const DECIMAL_WIRE_RE = /^-?\d+(\.\d+)?$/;

/**
 * 格式化货币。出参为用户可见单元格文本。
 * 调用方，销售分析页统计卡片、产品与客户排名金额列、销售目标表金额与差额列。
 * 入参，后端 rust_decimal 序列化字符串（如 "1250.00"）或历史数值入参，
 * null、undefined、空串为"缺值"，显示 common.valueMissing 占位，与真零 "0.00"
 * 可区分（禁止 value 真值判断后落 "¥0.00" 这类把缺值当 0 的掩盖写法）。
 * 传给谁，仅返回字符串给调用方渲染，不参与回写、不存储、不打印。
 * 数值字符串按后端定点结果原样加 ¥ 前缀透传，不再舍入；数值入参按两位定点显示。
 * 非数值入参是契约违背，记日志后抛错，不静默兜底。
 */
export const formatCurrency = (value: string | number | null | undefined): string => {
  if (value === null || value === undefined || value === '') {
    return i18n.global.t('common.valueMissing');
  }
  if (typeof value === 'number') {
    if (!Number.isFinite(value)) {
      const message = `formatCurrency 收到非有限数值金额 "${String(value)}"`;
      logger.error(message, { value });
      throw new Error(message);
    }
    return `¥${value.toFixed(2)}`;
  }
  if (!DECIMAL_WIRE_RE.test(value)) {
    const message = `formatCurrency 收到非法金额线格式 "${value}"，期望后端 Decimal 序列化字符串（如 "1250.00"）`;
    logger.error(message, { value });
    throw new Error(message);
  }
  return `¥${value}`;
};

/**
 * 根据完成率返回进度条颜色。出参为颜色字符串。
 * 调用方，销售目标表完成率列，仅在调用方显式判掉 null 后取值传入。
 * 入参，completion_rate 的 Decimal 出参字符串或数值；null 不属于本函数入参域。
 * 传给谁，仅返回颜色给 el-progress 渲染，不存储、不打印。
 */
export const getProgressColor = (percentage: number | string) => {
  const p = Number(percentage);
  if (p >= 100) return '#67c23a';
  if (p >= 80) return '#e6a23c';
  return '#f56c6c';
};

/** 销售目标状态 → ElTag type */
export const getTargetStatusType = (status: string) => {
  const map: Record<string, string> = {
    COMPLETED: 'success',
    IN_PROGRESS: 'warning',
    PARTIAL: 'info',
    NOT_STARTED: 'info',
  };
  return map[status] || 'info';
};

/** 销售目标状态码 → 中文标签 */
export const getTargetStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    COMPLETED: '已完成',
    IN_PROGRESS: '进行中',
    PARTIAL: '部分完成',
    NOT_STARTED: '未开始',
  };
  return map[status] || status;
};
