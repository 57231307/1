// sales-analysis 格式化工具集合
// 拆分自 sales-analysis/index.vue（P14 批 2 I-3 第 6 批）
// 行为完全保持一致（仅结构重构）

/**
 * 格式化货币：rust_decimal 出参为字符串（如 "1250.00"），统一 Number() 归一后再格式化；
 * 直接对字符串 .toFixed 会 TypeError，禁止假定入参是 number。
 */
export const formatCurrency = (value: number | string) => {
  const n = Number(value);
  return n ? `¥${n.toFixed(2)}` : '¥0.00';
};

/** 根据完成率返回进度条颜色（completion_rate 为 Decimal 出参字符串，先归一） */
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
