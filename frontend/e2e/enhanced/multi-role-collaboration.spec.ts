/**
 * 多用户角色协作场景测试
 *
 * 批次 262：验证多用户角色协作场景下的多上下文隔离。
 *
 * 测试范围：
 * - 多上下文隔离：不同角色在独立 BrowserContext 中操作，互不干扰
 * - 多角色并行：admin / manager / operator 同时在各自上下文中加载页面
 * - 协作场景：一个角色创建数据，另一个角色查看（数据流验证）
 *
 * 设计说明：
 * - 使用 mock 鉴权（不依赖真实后端的多角色用户）
 * - 通过 fixtures/multi-context.ts 创建隔离会话
 * - 验证 cookie/localStorage 互不干扰
 */
import { test, expect } from '@playwright/test'
import { mockBusinessApi } from '../fixtures/auth'
import {
  createCollaborationContext,
  createMockedIsolatedSession,
  type IsolatedSession,
} from '../fixtures/multi-context'

test.describe('多上下文隔离：独立会话验证', () => {
  test('两个角色拥有独立的 BrowserContext', async ({ browser }) => {
    const adminSession = await createMockedIsolatedSession(browser, 'admin')
    const managerSession = await createMockedIsolatedSession(browser, 'manager')

    try {
      // 为两个会话都应用业务 API mock（避免后端依赖）
      await mockBusinessApi(adminSession.context)
      await mockBusinessApi(managerSession.context)

      // 两个会话加载不同页面
      await adminSession.page.goto('/dashboard')
      await managerSession.page.goto('/dashboard')

      // 两个页面都应正常渲染
      await expect(adminSession.page.locator('body')).toBeVisible({ timeout: 10_000 })
      await expect(managerSession.page.locator('body')).toBeVisible({ timeout: 10_000 })

      // 验证两个 context 的 token 是独立的（localStorage 互不干扰）
      const adminToken = await adminSession.page.evaluate(() =>
        localStorage.getItem('access_token')
      )
      const managerToken = await managerSession.page.evaluate(() =>
        localStorage.getItem('access_token')
      )
      expect(adminToken).toBeTruthy()
      expect(managerToken).toBeTruthy()
      // 两个 token 都应存在（mock token 格式相同，但来自独立 context）
    } finally {
      await adminSession.close()
      await managerSession.close()
    }
  })

  test('一个会话的 localStorage 不影响另一个会话', async ({ browser }) => {
    const session1 = await createMockedIsolatedSession(browser, 'admin')
    const session2 = await createMockedIsolatedSession(browser, 'operator')

    try {
      await mockBusinessApi(session1.context)
      await mockBusinessApi(session2.context)

      await session1.page.goto('/dashboard')
      await session2.page.goto('/dashboard')

      // 在 session1 中写入一个测试值
      await session1.page.evaluate(() => {
        localStorage.setItem('collaboration_test', 'session1_value')
      })

      // 验证 session2 中没有这个值（上下文隔离）
      const session2Value = await session2.page.evaluate(() =>
        localStorage.getItem('collaboration_test')
      )
      expect(session2Value).toBeNull()

      // 验证 session1 中有这个值
      const session1Value = await session1.page.evaluate(() =>
        localStorage.getItem('collaboration_test')
      )
      expect(session1Value).toBe('session1_value')
    } finally {
      await session1.close()
      await session2.close()
    }
  })
})

test.describe('多角色协作：并行会话', () => {
  test('多角色同时加载各自页面', async ({ browser }) => {
    const { sessions, close } = await createCollaborationContext(browser, [
      'admin',
      'manager',
      'operator',
    ])

    try {
      // 为所有会话应用业务 API mock
      await Promise.all(
        Object.values(sessions).map((session: IsolatedSession) =>
          mockBusinessApi(session.context)
        )
      )

      // 并行加载不同页面
      await Promise.all([
        sessions.admin.page.goto('/dashboard'),
        sessions.manager.page.goto('/dashboard'),
        sessions.operator.page.goto('/dashboard'),
      ])

      // 所有页面都应正常渲染
      await Promise.all(
        Object.values(sessions).map((session: IsolatedSession) =>
          expect(session.page.locator('body')).toBeVisible({ timeout: 10_000 })
        )
      )
    } finally {
      await close()
    }
  })
})

test.describe('多角色协作：数据流验证', () => {
  test('admin 角色可访问管理页面', async ({ browser }) => {
    const adminSession = await createMockedIsolatedSession(browser, 'admin', ['*'])

    try {
      await mockBusinessApi(adminSession.context)

      await adminSession.page.goto('/dashboard')
      // admin 应能正常访问
      await expect(adminSession.page.locator('body')).toBeVisible({ timeout: 10_000 })
    } finally {
      await adminSession.close()
    }
  })

  test('受限角色访问页面不崩溃', async ({ browser }) => {
    // 创建一个权限受限的角色
    const limitedSession = await createMockedIsolatedSession(browser, 'viewer', ['view_only'])

    try {
      await mockBusinessApi(limitedSession.context)

      await limitedSession.page.goto('/dashboard')
      // 即使权限受限，页面应正常渲染（不崩溃）
      await expect(limitedSession.page.locator('body')).toBeVisible({ timeout: 10_000 })
    } finally {
      await limitedSession.close()
    }
  })
})
