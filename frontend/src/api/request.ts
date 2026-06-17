import axios from 'axios'
import type { AxiosInstance, AxiosRequestConfig, AxiosResponse } from 'axios'
import { ElMessage } from 'element-plus'
import {
  getToken,
  removeToken,
  getRefreshToken,
  setToken,
} from '@/utils/storage'
import router from '@/router'
import { refreshToken as refreshApi, loadCsrfToken, clearCsrfToken } from './auth'
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
  '/auth/csrf-token',
  '/init',
  '/health',
  '/ready',
  '/live',
  '/tracking/page-view',
]

/**
 * 判断 URL 是否属于公开路径（不需要携带 X-CSRF-Token 头）
 */
function isCsrfPublicPath(url: string): boolean {
  return CSRF_PUBLIC_PREFIXES.some(prefix => url.includes(prefix))
}

class Request {
  private instance: AxiosInstance

  constructor() {
    this.instance = axios.create({
      baseURL: import.meta.env.VITE_API_BASE_URL || '/api/v1/erp',
      timeout: 30000,
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
        const token = getToken()
        if (token) {
          config.headers.Authorization = `Bearer ${token}`
        }

        // CSRF 防护：所有非安全方法（POST/PUT/PATCH/DELETE）的「业务」请求必须携带 X-CSRF-Token
        // - 公开路径（login/refresh/health 等）跳过，由后端白名单控制
        // - 安全方法（GET/HEAD/OPTIONS）无需校验
        // - 若本地没有 CSRF Token（如刷新流程未拿到），由后端返回 403，前端在此处清空并跳转登录
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
            removeToken()
            router.push('/login')
          }
          return Promise.reject(new Error(safeMessage))
        }
        return res as any
      },
      async error => {
        const originalRequest = error.config

        // 拦截 HTTP 403 + 业务码 CSRF 校验失败：清空 CSRF Token 并跳转登录
        // 后端在缺失/无效 CSRF Token 时返回 403 + code 字段（字符串），前端在错误拦截器识别
        if (error.response?.status === 403) {
          const body = error.response.data as { code?: string } | undefined
          if (body && (body.code === 'CSRF_TOKEN_MISSING' || body.code === 'CSRF_TOKEN_INVALID')) {
            clearCsrfToken()
            removeToken()
            ElMessage.error('安全令牌已失效，请重新登录')
            router.push('/login')
            return Promise.reject(error)
          }
        }

        if (error.response?.status === 401 && !originalRequest?._retry) {
          if (isRefreshing) {
            return new Promise(resolve => {
              subscribeTokenRefresh(token => {
                originalRequest.headers.Authorization = `Bearer ${token}`
                resolve(this.instance(originalRequest))
              })
            })
          }

          originalRequest._retry = true
          isRefreshing = true

          try {
            const refreshToken = getRefreshToken()
            if (!refreshToken) {
              throw new Error('No refresh token')
            }

            const tokenData = await refreshApi(refreshToken)
            setToken(tokenData.token)
            onTokenRefreshed(tokenData.token)
            originalRequest.headers.Authorization = `Bearer ${tokenData.token}`
            return this.instance(originalRequest)
          } catch (refreshError) {
            removeToken()
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
          removeToken()
          router.push('/login')
        }
        return Promise.reject(error)
      }
    )
  }

  public get<T = unknown>(url: string, config?: AxiosRequestConfig): Promise<T> {
    return this.instance.get(url, config).then(res => res.data!)
  }

  public post<T = unknown>(url: string, data?: any, config?: AxiosRequestConfig): Promise<T> {
    return this.instance.post(url, data, config).then(res => res.data!)
  }

  public put<T = unknown>(url: string, data?: any, config?: AxiosRequestConfig): Promise<T> {
    return this.instance.put(url, data, config).then(res => res.data!)
  }

  public delete<T = unknown>(url: string, config?: AxiosRequestConfig): Promise<T> {
    return this.instance.delete(url, config).then(res => res.data!)
  }

  public patch<T = unknown>(url: string, data?: any, config?: AxiosRequestConfig): Promise<T> {
    return this.instance.patch(url, data, config).then(res => res.data!)
  }
}

const SAFE_ERROR_MESSAGES: Record<number, string> = {
  400: '请求参数错误',
  401: '未授权，请重新登录',
  403: '拒绝访问',
  404: '资源不存在',
  429: '请求过于频繁',
  500: '服务器内部错误',
  502: '网关错误',
  503: '服务暂时不可用',
}

function shouldRetry(error: any): boolean {
  if (error.response) {
    return [502, 503, 504].includes(error.response.status)
  }
  return error.code === 'ECONNABORTED' || error.code === 'NETWORK_ERROR' || !error.response
}

function getSafeErrorMessage(codeOrStatus?: number): string {
  if (codeOrStatus && SAFE_ERROR_MESSAGES[codeOrStatus]) {
    return SAFE_ERROR_MESSAGES[codeOrStatus]
  }
  if (codeOrStatus === 401) {
    return '未授权，请重新登录'
  }
  return '请求失败，请稍后重试'
}

export const request = new Request()
