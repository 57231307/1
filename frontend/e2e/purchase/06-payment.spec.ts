// P9-4 采购 E2E 套件 — 06 采购付款
// 创建时间: 2026-06-17
// 覆盖范围：采购付款全流程（4 用例）

import { test, expect } from '@playwright/test'
import { applyAuthMocks } from '../smoke/_helpers'

/**
 * 测试套件：采购付款
 *
 * 业务流程：
 * 1. 应付单付款（一次性）
 * 2. 应付单部分付款
 * 3. 付款方式（5 种）
 * 4. 付款单打印
 */
test.describe('06 采购付款', () => {
  test.beforeEach(async ({ page, context }) => {
    // V15 Batch 487 P0-T05：注入 auth mock，业务 API 走真实后端（applyAuthMocks 不再 mock 业务 API）
    await applyAuthMocks(context)
    await page.goto('/')
  })

  test('06-01 应付单可一次性付清', async ({ page }) => {
    await page.goto('/ap/invoice/list')
    const invoice = page.locator('tr, .el-table__row').filter({ hasText: '未付款' }).first()
    await invoice.getByRole('button', { name: /详情/ }).click()
    await page.getByRole('button', { name: /付款/ }).click()
    // 全额付款
    await page.getByRole('button', { name: /全额付款/ }).click()
    await page.getByRole('button', { name: /确认/ }).click()
    await expect(page.getByText(/已付款|付款成功/)).toBeVisible()
  })

  test('06-02 应付单可多次部分付款', async ({ page }) => {
    await page.goto('/ap/invoice/list')
    const invoice = page.locator('tr, .el-table__row').filter({ hasText: '未付款' }).first()
    await invoice.getByRole('button', { name: /详情/ }).click()
    // 第 1 次付款
    await page.getByRole('button', { name: /付款/ }).click()
    await page.getByLabel(/付款金额/).fill('5000')
    await page.getByRole('button', { name: /确认/ }).click()
    // 第 2 次付款
    await page.getByRole('button', { name: /付款/ }).click()
    await page.getByLabel(/付款金额/).fill('3000')
    await page.getByRole('button', { name: /确认/ }).click()
    await expect(page.getByText(/已付.*8000/)).toBeVisible()
  })

  test('06-03 支持 5 种付款方式', async ({ page }) => {
    await page.goto('/ap/invoice/list')
    const invoice = page.locator('tr, .el-table__row').filter({ hasText: '未付款' }).first()
    await invoice.getByRole('button', { name: /详情/ }).click()
    await page.getByRole('button', { name: /付款/ }).click()
    await page.getByLabel(/付款方式/).click()
    const options = page.getByRole('option')
    await expect(options.filter({ hasText: /银行转账/ })).toBeVisible()
    await expect(options.filter({ hasText: /现金/ })).toBeVisible()
    await expect(options.filter({ hasText: /承兑汇票/ })).toBeVisible()
    await expect(options.filter({ hasText: /支付宝/ })).toBeVisible()
    await expect(options.filter({ hasText: /微信/ })).toBeVisible()
  })

  test('06-04 付款单可打印', async ({ page }) => {
    await page.goto('/ap/payment/list')
    const payment = page.locator('tr, .el-table__row').first()
    await payment.getByRole('button', { name: /详情/ }).click()
    await page.getByRole('button', { name: /打印/ }).click()
    await expect(page.getByText(/付款单|付款凭证/)).toBeVisible()
  })
})
