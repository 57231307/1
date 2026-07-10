/**
 * 多上下文隔离 + 多用户角色协作工具集
 *
 * 批次 262：针对项目 E2E 测试增强，提供以下能力：
 * - 多上下文隔离：为不同用户角色创建独立的 BrowserContext（cookie/localStorage 互不干扰）
 * - 多用户角色协作：模拟 admin / manager / operator 等角色在同一业务流程中的协作
 * - 并行会话：多个角色同时在各自上下文中操作，验证协作场景
 *
 * 设计原则：
 * - 每个角色一个独立 BrowserContext，互不干扰（模拟不同浏览器/不同设备）
 * - 角色凭据从环境变量注入（fail-secure 模式，禁止硬编码）
 * - 协作场景使用真实后端登录（与 color-card.spec.ts 一致）
 * - smoke 场景可使用 mock 凭据（与 auth.ts 一致）
 */
import type { Browser, BrowserContext, Page } from '@playwright/test'
import { injectAuthToken, mockInitStatus } from './auth'

/**
 * 角色凭据配置
 */
export interface RoleCredentials {
  /** 角色名称（用于日志与断言） */
  role: string
  /** 登录用户名 */
  username: string
  /** 登录密码 */
  password: string
}

/**
 * 隔离会话：一个 BrowserContext + 一个 Page + 角色信息
 */
export interface IsolatedSession {
  /** 角色名称 */
  role: string
  /** 隔离的浏览器上下文（cookie/localStorage 独立） */
  context: BrowserContext
  /** 会话页面 */
  page: Page
  /** 关闭会话（关闭 context） */
  close: () => Promise<void>
}

/**
 * 从环境变量读取角色凭据（fail-secure 模式）
 *
 * 环境变量命名约定：
 * - E2E_ADMIN_USERNAME / E2E_ADMIN_PASSWORD
 * - E2E_MANAGER_USERNAME / E2E_MANAGER_PASSWORD
 * - E2E_OPERATOR_USERNAME / E2E_OPERATOR_PASSWORD
 *
 * 缺失时回退到 TEST_USERNAME / TEST_PASSWORD（admin 角色）
 */
export function loadRoleCredentials(role: string): RoleCredentials {
  const envPrefix = `E2E_${role.toUpperCase()}`
  const username = process.env[`${envPrefix}_USERNAME`]
  const password = process.env[`${envPrefix}_PASSWORD`]

  // 回退：admin 角色使用 TEST_USERNAME / TEST_PASSWORD
  if (!username || !password) {
    if (role === 'admin') {
      const fallbackUser = process.env.TEST_USERNAME
      const fallbackPass = process.env.TEST_PASSWORD
      if (fallbackUser && fallbackPass) {
        return { role, username: fallbackUser, password: fallbackPass }
      }
    }
    throw new Error(
      `E2E 多角色测试需要环境变量 ${envPrefix}_USERNAME / ${envPrefix}_PASSWORD` +
        `（fail-secure 模式，禁止硬编码凭据）`,
    )
  }

  return { role, username, password }
}

/**
 * 创建隔离的浏览器会话（独立 context）
 *
 * 使用场景：
 * - 多用户角色协作测试（每个角色一个 context，互不干扰）
 * - 多设备/多浏览器会话模拟（不同 context 模拟不同设备）
 *
 * @example
 * const adminSession = await createIsolatedSession(browser, 'admin')
 * const managerSession = await createIsolatedSession(browser, 'manager')
 */
export async function createIsolatedSession(
  browser: Browser,
  role: string,
): Promise<IsolatedSession> {
  const context = await browser.newContext()
  const page = await context.newPage()

  return {
    role,
    context,
    page,
    close: async () => {
      await context.close()
    },
  }
}

/**
 * 创建带 mock 鉴权的隔离会话（smoke 测试用，不依赖真实后端）
 *
 * 使用场景：
 * - smoke 测试中多角色协作（mock 不同角色的权限）
 * - 无后端环境下的多角色 UI 交互测试
 *
 * @param role 角色名称（用于日志）
 * @param permissions 该角色的权限列表（写入 mock /auth/me 响应）
 */
export async function createMockedIsolatedSession(
  browser: Browser,
  role: string,
  permissions: string[] = ['*'],
): Promise<IsolatedSession> {
  const session = await createIsolatedSession(browser, role)

  // 注入 mock token（绕过鉴权）
  await injectAuthToken(session.context)

  // 拦截 /auth/me 返回该角色的 mock 用户信息
  await session.context.route('**/api/v1/erp/auth/me**', (route) => {
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        code: 200,
        message: 'success',
        data: {
          id: Math.floor(Math.random() * 1000) + 1,
          username: `mock_${role}`,
          real_name: `测试${role}`,
          role_id: Math.floor(Math.random() * 10) + 1,
          role_name: role,
          permissions,
        },
        timestamp: new Date().toISOString(),
      }),
    })
  })

  // 拦截 init/status 返回 initialized: true
  await mockInitStatus(session.context)

  return session
}

/**
 * 执行真实登录流程（在指定会话中）
 *
 * 使用场景：
 * - 多角色协作测试中，每个角色在自己的 context 中真实登录
 * - 与 color-card.spec.ts 的 login 函数一致，但作用于指定 page
 *
 * @param session 隔离会话
 * @param credentials 角色凭据
 * @param baseURL 应用基础地址
 */
export async function loginSession(
  session: IsolatedSession,
  credentials: RoleCredentials,
  baseURL: string,
): Promise<void> {
  const { page } = session
  await page.goto(`${baseURL}/login`)
  await page.waitForSelector('input[name="username"]', { state: 'visible' })
  await page.waitForSelector('input[name="password"]', { state: 'visible' })

  await page.fill('input[name="username"]', credentials.username)
  await page.fill('input[name="password"]', credentials.password)
  await page.click('button[type="submit"]')

  // 等待登录成功后跳转
  await page.waitForURL(/dashboard|\/$/, { timeout: 15_000 })
}

/**
 * 并行执行多个角色的会话操作
 *
 * 使用场景：
 * - 多用户角色协作场景（admin 创建 → manager 审批 → operator 查看）
 * - 并行测试多个角色的 UI 交互（互不干扰）
 *
 * @example
 * await runParallelSessions(browser, [
 *   { role: 'admin', action: async (session) => { await adminCreatesOrder(session) } },
 *   { role: 'manager', action: async (session) => { await managerApprovesOrder(session) } },
 * ])
 */
export async function runParallelSessions(
  browser: Browser,
  sessions: Array<{
    role: string
    action: (session: IsolatedSession) => Promise<void>
  }>,
): Promise<void> {
  // 创建所有会话
  const isolated = await Promise.all(
    sessions.map((s) => createMockedIsolatedSession(browser, s.role)),
  )

  try {
    // 并行执行各会话的操作
    await Promise.all(
      isolated.map((session, index) => sessions[index].action(session)),
    )
  } finally {
    // 关闭所有会话
    await Promise.all(isolated.map((session) => session.close()))
  }
}

/**
 * 创建多角色协作上下文（一次性创建多个隔离会话）
 *
 * 使用场景：
 * - 业务流程协作测试（创建 → 审批 → 查看多角色流转）
 * - 需要多个角色同时在线的复杂场景
 *
 * @example
 * const { sessions, close } = await createCollaborationContext(browser, ['admin', 'manager'])
 * try {
 *   await sessions.admin.page.goto('/admin')
 *   await sessions.manager.page.goto('/dashboard')
 * } finally {
 *   await close()
 * }
 */
export async function createCollaborationContext(
  browser: Browser,
  roles: string[],
  useMock = true,
): Promise<{
  sessions: Record<string, IsolatedSession>
  close: () => Promise<void>
}> {
  const sessionList = await Promise.all(
    roles.map((role) =>
      useMock ? createMockedIsolatedSession(browser, role) : createIsolatedSession(browser, role),
    ),
  )

  const sessions: Record<string, IsolatedSession> = {}
  sessionList.forEach((session) => {
    sessions[session.role] = session
  })

  return {
    sessions,
    close: async () => {
      await Promise.all(sessionList.map((session) => session.close()))
    },
  }
}
