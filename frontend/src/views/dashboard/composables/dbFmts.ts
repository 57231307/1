// Dashboard 格式化工具集合
// 拆分自 Dashboard.vue（P14 批 2 I-3 第 6 批）
// 行为完全保持一致（仅结构重构）

/** 数字千分位格式化（后端金额/数量为字符串，一并接受） */
export const formatNumber = (num: number | string | undefined) => {
  const n = typeof num === 'string' ? Number(num) : num;
  if (!n) return '0';
  return n.toLocaleString();
};

/** 货币格式化（Intl 人民币，后端金额为字符串） */
export const formatCurrency = (amount: number | string | undefined) => {
  const n = typeof amount === 'string' ? Number(amount) : amount;
  if (!n) return '¥0';
  return new Intl.NumberFormat('zh-CN', {
    style: 'currency',
    currency: 'CNY',
    minimumFractionDigits: 0,
  }).format(n);
};

/** 活动类型 → ElTag type */
export const getActivityTypeColor = (type: string) => {
  const typeMap: Record<string, string> = {
    订单: 'success',
    采购: 'warning',
    库存: 'info',
    审批: 'primary',
    系统: 'danger',
  };
  return typeMap[type] || 'info';
};
