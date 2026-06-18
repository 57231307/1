import { request } from './request'
import type { ApiResponse, LoginRequest, LoginResponse, UserInfo } from '@/types/api'

// CSRF Token 在 localStorage 中的存储 key
// 命名遵循项目约定：所有 storage key 集中在此文件内
const CSRF_TOKEN_KEY = 'csrf_token'

/**
 * 登录响应扩展：包含后端返回的 CSRF Token（用于后续非安全方法请求）
 * 通过本地交叉类型扩展，避免修改全局类型定义
 */
type LoginResponseWithCsrf = LoginResponse & { csrf_token?: string }

/**
 * 刷新 Token 响应：包含新 access_token 与可选的 CSRF Token
 */
interface RefreshTokenResponse {
  token: string
  csrf_token?: string
}

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
  return request.post<LoginResponseWithCsrf>('/auth/login', data).then(res => {
    // 登录成功后保存 CSRF Token，供后续非安全方法请求使用
    if (res && res.csrf_token) {
      saveCsrfToken(res.csrf_token)
    }
    // 转换为标准 LoginResponse（去除 csrf_token 字段）
    const { csrf_token: _csrf, ...payload } = res
    void _csrf
    return payload
  })
}

export function logout(): Promise<void> {
  // 登出时清除本地 CSRF Token
  clearCsrfToken()
  return request.post<void>('/auth/logout')
}

export function refreshToken(refreshToken: string): Promise<{ token: string; csrf_token?: string }> {
  return request.post<RefreshTokenResponse>('/auth/refresh', { refresh_token: refreshToken }).then(res => {
    // 刷新 Token 后同步更新 CSRF Token（rotation 模式：旧 token 已被消费）
    if (res && res.csrf_token) {
      saveCsrfToken(res.csrf_token)
    }
    return res
  })
}

export function getUserInfo(): Promise<UserInfo> {
  return request.get<UserInfo>('/auth/me')
}

/**
 * TOTP 设置响应：包含 base32 编码的密钥与后端已生成好的 base64 QR 码 PNG
 * 后端使用 totp_rs 的 get_qr_base64() 直接产出 PNG，前端无需引入 qrcode npm 包
 */
export interface TotpSetupResponse {
  secret: string
  qr_code: string
}

/**
 * 启动 TOTP 2FA 设置
 * 调 GET /api/v1/erp/auth/totp/setup
 * 返回后端生成的密钥 + base64 QR 码图片
 */
export function setupTotp(): Promise<ApiResponse<TotpSetupResponse>> {
  return request.get<ApiResponse<TotpSetupResponse>>('/auth/totp/setup')
}

/**
 * 提交 TOTP 6 位令牌并正式启用 2FA
 * 调 POST /api/v1/erp/auth/totp/enable
 * 后端：验证通过则将 is_totp_enabled 置为 true
 */
export function enableTotp(token: string): Promise<ApiResponse<boolean>> {
  return request.post<ApiResponse<boolean>>('/auth/totp/enable', { token })
}

// 导出 CSRF Token 工具，供 request.ts 中的 axios 拦截器使用
export { loadCsrfToken, clearCsrfToken }
