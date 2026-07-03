// P3 6-8 修复：将 if (await X.isVisible()) 弱断言改为 await expect(X).toBeVisible() 强断言
import { test, expect, type Page } from '@playwright/test'
import { applyAuthMocks, waitForPageReady } from './_helpers'

test.describe('inventory 页面冒烟测试', () => {
  test.beforeEach(async ({ context }) => {
    await applyAuthMocks(context)
  })

  test('页面加载 + 表格渲染', async ({ page }) => {
    await page.goto('/inventory')
    await waitForPageReady(page, '/inventory')
    await expect(page.locator('.el-table-v2').first()).toBeAttached({ timeout: 10_000 })
  })

  test('搜索功能', async ({ page }) => {
    await page.goto('/inventory')
    await waitForPageReady(page, '/inventory')
    const searchInput = page.locator('input[placeholder*="搜索"], input[placeholder*="关键词"]').first()
    await expect(searchInput).toBeVisible({ timeout: 5_000 })
    await searchInput.fill('test')
    await page.waitForTimeout(500)
    await expect(page.locator('.el-table-v2').first()).toBeAttached()
  })

  test('分页切换', async ({ page }) => {
    await page.goto('/inventory')
    await waitForPageReady(page, '/inventory')
    const nextPageBtn = page.locator('.el-pagination .btn-next').first()
    await expect(nextPageBtn).toBeVisible({ timeout: 5_000 })
    await nextPageBtn.click()
    await page.waitForTimeout(500)
    await expect(page.locator('.el-table-v2').first()).toBeAttached()
  })

  test('导出按钮', async ({ page }) => {
    await page.goto('/inventory')
    await waitForPageReady(page, '/inventory')
    const exportBtn = page.locator('button:has-text("导出")').first()
    await expect(exportBtn).toBeVisible({ timeout: 5_000 })
    await expect(exportBtn).toBeEnabled()
  })

  test('新建按钮', async ({ page }) => {
    await page.goto('/inventory')
    await waitForPageReady(page, '/inventory')
    const newBtn = page.locator('button:has-text("新建")').first()
    await expect(newBtn).toBeVisible({ timeout: 5_000 })
    await newBtn.click()
    await expect(page.locator('.el-dialog, .el-drawer').first()).toBeVisible({ timeout: 3_000 })
    const cancelBtn = page.locator('button:has-text("取消")').first()
    await expect(cancelBtn).toBeVisible({ timeout: 3_000 })
    await cancelBtn.click()
  })
})
