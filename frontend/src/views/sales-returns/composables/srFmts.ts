/**
 * srFmts.ts - 销售退货格式化工具
 * 状态词表、el-tag 配色与标签键的单一映射源在 utils/sales-return-status.ts。
 */

/** 格式化退货金额 */
export const formatAmount = (value: number) => {
  return value !== undefined && value !== null ? `¥${Number(value).toFixed(2)}` : '¥0.00';
};
