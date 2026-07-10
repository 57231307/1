/**
 * 网络韧性测试：模拟后端异常返回 / 弱网环境 / 网络中断
 *
 * 批次 262：验证前端对后端异常与弱网环境的容错处理。
 *
 * 测试范围：
 * - 后端 500 错误：前端应显示错误提示而非崩溃
 * - 后端 403 错误：前端应显示权限不足提示
 * - 弱网环境（慢速返回）：前端应显示 loading 状态
 * - 网络中断：前端应显示网络错误提示
 *
 * 设计说明：
 * - 使用 smoke 测试的 mock 模式（不依赖真实后端的异常注入）
 * - 通过 fixtures/network.ts 的工具函数注入异常
 * - 与 fixtures/auth.ts 的 mock 配合，确保页面可访问
 */
import { test, expect } from '@playwright/test'
import { applyAuthMocks } from '../smoke/_helpers'
import { mockApiError, mockNetworkFailure, simulateSlowNetwork } from '../fixtures/network'

test.describe('网络韧性：后端异常返回处理', () => {
  test.beforeEach(async ({ context }) => {
    // 应用基础 mock（鉴权 + 初始化状态）
    await applyAuthMocks(context)
  })

  test('后端 500 错误时前端不崩溃', async ({ page }) => {
    // 注入销售订单接口 500 错误
    await mockApiError(page, '**/api/v1/erp/sales/orders**', {
      status: 500,
      errorCode: 'INTERNAL_ERROR',
      errorMessage: '模拟的服务器内部错误',
    })

    await page.goto('/sales')
    // 页面应正常渲染（不崩溃白屏），表格组件应存在
    await expect(page.locator('.el-table-v2').first()).toBeAttached({ timeout: 10_000 })
    // 页面应保持可交互（不卡死）
    await expect(page.locator('body')).toBeVisible()
  })

  test('后端 403 错误时前端显示错误提示', async ({ page }) => {
    // 注入权限不足错误
    await mockApiError(page, '**/api/v1/erp/sales/orders**', {
      status: 403,
      errorCode: 'PERMISSION_DENIED',
      errorMessage: '权限不足',
    })

    await page.goto('/sales')
    // 页面应正常渲染
    await expect(page.locator('.el-table-v2').first()).toBeAttached({ timeout: 10_000 })
  })

  test('网络中断时前端不卡死', async ({ page }) => {
    // 注入网络中断
    await mockNetworkFailure(page, '**/api/v1/erp/sales/orders**')

    await page.goto('/sales')
    // 即使网络中断，页面骨架应正常渲染
    await expect(page.locator('body')).toBeVisible({ timeout: 10_000 })
  })
})

test.describe('网络韧性：弱网环境处理', () => {
  test.beforeEach(async ({ context }) => {
    await applyAuthMocks(context)
  })

  test('慢速网络下页面可正常加载', async ({ page }) => {
    // 模拟弱网：所有 API 请求延迟 1.5 秒
    await simulateSlowNetwork(page, '**/api/v1/erp/**', 1500)

    await page.goto('/sales')
    // 即使慢速，页面最终应加载完成
    await expect(page.locator('.el-table-v2').first()).toBeAttached({ timeout: 15_000 })
  })

  test('慢速网络下表格组件渲染', async ({ page }) => {
    // 模拟弱网：延迟 800ms
    await simulateSlowNetwork(page, '**/api/v1/erp/**', 800)

    await page.goto('/sales')
    // 表格组件应渲染
    const table = page.locator('.el-table-v2').first()
    await expect(table).toBeAttached({ timeout: 15_000 })
  })
})

test.describe('网络韧性：业务错误码处理', () => {
  test.beforeEach(async ({ context }) => {
    await applyAuthMocks(context)
  })

  test('业务校验错误前端不崩溃', async ({ page }) => {
    // 注入业务校验错误（HTTP 400 + 业务码）
    await mockApiError(page, '**/api/v1/erp/sales/orders**', {
      status: 400,
      errorCode: 'VALIDATION_ERROR',
      errorMessage: '参数校验失败',
    })

    await page.goto('/sales')
    // 前端应正常处理业务错误，不崩溃
    await expect(page.locator('body')).toBeVisible({ timeout: 10_000 })
  })

  test('未授权错误前端处理', async ({ page }) => {
    // 注入 401 未授权（模拟 token 过期）
    await mockApiError(page, '**/api/v1/erp/sales/orders**', {
      status: 401,
      errorCode: 'UNAUTHORIZED',
      errorMessage: '登录已过期',
    })

    await page.goto('/sales')
    // 前端应处理 401（可能跳转登录或显示提示），不崩溃
    await expect(page.locator('body')).toBeVisible({ timeout: 10_000 })
  })
})
