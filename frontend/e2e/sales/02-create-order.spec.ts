// P9-3 销售 E2E 套件 — 02 创建销售订单
// 创建时间: 2026-06-17
// 覆盖范围：销售订单创建全流程（5 用例）

import { test, expect } from '@playwright/test'
import { applyAuthMocks } from '../smoke/_helpers'

/**
 * 测试套件：销售订单创建
 *
 * 业务流程：
 * 1. 报价单转销售订单
 * 2. 直接创建销售订单
 * 3. 多产品行/双计量单位（米+公斤）
 * 4. 颜色/等级要求
 * 5. 批次要求
 */
test.describe('02 创建销售订单', () => {
  test.beforeEach(async ({ page, context }) => {
    // V15 Batch 487 P0-T05：注入 auth mock，业务 API 走真实后端（applyAuthMocks 不再 mock 业务 API）
    await applyAuthMocks(context)
    await page.goto('/')
  })

  test('02-01 销售订单列表可访问', async ({ page }) => {
    await page.goto('/sales/order/list')
    await expect(page.getByText('销售订单列表')).toBeVisible()
    await expect(page.getByRole('button', { name: /新建销售订单/ })).toBeVisible()
  })

  test('02-02 从报价单一键转销售订单', async ({ page }) => {
    await page.goto('/sales/quotation/list')
    // 找到已审批通过的报价单
    const approved = page.locator('tr, .el-table__row').filter({ hasText: '已审批' }).first()
    await approved.getByRole('button', { name: /转订单/ }).click()
    // 确认对话框
    await page.getByRole('button', { name: /确定/ }).click()
    // 跳转到销售订单创建页（带预填数据）
    await expect(page).toHaveURL(/\/sales\/order\/create/)
    // 客户/产品应已预填
    const customerInput = page.getByLabel(/客户/).first()
    await expect(customerInput).not.toHaveValue('')
  })

  test('02-03 创建带双计量单位（米 + 公斤）的销售订单', async ({ page }) => {
    await page.goto('/sales/order/create')
    // 选客户
    await page.getByLabel(/客户/).first().click()
    await page.getByRole('option').first().click()
    // 选产品
    await page.getByLabel(/产品/).first().click()
    await page.getByRole('option').first().click()
    // 米数
    await page.getByLabel(/数量.*米|米数/).fill('500')
    // 公斤数
    await page.getByLabel(/数量.*公斤|公斤数/).fill('120')
    // 单价
    await page.getByLabel(/单价/).fill('35.50')
    await page.getByRole('button', { name: /保存/ }).click()
    await expect(page.getByText(/销售订单号.*SO-\d{8}-\d{4}/)).toBeVisible({ timeout: 5000 })
  })

  test('02-04 创建带颜色与等级要求的销售订单', async ({ page }) => {
    await page.goto('/sales/order/create')
    await page.getByLabel(/客户/).first().click()
    await page.getByRole('option').first().click()
    await page.getByLabel(/产品/).first().click()
    await page.getByRole('option').first().click()
    // 色号
    await page.getByLabel(/色号/).fill('CN-2026-001')
    await page.getByLabel(/潘通色号/).fill('PANTONE-18-1664')
    await page.getByLabel(/等级/).fill('A')
    await page.getByLabel(/数量/).fill('200')
    await page.getByLabel(/单价/).fill('45.00')
    await page.getByRole('button', { name: /保存/ }).click()
    await expect(page.getByText(/保存成功|创建成功/)).toBeVisible()
  })

  test('02-05 销售订单行项可动态增删', async ({ page }) => {
    await page.goto('/sales/order/create')
    // 添加 3 行产品
    for (let i = 0; i < 3; i++) {
      await page.getByRole('button', { name: /添加行项|新增行/ }).click()
    }
    // 验证有 3 个产品行
    const rows = page.locator('[data-testid="order-item-row"]')
    await expect(rows).toHaveCount(3)
    // 删除第 2 行
    await rows.nth(1).getByRole('button', { name: /删除/ }).click()
    await expect(rows).toHaveCount(2)
  })
})
