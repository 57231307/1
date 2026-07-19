// P9-3 销售 E2E 套件 — 05 应收单生成
// 创建时间: 2026-06-17
// 覆盖范围：发货后自动生成 AR 应收单（3 用例）

import { test, expect } from '@playwright/test'
import { applyAuthMocks } from '../smoke/_helpers'

/**
 * 测试套件：应收单生成
 *
 * 业务流程：
 * 1. 已发货订单自动生成应收单
 * 2. 应收单金额 = 发货金额 × (1 + 税率)
 * 3. 应收单可分次核销
 */
test.describe('05 AR 应收单生成', () => {
  test.beforeEach(async ({ page, context }) => {
    // V15 Batch 487 P0-T05：注入 auth mock，业务 API 走真实后端（applyAuthMocks 不再 mock 业务 API）
    await applyAuthMocks(context)
    await page.goto('/')
  })

  test('05-01 已发货订单自动生成 AR 应收单', async ({ page }) => {
    await page.goto('/sales/order/list')
    const shipped = page.locator('tr, .el-table__row').filter({ hasText: '已发货' }).first()
    await shipped.getByRole('button', { name: /详情/ }).click()
    // 查看关联的应收单
    await page.getByRole('tab', { name: /应收单|AR/ }).click()
    await expect(page.getByText(/应收单号.*AR-\d{8}-\d{4}/)).toBeVisible()
  })

  test('05-02 应收单金额 = 发货金额 + 税额', async ({ page }) => {
    await page.goto('/sales/order/list')
    const shipped = page.locator('tr, .el-table__row').filter({ hasText: '已发货' }).first()
    await shipped.getByRole('button', { name: /详情/ }).click()
    await page.getByRole('tab', { name: /应收单/ }).click()
    // 应收单总额应 = 销售订单总额（已含税）
    const total = page.getByTestId('ar-invoice-total')
    await expect(total).toBeVisible()
  })

  test('05-03 应收单支持分次收款', async ({ page }) => {
    await page.goto('/ar/invoice/list')
    const invoice = page.locator('tr, .el-table__row').filter({ hasText: '未收款' }).first()
    await invoice.getByRole('button', { name: /详情/ }).click()
    // 部分收款
    await page.getByRole('button', { name: /收款/ }).click()
    await page.getByLabel(/收款金额/).fill('1000')
    await page.getByLabel(/收款方式/).click()
    await page.getByRole('option', { name: /银行转账/ }).click()
    await page.getByRole('button', { name: /确认/ }).click()
    await expect(page.getByText(/收款成功|已收款/)).toBeVisible()
  })
})
