import axios from 'axios';
import type { AxiosError, AxiosInstance, AxiosRequestConfig, AxiosResponse } from 'axios';
import { ElMessage } from 'element-plus';
import { msg } from '@/utils/message';
// Wave B-3：移除 access_token / refresh_token 的 localStorage 引用
// - 凭据由后端写入 httpOnly Cookie，前端 JS 不可读
// - 401 自动刷新通过 refresh_token Cookie 自动携带，无需前端取 token
import { loadCsrfToken, clearCsrfToken } from '@/utils/storage';
import router from '@/router';
import { refreshToken as refreshApi } from './auth';
import type { ApiResponse } from '@/types/api';

let isRefreshing = false;
// FE-P1-1 修复（v13 前端审计）：排队请求同时持有 resolve 和 reject 回调，
// 刷新失败时能通知排队请求 reject，避免 Promise 永不 settle 导致 loading 状态/闭包泄漏
let refreshSubscribers: Array<{
  resolve: (token: string) => void;
  reject: (error: unknown) => void;
}> = [];

function subscribeTokenRefresh(resolve: (token: string) => void, reject: (error: unknown) => void) {
  refreshSubscribers.push({ resolve, reject });
}

function onTokenRefreshed(token: string) {
  refreshSubscribers.forEach(({ resolve }) => resolve(token));
  refreshSubscribers = [];
}

// FE-P1-1 修复：刷新失败时通知所有排队请求 reject，避免 Promise 永不 settle
function onTokenRefreshFailed(error: unknown) {
  refreshSubscribers.forEach(({ reject }) => reject(error));
  refreshSubscribers = [];
}

// V15 P2 20.2-D：错误消息去重，避免短时间内弹出多条相同错误提示
const _recentErrors = new Set<string>();
// 网络断开兜底文案只弹一次，直到有请求成功（证明网络恢复）再重置
let _networkErrorToastShown = false;
function showErrorOnce(message: string): void {
  if (_recentErrors.has(message)) return;
  _recentErrors.add(message);
  ElMessage.error(message);
  setTimeout(() => _recentErrors.delete(message), 2000);
}

/**
 * 从后端错误响应体中提取 message。
 *
 * 后端 AppError 出参的 message 默认即为脱敏常量（如「业务处理失败」），
 * 仅当构造点显式声明可外显（AppError::business_displayable）时才是真实业务文案
 * （见 backend/src/utils/error.rs 模块文档的安全边界）。
 * 因此这里优先展示后端 message，不再用前端固定文案覆盖它；
 * message 缺失/非字符串/空白时才回退到按 HTTP 状态码映射的固定文案。
 */
function extractBackendMessage(data: unknown): string | undefined {
  if (data && typeof data === 'object' && !Array.isArray(data)) {
    const m = (data as { message?: unknown }).message;
    if (typeof m === 'string' && m.trim() !== '') return m;
  }
  return undefined;
}

/**
 * 不需要携带 CSRF Token 的公开路径前缀（前缀匹配）
 * 这些端点在后端 CSRF 中间件中已加入白名单，前端无需注入头
 */
const CSRF_PUBLIC_PREFIXES = [
  '/auth/login',
  '/auth/refresh',
  '/auth/logout',
  // P3 7-17 修复：删除 /auth/csrf-token（接口已删除，CSRF token 通过 login/refresh Set-Cookie 下发）
  '/init',
  '/health',
  '/ready',
  '/live',
  '/tracking/page-view',
];

/**
 * 判断 URL 是否属于公开路径（不需要携带 X-CSRF-Token 头）
 * P3-3 修复（批次 84 v1 复审）：改为 startsWith 前缀匹配，避免子串误匹配
 * （原 includes 会将 /auth/login-xxx 误判为 /auth/login 公开路径）
 */
function isCsrfPublicPath(url: string): boolean {
  return CSRF_PUBLIC_PREFIXES.some(prefix => url === prefix || url.startsWith(prefix + '/'));
}

/**
 * 查询参数序列化：未填写的筛选项（空串/纯空白）不进入 query string。
 *
 * 与后端「空串查询参数在边界视为未提供」是同一契约的两端。列表页未选的筛选项会以
 * `?keyword=`/`?status=` 形态提交，若进入 query 会被后端反序列化成 Some("") 并生成
 * `WHERE col = ''` 恒 0 行。此处统一剔除空值键，从源头避免该缺陷复发。
 * 注意：数字 0 与布尔 false 是有效取值，绝不因「假值」被丢弃；仅空串/纯空白被剔除。
 */
function serializeParams(params: Record<string, unknown>): string {
  const search = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value === undefined || value === null) continue;
    if (Array.isArray(value)) {
      for (const item of value) {
        if (item === undefined || item === null) continue;
        if (typeof item === 'string' && item.trim() === '') continue;
        search.append(key, String(item));
      }
      continue;
    }
    if (typeof value === 'string' && value.trim() === '') continue;
    search.append(key, String(value));
  }
  return search.toString();
}

class Request {
  private instance: AxiosInstance;

  constructor() {
    this.instance = axios.create({
      baseURL: import.meta.env.VITE_API_BASE_URL || '/api/v1/erp',
      timeout: 30000,
      // Wave B-3：开启凭据发送，使 httpOnly Cookie（access_token / refresh_token）能随请求到达后端
      // 这是 httpOnly Cookie 鉴权方案的**关键开关**：未开启则浏览器拒绝发送 Set-Cookie 之外的 Cookie
      withCredentials: true,
      paramsSerializer: { serialize: serializeParams },
      headers: {
        'Content-Type': 'application/json',
        'X-Requested-With': 'XMLHttpRequest',
      },
    });

    this.setupInterceptors();
  }

  private setupInterceptors() {
    this.instance.interceptors.request.use(
      config => {
        // Wave B-3：不再手动注入 Authorization 头。
        // 凭据由 httpOnly Cookie 在 withCredentials=true 时由浏览器自动发送，
        // 配合后端 auth 中间件（Cookie 优先 → 旧 jwt Cookie → Authorization 头）实现无感鉴权。

        // CSRF 防护：所有非安全方法（POST/PUT/PATCH/DELETE）的「业务」请求必须携带 X-CSRF-Token
        // - 公开路径（login/refresh/health 等）跳过，由后端白名单控制
        // - 安全方法（GET/HEAD/OPTIONS）无需校验
        // - csrf_token 由后端以非 httpOnly Cookie 形式下发，前端从 document.cookie 读取后注入头
        const method = (config.method || 'get').toLowerCase();
        const url = config.url || '';
        if (
          method !== 'get' &&
          method !== 'head' &&
          method !== 'options' &&
          !isCsrfPublicPath(url)
        ) {
          const csrfToken = loadCsrfToken();
          if (csrfToken) {
            config.headers['X-CSRF-Token'] = csrfToken;
          } else {
            // batch-20 P3: CSRF token 缺失时提示用户
            console.warn('[CSRF] 安全令牌缺失，请求可能被后端拒绝');
            // 尝试刷新页面获取新的 CSRF token
            if (typeof window !== 'undefined') {
              console.warn('[CSRF] 建议刷新页面获取新的安全令牌');
            }
          }
        }

        return config;
      },
      error => {
        return Promise.reject(error);
      }
    );

    this.instance.interceptors.response.use(
      (response: AxiosResponse<ApiResponse>) => {
        // 任意成功响应证明网络恢复，重置网络断开兜底文案的一次性提示标记
        _networkErrorToastShown = false;
        // Blob 响应（文件下载/导出）无 code 信封，直接放行交给调用方处理二进制
        if (response.config.responseType === 'blob' || response.data instanceof Blob) {
          return response;
        }
        // 2xx 响应体即成功：后端成功信封的 code 恒为 200，失败一律以非 2xx 状态码返回并
        // 出参统一失败信封 `{code: "<字符串机器码>", message, trace_id, timestamp}`
        // （backend/src/utils/error.rs），因此这里不存在"200 里夹带业务错误码"的分支，
        // 业务/认证/权限失败全部由下方 error 拦截器处理。
        return response.data as unknown as AxiosResponse;
      },
      async error => {
        const originalRequest = error.config;

        // 拦截 HTTP 403 + 业务码 CSRF 校验失败：CSRF Token 为一次性消费，
        // 多标签页/并发请求共享同一 csrf_token Cookie 时，后发请求必然携带已消费的旧 token。
        // 恢复顺序：
        // 1. 后端在消费失败时通过 X-New-CSRF-Token 头下发了恢复 token → 写入 Cookie 后用它重放
        // 2. 无恢复头时读取最新 Cookie 中的 token 重放一次（带 _csrfRetry 标记防循环）
        // 仍失败才清空 token 并跳转登录，避免并发请求把用户误踢出登录态。
        if (error.response?.status === 403) {
          const body = error.response.data as { code?: string } | undefined;
          if (body && (body.code === 'CSRF_TOKEN_MISSING' || body.code === 'CSRF_TOKEN_INVALID')) {
            const isCsrfRetry = (originalRequest as { _csrfRetry?: boolean } | undefined)
              ?._csrfRetry;
            // 优先使用后端下发的恢复 token（并发竞败场景的权威来源）
            const recoveryToken = error.response.headers?.['x-new-csrf-token'] as
              string | undefined;
            if (!isCsrfRetry && recoveryToken) {
              document.cookie = `csrf_token=${recoveryToken}; Path=/; SameSite=Strict; Max-Age=1800`;
              (originalRequest as { _csrfRetry?: boolean })._csrfRetry = true;
              originalRequest.headers['X-CSRF-Token'] = recoveryToken;
              return this.instance(originalRequest);
            }
            const freshToken = loadCsrfToken();
            if (!isCsrfRetry && freshToken) {
              (originalRequest as { _csrfRetry?: boolean })._csrfRetry = true;
              originalRequest.headers['X-CSRF-Token'] = freshToken;
              return this.instance(originalRequest);
            }
            // csrf_token Cookie 由后端管理；前端只能清空 document.cookie 中非 httpOnly 的 csrf_token
            // 真正彻底清理需调用 logout 接口或后端通过 Set-Cookie + max-age=0 清除
            clearCsrfToken();
            msg.error('securityTokenExpired');
            router.push('/login');
            return Promise.reject(error);
          }
        }

        // Wave B-3：401 自动刷新流程
        // - 不再从前端取 refresh_token，浏览器会自动通过 httpOnly Cookie 发送
        // - 调 /auth/refresh 即可，后端会通过 Set-Cookie 头更新 access_token / csrf_token
        // - 重放时不需要重新注入 Authorization 头（Cookie 自动随 withCredentials=true 发送）
        if (
          error.response?.status === 401 &&
          !originalRequest?._retry &&
          !(originalRequest as any)?._skipAuthRetry
        ) {
          if (isRefreshing) {
            // FE-P1-1 修复：排队请求同时持有 resolve/reject，
            // 刷新成功走 resolve 重放，刷新失败走 reject 让 Promise settle
            // _retry 前置标记：防止重放后再次 401 时重新进入刷新逻辑形成循环
            originalRequest._retry = true;
            return new Promise((resolve, reject) => {
              subscribeTokenRefresh(() => resolve(this.instance(originalRequest)), reject);
            });
          }

          originalRequest._retry = true;
          isRefreshing = true;

          try {
            // 注意：refreshApi 内不应在请求体里带 refresh_token 字符串，
            // 因为后端已支持从 Cookie 读取；调用方传空字符串占位即可
            await refreshApi('');
            onTokenRefreshed('');
            return this.instance(originalRequest);
          } catch (refreshError) {
            // FE-P1-1 修复：刷新失败时通知所有排队请求 reject，避免 Promise 永不 settle
            onTokenRefreshFailed(refreshError);
            router.push('/login');
            return Promise.reject(refreshError);
          } finally {
            isRefreshing = false;
          }
        }

        // 网络层自动重试仅限幂等方法（GET/HEAD）：
        // POST/PUT/DELETE 重试可能造成重复制单/重复扣减，必须由上层带幂等键显式控制
        // 触发条件用 isIdempotent + shouldRetry，而非 _retry 标记：_retry 仅由 401 刷新链路置位，
        // 普通网络错误（无 response / 5xx 网关态）若仍卡 _retry 则该重试分支永不进入，
        // 退化为「首次失败即弹提示」，叠加 useTableApi 外层重试导致同一断网文案重复弹出。
        const reqMethod = (originalRequest?.method || '').toLowerCase();
        const isIdempotent = ['get', 'head', 'options'].includes(reqMethod);
        if (isIdempotent && shouldRetry(error)) {
          originalRequest._retryCount = originalRequest._retryCount || 0;

          if (originalRequest._retryCount < 3) {
            originalRequest._retryCount++;
            const delay = Math.min(1000 * originalRequest._retryCount + Math.random() * 1000, 5000);
            await new Promise(resolve => setTimeout(resolve, delay));
            return this.instance(originalRequest);
          }
        }

        // 优先展示后端 message（AppError 出参默认即脱敏常量，显式外显时为真实业务文案），
        // 缺失时回退按 HTTP 状态码映射的固定文案
        const safeMessage =
          extractBackendMessage(error.response?.data) ??
          getSafeErrorMessage(error.response?.status);
        // 纯网络错误（无 HTTP 响应）经拦截器 3 次退避重试穷尽后，断开期间同一兜底文案只弹一次，
        // 直到任意请求成功（_networkErrorToastShown 在成功拦截器复位）。
        // 这避免上层（useTableApi 等）继续重试时再次落入此处造成重复 toast。
        if (!error.response) {
          if (_networkErrorToastShown) {
            return Promise.reject(error);
          }
          _networkErrorToastShown = true;
        }
        showErrorOnce(safeMessage);

        if (error.response?.status === 401) {
          router.push('/login');
        }
        return Promise.reject(error);
      }
    );
  }

  public get<T = unknown>(url: string, config?: AxiosRequestConfig): Promise<T> {
    // P2 1-11 修复：拦截器已返回 ApiResponse 完整对象（非 AxiosResponse.data），
    // 直接断言为 T，避免原 `res.data!` 丢失 ApiResponse 外层结构
    return this.instance.get(url, config).then(res => res as unknown as T);
  }

  public post<T = unknown>(url: string, data?: unknown, config?: AxiosRequestConfig): Promise<T> {
    return this.instance.post(url, data, config).then(res => res as unknown as T);
  }

  public put<T = unknown>(url: string, data?: unknown, config?: AxiosRequestConfig): Promise<T> {
    return this.instance.put(url, data, config).then(res => res as unknown as T);
  }

  public delete<T = unknown>(url: string, config?: AxiosRequestConfig): Promise<T> {
    return this.instance.delete(url, config).then(res => res as unknown as T);
  }

  public patch<T = unknown>(url: string, data?: unknown, config?: AxiosRequestConfig): Promise<T> {
    return this.instance.patch(url, data, config).then(res => res as unknown as T);
  }
}

export const SAFE_ERROR_MESSAGES: Record<number, string> = {
  400: '请求参数错误',
  401: '未授权，请重新登录',
  403: '拒绝访问',
  404: '资源不存在',
  429: '请求过于频繁',
  500: '服务器内部错误',
  502: '网关错误',
  503: '服务暂时不可用',
};

// P3-2 修复（批次 84 v1 复审）：error 类型从 any 改为 AxiosError，简化 else 分支
// 原 `!error.response` 在 else 分支恒为 true（死代码），此处显式表达"无 response 时默认重试"
export function shouldRetry(error: AxiosError): boolean {
  if (error.response) {
    return [502, 503, 504].includes(error.response.status);
  }
  // 无 response 时为网络错误或超时（ECONNABORTED / NETWORK_ERROR 等），默认重试
  return true;
}

export function getSafeErrorMessage(codeOrStatus?: number): string {
  if (codeOrStatus && SAFE_ERROR_MESSAGES[codeOrStatus]) {
    return SAFE_ERROR_MESSAGES[codeOrStatus];
  }
  if (codeOrStatus === 401) {
    return '未授权，请重新登录';
  }
  return '请求失败，请稍后重试';
}

export const request = new Request();
