// P0-5 面料多色号定价扩展 E2E 测试
// 创建时间: 2026-06-18
//
// v8 复审 P0-2 修复（2026-06-30）：
// 对齐批次 28 P0-1 fail-secure 模式，凭据从环境变量注入，禁止硬编码 admin/admin123。

import { test, expect } from '@playwright/test'

const BASE_URL = process.env.BASE_URL || 'http://localhost:8080'

/**
 * 批次 28 P0-1 fail-secure 模式：
 * 凭据必须从环境变量注入，缺失时直接 fail，禁止硬编码 admin/admin123。
 */
const TEST_USERNAME = process.env.TEST_USERNAME
const TEST_PASSWORD = process.env.TEST_PASSWORD

async function login(page: import('@playwright/test').Page) {
  if (!TEST_USERNAME || !TEST_PASSWORD) {
    throw new Error(
      'E2E 测试需要环境变量 TEST_USERNAME / TEST_PASSWORD（fail-secure 模式，对齐批次 28 P0-1）',
    )
  }
  await page.goto(`${BASE_URL}/login`)
  await page.fill('input[name="username"]', TEST_USERNAME)
  await page.fill('input[name="password"]', TEST_PASSWORD)
  await page.click('button[type="submit"]')
  await page.waitForURL(/dashboard/)
}

test.describe('面料多色号定价扩展', () => {
  test('1. 登录并访问色号价格列表', async ({ page }) => {
    await login(page)
    await page.goto(`${BASE_URL}/color-prices/list`)
    await expect(page.locator('h3, .el-card__header')).toContainText('色号价格')
  })

  test('2. 详情页查看历史图表', async ({ page }) => {
    await login(page)
    await page.goto(`${BASE_URL}/color-prices/list`)
    // 点击第一个详情链接
    const detailLink = page.locator('a:has-text("详情"), button:has-text("详情")').first()
    if (await detailLink.isVisible({ timeout: 3000 }).catch(() => false)) {
      await detailLink.click()
      await page.waitForLoadState('networkidle')
    }
  })

  test('3. 批量调价页面加载', async ({ page }) => {
    await login(page)
    await page.goto(`${BASE_URL}/color-prices/batch`)
    // 等待页面加载
    await page.waitForLoadState('networkidle')
  })
})
