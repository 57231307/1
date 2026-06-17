import { request } from './request'
import type { LoginRequest, LoginResponse, UserInfo } from '@/types/api'

// CSRF Token 在 localStorage 中的存储 key
// 命名遵循项目约定：所有 storage key 集中在此文件内
const CSRF_TOKEN_KEY = 'csrf_token'

/**
 * 保存 CSRF Token 到 localStorage
 * 登录成功与 refresh 成功后调用，供后续非安全方法请求使用
 */
function saveCsrfToken(token: string): void {
  localStorage.setItem(CSRF_TOKEN_KEY, token)
}

/**
 * 从 localStorage 读取 CSRF Token
 */
function loadCsrfToken(): string | null {
  return localStorage.getItem(CSRF_TOKEN_KEY)
}

/**
 * 从 localStorage 删除 CSRF Token
 * 登出或 CSRF 校验失败时调用
 */
function clearCsrfToken(): void {
  localStorage.removeItem(CSRF_TOKEN_KEY)
}

export function login(data: LoginRequest): Promise<LoginResponse> {
  return request.post('/auth/login', data).then(res => {
    // 登录成功后保存 CSRF Token，供后续非安全方法请求使用
    if (res && res.csrf_token) {
      saveCsrfToken(res.csrf_token)
    }
    return res
  })
}

export function logout(): Promise<void> {
  // 登出时清除本地 CSRF Token
  clearCsrfToken()
  return request.post('/auth/logout')
}

export function refreshToken(refreshToken: string): Promise<{ token: string; csrf_token?: string }> {
  return request.post('/auth/refresh', { refresh_token: refreshToken }).then(res => {
    // 刷新 Token 后同步更新 CSRF Token（rotation 模式：旧 token 已被消费）
    if (res && res.csrf_token) {
      saveCsrfToken(res.csrf_token)
    }
    return res
  })
}

export function getUserInfo(): Promise<UserInfo> {
  return request.get('/auth/me')
}

// 导出 CSRF Token 工具，供 request.ts 中的 axios 拦截器使用
export { loadCsrfToken, clearCsrfToken }
