import { request } from './request'
import type { ApiResponse, LoginRequest, LoginResponse, UserInfo } from '@/types/api'
// Wave B-3：CSRF Token 由后端写入非 httpOnly Cookie，前端从 document.cookie 读取
// 这里重新导出 storage.ts 中的工具，保留外部调用方（request.ts / user.ts）的 API 一致性
export { loadCsrfToken, clearCsrfToken } from '@/utils/storage'

/**
 * 登录响应扩展：包含后端返回的 CSRF Token（保留向后兼容）
 * 注意：新版流程下，csrf_token 也通过 Set-Cookie 头写入，前端只读取不存 localStorage
 */
type LoginResponseWithCsrf = LoginResponse & { csrf_token?: string }

/**
 * 刷新 Token 响应：包含新 access_token 与可选的 CSRF Token
 */
interface RefreshTokenResponse {
  token: string
  csrf_token?: string
}

export function login(data: LoginRequest): Promise<LoginResponse> {
  return request.post<LoginResponseWithCsrf>('/auth/login', data).then(res => {
    // Wave B-3：不再写 localStorage。Cookie 由后端 Set-Cookie 自动写入。
    // 转换为标准 LoginResponse（去除 csrf_token 字段）
    const { csrf_token: _csrf, ...payload } = res
    void _csrf
    return payload
  })
}

export function logout(): Promise<void> {
  // Wave B-3：登出由后端通过 Set-Cookie + max-age=0 清除所有登录态 Cookie
  return request.post<void>('/auth/logout')
}

/**
 * 刷新 Token
 * - 不再从请求体带 refresh_token，浏览器会自动通过 httpOnly Cookie 发送
 * - 后端刷新成功后通过 Set-Cookie 头更新 access_token / csrf_token
 * - 入参保留 refreshToken 字符串参数以兼容历史调用，Wave B-3 后该参数已无效
 */
export function refreshToken(_refreshToken: string): Promise<{ token: string; csrf_token?: string }> {
  return request.post<RefreshTokenResponse>('/auth/refresh', {}).then(res => {
    // Cookie 已由后端 Set-Cookie 写入，前端无需再保存
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
