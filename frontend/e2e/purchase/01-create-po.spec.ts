// P9-4 采购 E2E 套件 — 01 创建采购订单
// 创建时间: 2026-06-17
// 覆盖范围：采购订单创建全流程（5 用例）

import { test, expect } from '@playwright/test'

/**
 * 测试套件：采购订单创建
 *
 * 业务流程：
 * 1. 进入采购 → 采购订单 → 新建
 * 2. 选择供应商（必填）
 * 3. 选择产品行（必填 ≥1 行）
 * 4. 设置采购数量、单价、税率
 * 5. 提交并验证采购订单号生成
 */
test.describe('01 创建采购订单', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
  })

  test('01-01 进入采购订单列表页可见菜单', async ({ page }) => {
    await page.goto('/purchase/order/list')
    await expect(page.getByText('采购订单列表')).toBeVisible()
    await expect(page.getByRole('button', { name: /新建采购订单/ })).toBeVisible()
  })

  test('01-02 创建空采购订单应校验失败', async ({ page }) => {
    await page.goto('/purchase/order/create')
    // 不填任何字段直接提交
    await page.getByRole('button', { name: /保存/ }).click()
    // 应显示供应商/产品必填错误
    await expect(page.getByText(/供应商.*必选|请选择供应商/)).toBeVisible()
  })

  test('01-03 创建有效采购订单成功并生成采购单号', async ({ page }) => {
    await page.goto('/purchase/order/create')
    // 选择供应商
    await page.getByLabel(/供应商/).first().click()
    await page.getByRole('option').first().click()
    // 选择产品
    await page.getByLabel(/产品/).first().click()
    await page.getByRole('option').first().click()
    // 数量
    await page.getByLabel(/数量/).fill('200')
    // 单价
    await page.getByLabel(/单价/).fill('30.00')
    // 提交
    await page.getByRole('button', { name: /保存/ }).click()
    // 验证成功提示 + 采购单号格式 PO-YYYYMMDD-XXXX
    await expect(page.getByText(/采购订单号.*PO-\d{8}-\d{4}/)).toBeVisible({ timeout: 5000 })
  })

  test('01-04 采购订单可指定交货日期与仓库', async ({ page }) => {
    await page.goto('/purchase/order/create')
    await page.getByLabel(/供应商/).first().click()
    await page.getByRole('option').first().click()
    await page.getByLabel(/产品/).first().click()
    await page.getByRole('option').first().click()
    // 交货日期
    await page.getByLabel(/交货日期/).fill('2026-07-15')
    // 仓库
    await page.getByLabel(/仓库/).click()
    await page.getByRole('option').first().click()
    await page.getByLabel(/数量/).fill('100')
    await page.getByLabel(/单价/).fill('25.50')
    await page.getByRole('button', { name: /保存/ }).click()
    await expect(page.getByText(/保存成功|创建成功/)).toBeVisible()
  })

  test('01-05 采购订单支持多产品行批量下单', async ({ page }) => {
    await page.goto('/purchase/order/create')
    await page.getByLabel(/供应商/).first().click()
    await page.getByRole('option').first().click()
    // 添加 3 行产品
    for (let i = 0; i < 3; i++) {
      await page.getByRole('button', { name: /添加行项|新增行/ }).click()
      // 每行选不同产品
      const productSelect = page.locator('[data-testid="po-item-row"]').nth(i).getByLabel(/产品/)
      await productSelect.click()
      await page.getByRole('option').nth(i).click()
    }
    // 验证有 3 个采购行
    const rows = page.locator('[data-testid="po-item-row"]')
    await expect(rows).toHaveCount(3)
  })
})
