/**
 * V15 P1-20-10 前端错误监控 SDK（审计 batch-20 维度 24.14 缺陷 2）
 *
 * 业务背景：审计报告指出未接入前端监控，生产环境错误无法实时告警与归因。
 * 当前未引入 Sentry/Bugsnag 等第三方 SDK，采用自研轻量监控方案。
 *
 * 能力：
 * - 监听 window error 事件捕获运行时异常
 * - 监听 unhandledrejection 事件捕获未处理的 Promise rejection
 * - 5 分钟内相同错误指纹去重（避免上报风暴）
 * - 上报到后端 /api/v1/erp/tracking/frontend-error 接口（best-effort，失败静默）
 * - 同时通过 logger 本地记录（开发环境可见）
 *
 * 用法：
 * ```ts
 * import { initMonitor } from '@/utils/monitor'
 * initMonitor()  // 在 main.ts 中调用一次
 * ```
 */
import { logger } from '@/utils/logger';

/** 错误上报载荷 */
interface ErrorReport {
  /** 错误类型：error / unhandledrejection / manual */
  type: string;
  /** 错误消息 */
  message: string;
  /** 错误源（script URL） */
  source?: string;
  /** 错误行号 */
  lineno?: number;
  /** 错误列号 */
  colno?: number;
  /** 错误堆栈 */
  stack?: string;
  /** 发生页面 URL */
  url: string;
  /** 用户代理 */
  userAgent: string;
  /** 时间戳（ISO 8601） */
  timestamp: string;
}

/** 去重窗口：5 分钟（毫秒） */
const DEDUP_WINDOW_MS = 5 * 60 * 1000;

/** 错误指纹最近上报时间映射 */
const errorFingerprints = new Map<string, number>();

/** 上报端点 */
const REPORT_ENDPOINT = '/api/v1/erp/tracking/frontend-error';

/** 生成错误指纹（type + message + source 哈希） */
function fingerprint(report: Pick<ErrorReport, 'type' | 'message' | 'source'>): string {
  return `${report.type}::${report.message}::${report.source || ''}`;
}

/** 判断错误是否在去重窗口内 */
function isDuplicate(fp: string): boolean {
  const now = Date.now();
  const lastSeen = errorFingerprints.get(fp);
  if (lastSeen && now - lastSeen < DEDUP_WINDOW_MS) {
    return true;
  }
  errorFingerprints.set(fp, now);
  return false;
}

/** 上报错误到后端（best-effort，失败静默不影响用户） */
function reportError(report: ErrorReport): void {
  // 本地日志记录（开发环境可见）
  logger.error('[Frontend Monitor]', {
    type: report.type,
    message: report.message,
    stack: report.stack,
    url: report.url,
  });

  // 去重检查
  const fp = fingerprint(report);
  if (isDuplicate(fp)) return;

  // 上报到后端（fetch + catch，不阻塞主流程）
  try {
    fetch(REPORT_ENDPOINT, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify(report),
      keepalive: true,
    }).catch(() => {
      // 上报失败静默处理（不抛出二次错误）
    });
  } catch {
    // fetch 本身不可用时静默
  }
}

/** 手动上报错误 */
export function captureError(error: Error | string, context?: Record<string, unknown>): void {
  const message = typeof error === 'string' ? error : error.message;
  const stack = typeof error === 'string' ? undefined : error.stack;
  reportError({
    type: 'manual',
    message,
    stack,
    url: window.location.href,
    userAgent: navigator.userAgent,
    timestamp: new Date().toISOString(),
    ...context,
  });
}

/** 初始化前端错误监控 */
export function initMonitor(): void {
  if (typeof window === 'undefined') return;

  // 监听运行时错误（script error、undefined is not a function 等）
  window.addEventListener('error', (event: ErrorEvent) => {
    reportError({
      type: 'error',
      message: event.message || 'Unknown error',
      source: event.filename,
      lineno: event.lineno,
      colno: event.colno,
      stack: event.error?.stack,
      url: window.location.href,
      userAgent: navigator.userAgent,
      timestamp: new Date().toISOString(),
    });
  });

  // 监听未处理的 Promise rejection
  window.addEventListener('unhandledrejection', (event: PromiseRejectionEvent) => {
    const reason = event.reason;
    const message = reason instanceof Error ? reason.message : String(reason);
    const stack = reason instanceof Error ? reason.stack : undefined;
    reportError({
      type: 'unhandledrejection',
      message,
      stack,
      url: window.location.href,
      userAgent: navigator.userAgent,
      timestamp: new Date().toISOString(),
    });
  });

  logger.info('[Frontend Monitor] initialized');
}

export default { initMonitor, captureError };
