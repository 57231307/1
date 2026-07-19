// P9-3 销售 E2E 套件 — 07 销售统计报表
// 创建时间: 2026-06-17
// 覆盖范围：销售统计与报表（6 用例）

import { test, expect } from '@playwright/test'
import { applyAuthMocks } from '../smoke/_helpers'

/**
 * 测试套件：销售统计
 *
 * 业务流程：
 * 1. 销售订单汇总（按月/客户/产品）
 * 2. 销售业绩排行
 * 3. 销售回款率
 * 4. 销售毛利率
 * 5. 退货率
 * 6. 导出 Excel
 */
test.describe('07 销售统计报表', () => {
  test.beforeEach(async ({ page, context }) => {
    // V15 Batch 487 P0-T05：注入 auth mock，业务 API 走真实后端（applyAuthMocks 不再 mock 业务 API）
    await applyAuthMocks(context)
    await page.goto('/')
  })

  test('07-01 销售订单汇总按月份分组', async ({ page }) => {
    await page.goto('/sales/report/order-summary')
    await page.getByLabel(/统计维度/).click()
    await page.getByRole('option', { name: /按月/ }).click()
    await page.getByLabel(/起始月份/).fill('2026-01')
    await page.getByLabel(/截止月份/).fill('2026-06')
    await page.getByRole('button', { name: /查询/ }).click()
    // 验证有数据行
    const rows = page.locator('tr, .el-table__row').filter({ hasText: '2026' })
    await expect(rows.first()).toBeVisible({ timeout: 5000 })
  })

  test('07-02 销售订单汇总按客户分组', async ({ page }) => {
    await page.goto('/sales/report/order-summary')
    await page.getByLabel(/统计维度/).click()
    await page.getByRole('option', { name: /按客户/ }).click()
    await page.getByRole('button', { name: /查询/ }).click()
    // 验证有客户名列
    await expect(page.getByText(/客户名称|客户/)).toBeVisible()
  })

  test('07-03 销售业绩排行（销售员 TOP 10）', async ({ page }) => {
    await page.goto('/sales/report/performance')
    await expect(page.getByText('销售业绩')).toBeVisible()
    // 验证有 TOP 10 排名
    const rows = page.locator('tr, .el-table__row')
    await expect(rows).toHaveCount(11, { timeout: 5000 }) // 1 表头 + 10 数据
  })

  test('07-04 销售回款率统计', async ({ page }) => {
    await page.goto('/sales/report/collection-rate')
    await page.getByLabel(/年度/).fill('2026')
    await page.getByRole('button', { name: /查询/ }).click()
    // 验证有"回款率"列
    await expect(page.getByText(/回款率.*%/)).toBeVisible({ timeout: 5000 })
  })

  test('07-05 销售毛利率统计', async ({ page }) => {
    await page.goto('/sales/report/gross-margin')
    await page.getByLabel(/起始月份/).fill('2026-01')
    await page.getByLabel(/截止月份/).fill('2026-06')
    await page.getByRole('button', { name: /查询/ }).click()
    // 验证毛利率列
    await expect(page.getByText(/毛利率.*%/)).toBeVisible({ timeout: 5000 })
  })

  test('07-06 销售报表可导出 Excel', async ({ page }) => {
    await page.goto('/sales/report/order-summary')
    await page.getByRole('button', { name: /查询/ }).click()
    // 触发下载
    const downloadPromise = page.waitForEvent('download')
    await page.getByRole('button', { name: /导出/ }).click()
    const download = await downloadPromise
    expect(download.suggestedFilename()).toMatch(/\.xlsx?$/)
  })
})
