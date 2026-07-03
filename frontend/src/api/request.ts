import axios from 'axios'
import type { AxiosError, AxiosInstance, AxiosRequestConfig, AxiosResponse } from 'axios'
import { ElMessage } from 'element-plus'
// Wave B-3：移除 access_token / refresh_token 的 localStorage 引用
// - 凭据由后端写入 httpOnly Cookie，前端 JS 不可读
// - 401 自动刷新通过 refresh_token Cookie 自动携带，无需前端取 token
import { loadCsrfToken, clearCsrfToken } from '@/utils/storage'
import router from '@/router'
import { refreshToken as refreshApi } from './auth'
import type { ApiResponse } from '@/types/api'

let isRefreshing = false
let refreshSubscribers: ((token: string) => void)[] = []

function subscribeTokenRefresh(cb: (token: string) => void) {
  refreshSubscribers.push(cb)
}

function onTokenRefreshed(token: string) {
  refreshSubscribers.forEach(cb => cb(token))
  refreshSubscribers = []
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
]

/**
 * 判断 URL 是否属于公开路径（不需要携带 X-CSRF-Token 头）
 * P3-3 修复（批次 84 v1 复审）：改为 startsWith 前缀匹配，避免子串误匹配
 * （原 includes 会将 /auth/login-xxx 误判为 /auth/login 公开路径）
 */
function isCsrfPublicPath(url: string): boolean {
  return CSRF_PUBLIC_PREFIXES.some(prefix => url === prefix || url.startsWith(prefix + '/'))
}

class Request {
  private instance: AxiosInstance

  constructor() {
    this.instance = axios.create({
      baseURL: import.meta.env.VITE_API_BASE_URL || '/api/v1/erp',
      timeout: 30000,
      // Wave B-3：开启凭据发送，使 httpOnly Cookie（access_token / refresh_token）能随请求到达后端
      // 这是 httpOnly Cookie 鉴权方案的**关键开关**：未开启则浏览器拒绝发送 Set-Cookie 之外的 Cookie
      withCredentials: true,
      headers: {
        'Content-Type': 'application/json',
        'X-Requested-With': 'XMLHttpRequest',
      },
    })

    this.setupInterceptors()
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
        const method = (config.method || 'get').toLowerCase()
        const url = config.url || ''
        if (
          method !== 'get' &&
          method !== 'head' &&
          method !== 'options' &&
          !isCsrfPublicPath(url)
        ) {
          const csrfToken = loadCsrfToken()
          if (csrfToken) {
            config.headers['X-CSRF-Token'] = csrfToken
          }
        }

        return config
      },
      error => {
        return Promise.reject(error)
      }
    )

    this.instance.interceptors.response.use(
      (response: AxiosResponse<ApiResponse>) => {
        const res = response.data
        if (res.code !== 200 && res.code !== 0) {
          const safeMessage = getSafeErrorMessage(res.code)
          ElMessage.error(safeMessage)
          if (res.code === 401) {
            // Wave B-3：凭据由后端 Cookie 管理，前端无需清理 localStorage；
            // 直接跳转登录页，后端会在登出时通过 Set-Cookie 清除 Cookie
            router.push('/login')
          }
          return Promise.reject(new Error(safeMessage))
        }
        // P2 1-11 修复：原 `return res as any` 丢失类型信息
        // 拦截器返回 ApiResponse 而非 AxiosResponse，用 unknown 断言满足 axios 类型系统
        return res as unknown as AxiosResponse
      },
      async error => {
        const originalRequest = error.config

        // 拦截 HTTP 403 + 业务码 CSRF 校验失败：清空 CSRF Token 并跳转登录
        // 后端在缺失/无效 CSRF Token 时返回 403 + code 字段（字符串），前端在错误拦截器识别
        if (error.response?.status === 403) {
          const body = error.response.data as { code?: string } | undefined
          if (body && (body.code === 'CSRF_TOKEN_MISSING' || body.code === 'CSRF_TOKEN_INVALID')) {
            // csrf_token Cookie 由后端管理；前端只能清空 document.cookie 中非 httpOnly 的 csrf_token
            // 真正彻底清理需调用 logout 接口或后端通过 Set-Cookie + max-age=0 清除
            clearCsrfToken()
            ElMessage.error('安全令牌已失效，请重新登录')
            router.push('/login')
            return Promise.reject(error)
          }
        }

        // Wave B-3：401 自动刷新流程
        // - 不再从前端取 refresh_token，浏览器会自动通过 httpOnly Cookie 发送
        // - 调 /auth/refresh 即可，后端会通过 Set-Cookie 头更新 access_token / csrf_token
        // - 重放时不需要重新注入 Authorization 头（Cookie 自动随 withCredentials=true 发送）
        if (error.response?.status === 401 && !originalRequest?._retry) {
          if (isRefreshing) {
            return new Promise(resolve => {
              subscribeTokenRefresh(() => {
                resolve(this.instance(originalRequest))
              })
            })
          }

          originalRequest._retry = true
          isRefreshing = true

          try {
            // 注意：refreshApi 内不应在请求体里带 refresh_token 字符串，
            // 因为后端已支持从 Cookie 读取；调用方传空字符串占位即可
            await refreshApi('')
            onTokenRefreshed('')
            return this.instance(originalRequest)
          } catch (refreshError) {
            router.push('/login')
            return Promise.reject(refreshError)
          } finally {
            isRefreshing = false
          }
        }

        if (originalRequest?._retry && shouldRetry(error)) {
          originalRequest._retryCount = originalRequest._retryCount || 0

          if (originalRequest._retryCount < 3) {
            originalRequest._retryCount++
            const delay = Math.min(1000 * originalRequest._retryCount + Math.random() * 1000, 5000)
            await new Promise(resolve => setTimeout(resolve, delay))
            return this.instance(originalRequest)
          }
        }

        const safeMessage = getSafeErrorMessage(error.response?.status)
        ElMessage.error(safeMessage)

        if (error.response?.status === 401) {
          router.push('/login')
        }
        return Promise.reject(error)
      }
    )
  }

  public get<T = unknown>(url: string, config?: AxiosRequestConfig): Promise<T> {
    // P2 1-11 修复：拦截器已返回 ApiResponse 完整对象（非 AxiosResponse.data），
    // 直接断言为 T，避免原 `res.data!` 丢失 ApiResponse 外层结构
    return this.instance.get(url, config).then(res => res as unknown as T)
  }

  public post<T = unknown>(url: string, data?: unknown, config?: AxiosRequestConfig): Promise<T> {
    return this.instance.post(url, data, config).then(res => res as unknown as T)
  }

  public put<T = unknown>(url: string, data?: unknown, config?: AxiosRequestConfig): Promise<T> {
    return this.instance.put(url, data, config).then(res => res as unknown as T)
  }

  public delete<T = unknown>(url: string, config?: AxiosRequestConfig): Promise<T> {
    return this.instance.delete(url, config).then(res => res as unknown as T)
  }

  public patch<T = unknown>(url: string, data?: unknown, config?: AxiosRequestConfig): Promise<T> {
    return this.instance.patch(url, data, config).then(res => res as unknown as T)
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
}

// P3-2 修复（批次 84 v1 复审）：error 类型从 any 改为 AxiosError，移除死分支 !error.response
// （原 !error.response 在 if (error.response) 后恒为 false，属于死代码）
export function shouldRetry(error: AxiosError): boolean {
  if (error.response) {
    return [502, 503, 504].includes(error.response.status)
  }
  // 无 response 时为网络错误或超时，按 code 判定
  return error.code === 'ECONNABORTED' || error.code === 'NETWORK_ERROR'
}

export function getSafeErrorMessage(codeOrStatus?: number): string {
  if (codeOrStatus && SAFE_ERROR_MESSAGES[codeOrStatus]) {
    return SAFE_ERROR_MESSAGES[codeOrStatus]
  }
  if (codeOrStatus === 401) {
    return '未授权，请重新登录'
  }
  return '请求失败，请稍后重试'
}

export const request = new Request()
