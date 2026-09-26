/**
 * 统一日志工具
 * 在开发环境输出日志，生产环境自动禁用
 */

type LogLevel = 'debug' | 'info' | 'warn' | 'error';

class Logger {
  private enabled: boolean;
  private level: LogLevel;

  constructor() {
    this.enabled = import.meta.env.DEV;
    this.level = 'debug';
  }

  private shouldLog(level: LogLevel): boolean {
    if (!this.enabled) return false;
    const levels: LogLevel[] = ['debug', 'info', 'warn', 'error'];
    return levels.indexOf(level) >= levels.indexOf(this.level);
  }

  debug(message: string, ...args: unknown[]): void {
    if (this.shouldLog('debug')) {
      console.debug(`[DEBUG] ${message}`, ...args);
    }
  }

  info(message: string, ...args: unknown[]): void {
    if (this.shouldLog('info')) {
      console.info(`[INFO] ${message}`, ...args);
    }
  }

  warn(message: string, ...args: unknown[]): void {
    if (this.shouldLog('warn')) {
      console.warn(`[WARN] ${message}`, ...args);
    }
  }

  error(message: string, ...args: unknown[]): void {
    // V15 P1-20-10 错误始终输出（生产环境也需要错误日志用于监控）
    console.error(`[ERROR] ${message}`, ...args);
  }
}

/**
 * 辅助下拉数据（用户/角色/部门等）加载失败的分级留痕。
 *
 * 403 表示当前角色本就无该接口权限，属权限下界正常生效：用户提示已由
 * axios 拦截器统一给出，这里降为 warn 级，避免把「无权限」当成应用缺陷。
 * 其余失败（5xx、超时、响应结构异常）是真实缺陷，保持 error 级始终输出。
 */
export function logAuxLoadFailure(message: string, error: unknown): void {
  const status = (error as { response?: { status?: number } } | undefined)?.response?.status;
  if (status === 403) logger.warn(message, error);
  else logger.error(message, error);
}

export const logger = new Logger();
// 同时导出 default 以兼容 `import logger from '@/utils/logger'`
export default logger;
