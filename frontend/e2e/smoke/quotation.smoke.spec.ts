// P3 6-8 修复：将 if (await X.isVisible()) 弱断言改为 await expect(X).toBeVisible() 强断言
// 注：本文件已全部使用强断言，无需修改 if 模式
// 报价单 E2E 冒烟测试
// - 覆盖：列表页 / 新建页 / 详情页 / 审批页
// - 沙箱无 backend，全部走 API mock
// - 主要验证页面能正常加载、按钮可见

import { test, expect } from '@playwright/test'
import { applyAuthMocks, waitForPageReady } from './_helpers'

test.describe('报价单 E2E', () => {
  test.beforeEach(async ({ context }) => {
    await applyAuthMocks(context)
  })

  test('报价单列表页加载', async ({ page }) => {
    await page.goto('/quotations')
    await waitForPageReady(page, '/quotations')

    // 标题
    await expect(page.locator('.title').first()).toContainText('报价单管理')

    // 新建按钮可见
    const newBtn = page.locator('button:has-text("新建报价单")')
    await expect(newBtn).toBeVisible()

    // 筛选区
    await expect(page.locator('.filter-form')).toBeVisible()
  })

  test('新建报价单页加载', async ({ page }) => {
    await page.goto('/quotations/new')
    await waitForPageReady(page, '/quotations/new')

    // 标题
    await expect(page.locator('.title').first()).toContainText('新建报价单')

    // 关键表单字段
    await expect(page.locator('text=客户').first()).toBeVisible()
    await expect(page.locator('text=报价日期').first()).toBeVisible()
    await expect(page.locator('text=有效期至').first()).toBeVisible()
    await expect(page.locator('text=价格条款').first()).toBeVisible()

    // 按钮
    await expect(page.locator('button:has-text("保存草稿")')).toBeVisible()
    await expect(page.locator('button:has-text("提交审批")')).toBeVisible()

    // 明细/条款区块
    await expect(page.locator('button:has-text("添加产品")')).toBeVisible()
    await expect(page.locator('text=报价明细')).toBeVisible()
    await expect(page.locator('text=贸易条款')).toBeVisible()
  })

  test('报价单详情页加载（mock 404）', async ({ page }) => {
    await page.goto('/quotations/1')
    await waitForPageReady(page, '/quotations/1')
    // 页面能渲染（无 JS 崩溃）即视为通过
    await expect(page.locator('.quotation-detail')).toBeAttached()
  })

  test('报价单编辑页加载', async ({ page }) => {
    await page.goto('/quotations/1/edit')
    await waitForPageReady(page, '/quotations/1/edit')
    // 复用 create.vue，会渲染同样的表单
    await expect(page.locator('.quotation-create')).toBeAttached()
  })

  test('报价单审批页加载', async ({ page }) => {
    await page.goto('/quotations/1/approval')
    await waitForPageReady(page, '/quotations/1/approval')
    await expect(page.locator('.approval-page')).toBeAttached()
  })
})
