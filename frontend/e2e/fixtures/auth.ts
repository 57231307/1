/**
 * E2E 测试 auth mock 数据夹具
 * 规则 6：测试 mock 数据禁止硬编码在测试用例中，统一抽取到 fixtures
 *
 * 说明：smoke 测试使用 mock 绕过鉴权（不依赖后端）；
 * 业务流程测试（color-card/sales/purchase 等）使用真实后端登录，不使用本文件。
 *
 * V15 Batch 487 P0-T05 修复（规则 5）：
 * - applyAuthMocks 不再自动调用 mockBusinessApi，让 sales/* / purchase/* 等业务流程
 *   E2E 走真实后端（修复批次 190 E2E 通过率 0% 的根因之一）
 * - mockBusinessApi 函数保留，仅供 enhanced/multi-role-collaboration.spec.ts 等
 *   多上下文隔离测试显式调用（此类测试不依赖业务数据，只需页面可加载）
 * - 业务流程测试严禁调用 mockBusinessApi（违反规则 5）
 */
import type { BrowserContext, Page } from '@playwright/test'

/** 测试用户信息（mock，仅 smoke 测试用） */
const MOCK_USER = {
  id: 1,
  username: 'admin',
  real_name: '管理员',
  role_id: 1,
  role_name: '超级管理员',
  permissions: ['*'],
} as const

/** mock JWT header */
const MOCK_JWT_HEADER = { alg: 'HS256', typ: 'JWT' } as const

/**
 * 生成一个合法格式的 JWT token（路由只校验格式 + exp，不验证签名）
 */
export function generateFakeJwt(): string {
  const header = Buffer.from(JSON.stringify(MOCK_JWT_HEADER)).toString('base64url')
  const exp = Math.floor(Date.now() / 1000) + 3600
  const payload = Buffer.from(JSON.stringify({ user_id: MOCK_USER.id, exp })).toString('base64url')
  const sig = Buffer.from('fake-signature-for-smoke-test-only').toString('base64url')
  return `${header}.${payload}.${sig}`
}

/**
 * 在所有脚本执行前注入 localStorage token
 */
export async function injectAuthToken(context: BrowserContext): Promise<void> {
  const token = generateFakeJwt()
  await context.addInitScript(
    (t) => {
      localStorage.setItem('access_token', t)
      localStorage.setItem('refresh_token', t)
    },
    token,
  )
}

/**
 * 拦截 /api/v1/erp/auth/me 返回 mock 用户信息（绕过 userStore.fetchUserInfo）
 */
export async function mockAuthMe(context: BrowserContext): Promise<void> {
  await context.route('**/api/v1/erp/auth/me**', (route) => {
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        code: 200,
        message: 'success',
        data: MOCK_USER,
        timestamp: new Date().toISOString(),
      }),
    })
  })
}

/**
 * 拦截 /api/v1/erp/init/status 返回 initialized: true（绕过路由初始化检查）
 */
export async function mockInitStatus(context: BrowserContext): Promise<void> {
  await context.route('**/api/v1/erp/init/status**', (route) => {
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

/** 空分页响应（避免前端 catch 长时间等待） */
const EMPTY_PAGINATION = {
  code: 200,
  message: 'success',
  data: { items: [], total: 0, page: 1, page_size: 20 },
} as const

/**
 * 拦截 /api/v1/erp/* 业务 API 返回空数据
 *
 * 设计原因：CI 环境无后端时，spec 直接 page.goto 会触发业务 API 请求，
 * 无响应会导致断言超时。此函数注册在 mockAuthMe / mockInitStatus 之后，
 * 对未匹配的业务 API 返回空分页数据。
 *
 * V15 Batch 487 P0-T05 修复（规则 5）：
 * - applyAuthMocks 不再自动调用本函数，业务流程测试（sales/purchase 等）
 *   必须走真实后端，禁止调用本函数
 * - 本函数仅供 enhanced/multi-role-collaboration.spec.ts 等多上下文隔离
 *   测试显式调用（此类测试不依赖业务数据，只需页面可加载）
 * - smoke 测试如需使用，应显式调用（不再通过 applyAuthMocks 自动应用）
 */
export async function mockBusinessApi(context: BrowserContext): Promise<void> {
  await context.route('**/api/v1/erp/**', (route) => {
    const url = route.request().url()
    // 已被 mockAuthMe / mockInitStatus 处理的请求放行到下一个 handler
    if (url.includes('/auth/me') || url.includes('/init/status')) {
      return route.fallback()
    }
    // 其他业务 API 返回空分页数据，避免前端 catch 长时间等待
    return route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        ...EMPTY_PAGINATION,
        timestamp: new Date().toISOString(),
      }),
    })
  })
}

/**
 * 一站式应用 auth mock（仅 smoke 测试使用）
 *
 * V15 Batch 487 P0-T05 修复（规则 5）：
 * 不再自动调用 mockBusinessApi，让 sales/* / purchase/* 等业务流程 E2E
 * 走真实后端。如需 mock 业务 API（如 enhanced 多上下文隔离测试），
 * 应显式调用 mockBusinessApi(context)。
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
  await page
    .waitForURL((url) => url.pathname === expectedPath || url.pathname.includes(expectedPath), {
      timeout: 10_000,
    })
    .catch(() => {
      console.warn(`[smoke] URL 未匹配 ${expectedPath}，当前：${page.url()}`)
    })
}
