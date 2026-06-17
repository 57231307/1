// P9-4 采购 E2E 套件 — 04 采购质检
// 创建时间: 2026-06-17
// 覆盖范围：采购质检全流程（3 用例）

import { test, expect } from '@playwright/test'

/**
 * 测试套件：采购质检
 *
 * 业务流程：
 * 1. 入库单可触发质检
 * 2. 质检合格 → 库存增加
 * 3. 质检不合格 → 进入退货流程
 */
test.describe('04 采购质检', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
  })

  test('04-01 入库单可发起质检', async ({ page }) => {
    await page.goto('/purchase/receipt/list')
    const receipt = page.locator('tr, .el-table__row').filter({ hasText: '待质检|已入库/' }).first()
    await receipt.getByRole('button', { name: /详情/ }).click()
    // 发起质检
    await page.getByRole('button', { name: /发起质检|质检/ }).click()
    // 验证质检单生成
    await expect(page.getByText(/质检单号.*QI-\d{8}-\d{4}/)).toBeVisible({ timeout: 5000 })
  })

  test('04-02 质检合格后库存自动增加', async ({ page }) => {
    await page.goto('/purchase/inspection/list')
    const inspection = page.locator('tr, .el-table__row').filter({ hasText: '待质检' }).first()
    await inspection.getByRole('button', { name: /详情/ }).click()
    // 录入质检结果 - 全部合格
    await page.getByLabel(/合格数量/).fill('100')
    await page.getByLabel(/不合格数量/).fill('0')
    // 提交
    await page.getByRole('button', { name: /保存|提交/ }).click()
    await expect(page.getByText(/质检完成|合格/)).toBeVisible({ timeout: 5000 })
  })

  test('04-03 质检不合格可触发退货流程', async ({ page }) => {
    await page.goto('/purchase/inspection/list')
    const inspection = page.locator('tr, .el-table__row').filter({ hasText: '待质检' }).first()
    await inspection.getByRole('button', { name: /详情/ }).click()
    // 录入不合格
    await page.getByLabel(/合格数量/).fill('80')
    await page.getByLabel(/不合格数量/).fill('20')
    // 不合格原因
    await page.getByLabel(/不合格原因/).fill('E2E 测试：色差超标')
    await page.getByRole('button', { name: /保存|提交/ }).click()
    // 应提示退货
    await expect(page.getByText(/退货|不合格处理/)).toBeVisible()
  })
})
