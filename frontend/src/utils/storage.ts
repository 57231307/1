// 秉羲 ERP token 存储工具
//
// Wave B-3 安全加固（OWASP A07:2021 XSS 防护）：
// - access_token / refresh_token 由后端写入 httpOnly Cookie，
//   前端 JS **无法读取**也无法写入——这正是核心安全收益。
// - 这里仅保留 csrf_token 读取工具，供 axios 拦截器在请求头注入
//   X-CSRF-Token。csrf_token 由后端显式设置为非 httpOnly，以便前端可读。
// - 旧的 setToken / getToken / removeToken / setRefreshToken / getRefreshToken
//   已废弃删除，避免误用引入 XSS 风险。

/**
 * 从 document.cookie 读取指定键的值
 * - 适用于读取**非 httpOnly** 的 Cookie（httpOnly 的 Cookie 浏览器拒绝 JS 读取）
 * - csrf_token 后端设置时使用 http_only=false
 */
function readCookie(name: string): string | null {
  if (typeof document === 'undefined' || !document.cookie) {
    return null
  }
  const target = `${name}=`
  const parts = document.cookie.split(';')
  for (const raw of parts) {
    const cookie = raw.trim()
    if (cookie.startsWith(target)) {
      return decodeURIComponent(cookie.substring(target.length))
    }
  }
  return null
}

/**
 * 获取当前会话的 CSRF Token（用于 X-CSRF-Token 头注入）
 * - 失败返回 null，由调用方决定是否发起请求
 */
export function getCsrfToken(): string | null {
  return readCookie('csrf_token')
}

/**
 * 兼容旧名称：刷新场景下可能仍会调用 loadCsrfToken
 * 实际行为与 getCsrfToken 完全相同（从非 httpOnly Cookie 读取）
 */
export function loadCsrfToken(): string | null {
  return getCsrfToken()
}

/**
 * 兼容旧名称：清除 CSRF Token
 * - 后端在登出 / CSRF 校验失败时会通过 Set-Cookie 头自动清除
 * - 这里保留空操作以兼容旧调用方，避免未定义引用
 */
export function clearCsrfToken(): void {
  // 后端负责清除（Set-Cookie + max-age=0），前端无需也无法操作 httpOnly Cookie
}
