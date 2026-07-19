// P9-4 采购 E2E 套件 — 07 供应商报表
// 创建时间: 2026-06-17
// 覆盖范围：供应商分析与统计（6 用例）

import { test, expect } from '@playwright/test'
import { applyAuthMocks } from '../smoke/_helpers'

/**
 * 测试套件：供应商报表
 *
 * 业务流程：
 * 1. 供应商采购汇总
 * 2. 供应商到货及时率
 * 3. 供应商质检合格率
 * 4. 供应商账期与应付余额
 * 5. 供应商评级
 * 6. 导出 Excel
 */
test.describe('07 供应商报表', () => {
  test.beforeEach(async ({ page, context }) => {
    // V15 Batch 487 P0-T05：注入 auth mock，业务 API 走真实后端（applyAuthMocks 不再 mock 业务 API）
    await applyAuthMocks(context)
    await page.goto('/')
  })

  test('07-01 供应商采购汇总（按月份）', async ({ page }) => {
    await page.goto('/purchase/report/supplier-summary')
    await page.getByLabel(/统计维度/).click()
    await page.getByRole('option', { name: /按月/ }).click()
    await page.getByLabel(/起始月份/).fill('2026-01')
    await page.getByLabel(/截止月份/).fill('2026-06')
    await page.getByRole('button', { name: /查询/ }).click()
    const rows = page.locator('tr, .el-table__row').filter({ hasText: '2026' })
    await expect(rows.first()).toBeVisible({ timeout: 5000 })
  })

  test('07-02 供应商到货及时率统计', async ({ page }) => {
    await page.goto('/purchase/report/on-time-rate')
    await page.getByLabel(/起始日期/).fill('2026-01-01')
    await page.getByLabel(/截止日期/).fill('2026-06-30')
    await page.getByRole('button', { name: /查询/ }).click()
    // 验证"及时率"列
    await expect(page.getByText(/及时率.*%/)).toBeVisible({ timeout: 5000 })
  })

  test('07-03 供应商质检合格率统计', async ({ page }) => {
    await page.goto('/purchase/report/quality-rate')
    await page.getByLabel(/年度/).fill('2026')
    await page.getByRole('button', { name: /查询/ }).click()
    // 验证"合格率"列
    await expect(page.getByText(/合格率.*%/)).toBeVisible({ timeout: 5000 })
  })

  test('07-04 供应商账期与应付余额', async ({ page }) => {
    await page.goto('/purchase/report/ap-aging')
    await expect(page.getByText('应付账龄分析')).toBeVisible()
    // 验证账龄分组列：30天内 / 30-60天 / 60-90天 / 90天以上
    await expect(page.getByText(/30 天内/)).toBeVisible()
    await expect(page.getByText(/30-60 天/)).toBeVisible()
    await expect(page.getByText(/60-90 天/)).toBeVisible()
    await expect(page.getByText(/90 天以上/)).toBeVisible()
  })

  test('07-05 供应商评级查看', async ({ page }) => {
    await page.goto('/purchase/report/supplier-grade')
    // 验证有评级 A/B/C/D 列
    await expect(page.getByText(/A 级/)).toBeVisible()
    await expect(page.getByText(/B 级/)).toBeVisible()
    await expect(page.getByText(/C 级/)).toBeVisible()
    await expect(page.getByText(/D 级/)).toBeVisible()
  })

  test('07-06 供应商报表可导出 Excel', async ({ page }) => {
    await page.goto('/purchase/report/supplier-summary')
    await page.getByRole('button', { name: /查询/ }).click()
    const downloadPromise = page.waitForEvent('download')
    await page.getByRole('button', { name: /导出/ }).click()
    const download = await downloadPromise
    expect(download.suggestedFilename()).toMatch(/\.xlsx?$/)
  })
})
