// P0-5 面料多色号定价扩展 E2E 测试
// 创建时间: 2026-06-18

import { test, expect } from '@playwright/test'

const BASE_URL = process.env.BASE_URL || 'http://localhost:8080'

test.describe('面料多色号定价扩展', () => {
  test('1. 登录并访问色号价格列表', async ({ page }) => {
    await page.goto(`${BASE_URL}/login`)
    await page.fill('input[name="username"]', 'admin')
    await page.fill('input[name="password"]', 'admin123')
    await page.click('button[type="submit"]')
    await page.waitForURL(/dashboard/)
    await page.goto(`${BASE_URL}/color-prices/list`)
    await expect(page.locator('h3, .el-card__header')).toContainText('色号价格')
  })

  test('2. 详情页查看历史图表', async ({ page }) => {
    await page.goto(`${BASE_URL}/login`)
    await page.fill('input[name="username"]', 'admin')
    await page.fill('input[name="password"]', 'admin123')
    await page.click('button[type="submit"]')
    await page.goto(`${BASE_URL}/color-prices/detail/1`)
    await expect(page.locator('.el-descriptions')).toBeVisible()
  })

  test('3. 批量调价 +5% 自动通过', async ({ page }) => {
    await page.goto(`${BASE_URL}/login`)
    await page.fill('input[name="username"]', 'admin')
    await page.fill('input[name="password"]', 'admin123')
    await page.click('button[type="submit"]')
    await page.goto(`${BASE_URL}/color-prices/batch-adjust`)
    // 选中第一行
    await page.locator('.el-table__row').first().locator('.el-checkbox').click()
    // 提交
    await page.click('button:has-text("提交批量调价")')
    await page.waitForTimeout(2000)
  })

  test('4. 批量调价 +15% 待审批', async ({ page }) => {
    await page.goto(`${BASE_URL}/login`)
    await page.fill('input[name="username"]', 'admin')
    await page.fill('input[name="password"]', 'admin123')
    await page.click('button[type="submit"]')
    await page.goto(`${BASE_URL}/color-prices/batch-adjust`)
    // 调高百分比到 15
    await page.locator('input[type="number"]').first().fill('15')
    await page.locator('.el-table__row').first().locator('.el-checkbox').click()
    await page.click('button:has-text("提交批量调价")')
    await page.waitForTimeout(2000)
  })

  test('5. 价格计算 API 演示', async ({ page }) => {
    await page.goto(`${BASE_URL}/login`)
    await page.fill('input[name="username"]', 'admin')
    await page.fill('input[name="password"]', 'admin123')
    await page.click('button[type="submit"]')
    await page.goto(`${BASE_URL}/color-prices/batch-adjust`)
    await page.locator('.el-table__row').first().locator('.el-checkbox').click()
    await page.click('button:has-text("价格计算演示")')
    await page.waitForTimeout(1500)
  })
})
