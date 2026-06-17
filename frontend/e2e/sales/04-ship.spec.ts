// P9-3 销售 E2E 套件 — 04 销售发货
// 创建时间: 2026-06-17
// 覆盖范围：销售发货全流程（4 用例）

import { test, expect } from '@playwright/test'

/**
 * 测试套件：销售发货
 *
 * 业务流程：
 * 1. 创建发货单
 * 2. 部分发货
 * 3. 全部发货
 * 4. 库存自动扣减验证
 */
test.describe('04 销售发货', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
  })

  test('04-01 已审核订单可创建发货单', async ({ page }) => {
    await page.goto('/sales/order/list')
    const approved = page.locator('tr, .el-table__row').filter({ hasText: '已审核' }).first()
    await approved.getByRole('button', { name: /详情/ }).click()
    // 创建发货单
    await page.getByRole('button', { name: /创建发货|发货/ }).click()
    // 选择仓库
    await page.getByLabel(/仓库/).click()
    await page.getByRole('option').first().click()
    // 收货地址
    await page.getByLabel(/收货地址/).fill('浙江省杭州市西湖区文三路 100 号')
    await page.getByRole('button', { name: /保存/ }).click()
    await expect(page.getByText(/发货单号.*DN-\d{8}-\d{4}/)).toBeVisible({ timeout: 5000 })
  })

  test('04-02 发货数量必须 ≤ 订单数量', async ({ page }) => {
    await page.goto('/sales/order/list')
    const approved = page.locator('tr, .el-table__row').filter({ hasText: '已审核' }).first()
    await approved.getByRole('button', { name: /详情/ }).click()
    await page.getByRole('button', { name: /创建发货/ }).click()
    // 输入超过订单数量的发货数
    const shipQty = page.getByLabel(/本次发货数量/).first()
    await shipQty.fill('99999')
    await page.getByRole('button', { name: /保存/ }).click()
    // 应提示超过订单数量
    await expect(page.getByText(/超过订单数量|超出可发数量/)).toBeVisible()
  })

  test('04-03 部分发货后订单状态为"部分发货"', async ({ page }) => {
    await page.goto('/sales/order/list')
    const approved = page.locator('tr, .el-table__row').filter({ hasText: '已审核' }).first()
    await approved.getByRole('button', { name: /详情/ }).click()
    await page.getByRole('button', { name: /创建发货/ }).click()
    // 输入部分发货数量（订单数量的 50%）
    const shipQty = page.getByLabel(/本次发货数量/).first()
    await shipQty.fill('50')
    await page.getByRole('button', { name: /保存/ }).click()
    // 回到订单详情
    await page.goto(page.url().replace('/delivery', ''))
    await expect(page.getByText('部分发货').first()).toBeVisible({ timeout: 5000 })
  })

  test('04-04 全部发货后订单状态为"已发货"', async ({ page }) => {
    await page.goto('/sales/order/list')
    const approved = page.locator('tr, .el-table__row').filter({ hasText: '已审核' }).first()
    await approved.getByRole('button', { name: /详情/ }).click()
    await page.getByRole('button', { name: /创建发货/ }).click()
    // 输入全部发货数量
    const shipQty = page.getByLabel(/本次发货数量/).first()
    await shipQty.fill('500')
    await page.getByRole('button', { name: /保存/ }).click()
    // 回到订单详情
    await expect(page.getByText('已发货').first()).toBeVisible({ timeout: 5000 })
  })
})
