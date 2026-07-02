// P9-4 采购 E2E 套件 — 03 采购入库
// 创建时间: 2026-06-17
// 覆盖范围：采购入库全流程（4 用例）

import { test, expect } from '@playwright/test'
import { applyAuthMocks } from '../smoke/_helpers'

/**
 * 测试套件：采购入库
 *
 * 业务流程：
 * 1. 创建入库单
 * 2. 部分入库
 * 3. 全部入库
 * 4. 库存自动增加验证
 */
test.describe('03 采购入库', () => {
  test.beforeEach(async ({ page, context }) => {
    // P1 6-7 修复（批次 66）：注入 auth mock + mock 业务 API，避免 CI 无后端 timeout
    await applyAuthMocks(context)
    await page.goto('/')
  })

  test('03-01 已审核采购订单可创建入库单', async ({ page }) => {
    await page.goto('/purchase/order/list')
    const approved = page.locator('tr, .el-table__row').filter({ hasText: '已审核' }).first()
    await approved.getByRole('button', { name: /详情/ }).click()
    // 创建入库单
    await page.getByRole('button', { name: /创建入库|入库/ }).click()
    // 选择仓库
    await page.getByLabel(/仓库/).click()
    await page.getByRole('option').first().click()
    // 库位
    await page.getByLabel(/库位/).click()
    await page.getByRole('option').first().click()
    await page.getByRole('button', { name: /保存/ }).click()
    await expect(page.getByText(/入库单号.*GR-\d{8}-\d{4}/)).toBeVisible({ timeout: 5000 })
  })

  test('03-02 入库数量必须 ≤ 采购订单数量', async ({ page }) => {
    await page.goto('/purchase/order/list')
    const approved = page.locator('tr, .el-table__row').filter({ hasText: '已审核' }).first()
    await approved.getByRole('button', { name: /详情/ }).click()
    await page.getByRole('button', { name: /创建入库/ }).click()
    // 输入超过采购订单数量的入库数
    const recQty = page.getByLabel(/本次入库数量/).first()
    await recQty.fill('99999')
    await page.getByRole('button', { name: /保存/ }).click()
    // 应提示超过采购数量
    await expect(page.getByText(/超过采购数量|超出可入数量/)).toBeVisible()
  })

  test('03-03 部分入库后采购订单状态为"部分入库"', async ({ page }) => {
    await page.goto('/purchase/order/list')
    const approved = page.locator('tr, .el-table__row').filter({ hasText: '已审核' }).first()
    await approved.getByRole('button', { name: /详情/ }).click()
    await page.getByRole('button', { name: /创建入库/ }).click()
    // 输入部分入库数量
    const recQty = page.getByLabel(/本次入库数量/).first()
    await recQty.fill('50')
    await page.getByRole('button', { name: /保存/ }).click()
    // 回到采购订单详情
    await page.goto(page.url().replace('/receipt', ''))
    await expect(page.getByText('部分入库').first()).toBeVisible({ timeout: 5000 })
  })

  test('03-04 全部入库后采购订单状态为"已入库"', async ({ page }) => {
    await page.goto('/purchase/order/list')
    const approved = page.locator('tr, .el-table__row').filter({ hasText: '已审核' }).first()
    await approved.getByRole('button', { name: /详情/ }).click()
    await page.getByRole('button', { name: /创建入库/ }).click()
    // 输入全部入库数量
    const recQty = page.getByLabel(/本次入库数量/).first()
    await recQty.fill('500')
    await page.getByRole('button', { name: /保存/ }).click()
    // 回到采购订单详情
    await expect(page.getByText('已入库').first()).toBeVisible({ timeout: 5000 })
  })
})
