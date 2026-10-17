/**
 * 打印任务状态（print_tasks.status）词表与 i18n 文案键映射。
 *
 * 唯一真相是写入方 `backend/src/models/status/print.rs` 的 `print_task_status::ALL`
 * （小写英文闭合五值，与建表迁移 chk_print_tasks_status 逐项一致）。本文件只做两件事：
 * 把 token 映射到中英齐全的展示文案键，以及在渲染前拦下词表外的值（抛错暴露，不猜含义）。
 * 后端增删取值必须同步本数组与两份语言包，否则界面直接显示原始键名而不是假翻译。
 */

/** 与后端 print_task_status::ALL 逐字符同序的取值集 */
export const PRINT_TASK_STATUSES = [
  'pending',
  'queued',
  'dispatched',
  'succeeded',
  'failed',
] as const;

export type PrintTaskStatusToken = (typeof PRINT_TASK_STATUSES)[number];

/** 终态：到达其一即不再轮询（其余三个为在途态） */
export const PRINT_TASK_TERMINAL_STATUSES: PrintTaskStatusToken[] = ['succeeded', 'failed'];

export function isPrintTaskTerminal(status: string): boolean {
  return PRINT_TASK_TERMINAL_STATUSES.includes(status as PrintTaskStatusToken);
}

/** 词表外 token 直接抛错，避免把后端词表漂移压成界面静默错译 */
export function normalizePrintTaskStatus(status: unknown): PrintTaskStatusToken {
  const token = String(status ?? '');
  if (!(PRINT_TASK_STATUSES as readonly string[]).includes(token)) {
    throw new Error(`未知的打印任务状态: ${token}`);
  }
  return token as PrintTaskStatusToken;
}

/**
 * 状态 token 到 i18n 文案键的映射。键路径写成完整字面量（不用动态拼接）：
 * check-i18n 只认字面量引用，拼出来的 `printTaskStatus.${x}` 会把这五条文案判成孤儿键。
 */
const LABEL_KEY_BY_STATUS: Record<PrintTaskStatusToken, string> = {
  pending: 'printTaskStatus.pending',
  queued: 'printTaskStatus.queued',
  dispatched: 'printTaskStatus.dispatched',
  succeeded: 'printTaskStatus.succeeded',
  failed: 'printTaskStatus.failed',
};

/** 状态文案键（词表外 token 由 normalize 先抛错，不会走到取键这一步） */
export function printTaskStatusLabelKey(status: unknown): string {
  return LABEL_KEY_BY_STATUS[normalizePrintTaskStatus(status)];
}

/** 状态标签色：在途态统一 warning，成功 success，失败 danger */
export function printTaskStatusTagType(status: unknown): 'warning' | 'success' | 'danger' {
  const token = normalizePrintTaskStatus(status);
  if (token === 'succeeded') return 'success';
  if (token === 'failed') return 'danger';
  return 'warning';
}
