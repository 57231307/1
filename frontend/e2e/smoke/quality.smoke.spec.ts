import { test, expect } from '@playwright/test'
import { applyAuthMocks, waitForPageReady } from './_helpers'

test.describe('quality 页面冒烟测试', () => {
  test.beforeEach(async ({ context }) => {
    await applyAuthMocks(context)
  })

  test('页面加载 + 表格渲染', async ({ page }) => {
    await page.goto('/quality')
    await waitForPageReady(page, '/quality')
    await expect(page.locator('.el-table-v2').first()).toBeAttached({ timeout: 10_000 })
  })

  test('搜索功能', async ({ page }) => {
    await page.goto('/quality')
    await waitForPageReady(page, '/quality')
    const searchInput = page.locator('input[placeholder*="搜索"], input[placeholder*="关键词"]').first()
    if (await searchInput.isVisible()) {
      await searchInput.fill('test')
      await page.waitForTimeout(500)
      await expect(page.locator('.el-table-v2').first()).toBeAttached()
    }
  })

  test('分页切换', async ({ page }) => {
    await page.goto('/quality')
    await waitForPageReady(page, '/quality')
    const nextPageBtn = page.locator('.el-pagination .btn-next').first()
    if (await nextPageBtn.isVisible()) {
      await nextPageBtn.click()
      await page.waitForTimeout(500)
      await expect(page.locator('.el-table-v2').first()).toBeAttached()
    }
  })

  test('导出按钮', async ({ page }) => {
    await page.goto('/quality')
    await waitForPageReady(page, '/quality')
    const exportBtn = page.locator('button:has-text("导出")').first()
    if (await exportBtn.isVisible()) {
      await expect(exportBtn).toBeEnabled()
    }
  })

  test('新建按钮', async ({ page }) => {
    await page.goto('/quality')
    await waitForPageReady(page, '/quality')
    const newBtn = page.locator('button:has-text("新建")').first()
    if (await newBtn.isVisible()) {
      await newBtn.click()
      await expect(page.locator('.el-dialog, .el-drawer').first()).toBeVisible({ timeout: 3_000 })
      const cancelBtn = page.locator('button:has-text("取消")').first()
      if (await cancelBtn.isVisible()) {
        await cancelBtn.click()
      }
    }
  })
})
