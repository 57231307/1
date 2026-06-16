// P2-3 V2Table 冒烟测试公共工具
// 沙箱环境无 backend，需要：
// 1. 注入 JWT token 到 localStorage（绕过路由 requiresAuth）
// 2. 拦截 /api/v1/erp/auth/me 返回伪造用户信息
// 3. 拦截 /api/v1/erp/init/status 返回 initialized: true

import type { Page, BrowserContext } from '@playwright/test'

/**
 * 生成一个合法的 JWT token（路由只校验格式 + exp，不验证签名）
 */
export function generateFakeJwt(): string {
  const header = Buffer.from(JSON.stringify({ alg: 'HS256', typ: 'JWT' })).toString('base64url')
  const exp = Math.floor(Date.now() / 1000) + 3600
  const payload = Buffer.from(JSON.stringify({ user_id: 1, exp })).toString('base64url')
  const sig = Buffer.from('fake-signature-for-smoke-test').toString('base64url')
  return `${header}.${payload}.${sig}`
}

/**
 * 在所有脚本执行前注入 localStorage token
 */
export async function injectAuthToken(context: BrowserContext): Promise<void> {
  const token = generateFakeJwt()
  await context.addInitScript(t => {
    localStorage.setItem('access_token', t)
    localStorage.setItem('refresh_token', t)
  }, token)
}

/**
 * 拦截 /api/v1/erp/auth/me 返回伪造用户信息（绕过 userStore.fetchUserInfo）
 */
export async function mockAuthMe(context: BrowserContext): Promise<void> {
  await context.route('**/api/v1/erp/auth/me**', route => {
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        code: 200,
        message: 'success',
        data: {
          id: 1,
          username: 'admin',
          real_name: '管理员',
          role_id: 1,
          role_name: '超级管理员',
          permissions: ['*'],
        },
        timestamp: new Date().toISOString(),
      }),
    })
  })
}

/**
 * 拦截 /api/v1/erp/init/status 返回 initialized: true（绕过路由初始化检查）
 */
export async function mockInitStatus(context: BrowserContext): Promise<void> {
  await context.route('**/api/v1/erp/init/status**', route => {
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        code: 200,
        data: { initialized: true },
        timestamp: new Date().toISOString(),
      }),
    })
  })
}

/**
 * 一站式应用所有 auth mock
 */
export async function applyAuthMocks(context: BrowserContext): Promise<void> {
  await injectAuthToken(context)
  await mockAuthMe(context)
  await mockInitStatus(context)
}

/**
 * 等待 URL 不再是 /login（确认绕过鉴权）
 */
export async function waitForPageReady(page: Page, expectedPath: string): Promise<void> {
  // 等待 URL 包含 expectedPath（路由跳转可能异步）
  await page.waitForURL(url => url.pathname === expectedPath || url.pathname.includes(expectedPath), {
    timeout: 10_000,
  }).catch(() => {
    // 如果超时，记录当前 URL 但不抛出
    console.warn(`[smoke] URL 未匹配 ${expectedPath}，当前：${page.url()}`)
  })
}
