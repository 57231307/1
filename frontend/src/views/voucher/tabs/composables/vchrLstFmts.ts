/**
 * vchrLstFmts.ts - 凭证列表格式化工具
 * 任务编号: P14 批 2 I-3 第 1 批
 * 提供状态标签/类型映射/格式化金额/凭证类型选项等纯函数
 *
 * P0 契约修复（本轮，三端同源）：
 * - 状态词表对齐后端 crate::models::status::finance::voucher（批次 102 v6 P3-1）：
 *   draft→submitted→reviewed→posted；原前端自创 'approved' 后端从不产出 ⇒ 状态列/
 *   过滤/按钮判断全部失真。
 * - 凭证类型不在前端维护第二套常量：原 general/customized 自创词表删除，
 *   展示直接使用后端 voucher_type 值（词表单一真源 = VoucherService::available_voucher_types，
 *   下拉选项经 GET /vouchers/types 获取）。
 */

/** 状态 → 中文标签（后端 status::finance::voucher 词表） */
const STATUS_LABEL_MAP: Record<string, string> = {
  draft: '草稿',
  submitted: '已提交',
  reviewed: '已审核',
  posted: '已记账',
};

/** 状态 → CSS 类名 */
const STATUS_CLASS_MAP: Record<string, string> = {
  draft: 'status-draft',
  submitted: 'status-submitted',
  reviewed: 'status-reviewed',
  posted: 'status-posted',
};

/** 状态过滤下拉选项（''=全部） */
export const STATUS_OPTIONS = [
  { label: '全部', value: '' },
  { label: '草稿', value: 'draft' },
  { label: '已提交', value: 'submitted' },
  { label: '已审核', value: 'reviewed' },
  { label: '已记账', value: 'posted' },
];

/** 获取状态中文标签 */
export const getStatusLabel = (value: string) => STATUS_LABEL_MAP[value] || value;

/** 获取状态 CSS 类名 */
export const getStatusClass = (value: string) => STATUS_CLASS_MAP[value] || '';

/**
 * 金额格式化（保留 2 位小数）。
 * 后端 rust_decimal 序列化为字符串，入参统一 Number() 归一（nullish 视为 0）。
 */
export const formatAmount = (amount: number | string | null | undefined) => {
  return (Number(amount ?? 0) || 0).toFixed(2);
};
