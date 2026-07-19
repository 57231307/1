// P9-4 采购 E2E 套件 — 02 采购订单审批
// 创建时间: 2026-06-17
// 覆盖范围：采购订单审批工作流（3 用例）

import { test, expect } from '@playwright/test'
import { applyAuthMocks } from '../smoke/_helpers'

/**
 * 测试套件：采购订单审批
 *
 * 业务流程：
 * 1. 提交审批（草稿 → 待审）
 * 2. 审批通过
 * 3. 审批驳回
 */
test.describe('02 采购订单审批', () => {
  test.beforeEach(async ({ page, context }) => {
    // V15 Batch 487 P0-T05：注入 auth mock，业务 API 走真实后端（applyAuthMocks 不再 mock 业务 API）
    await applyAuthMocks(context)
    await page.goto('/')
  })

  test('02-01 草稿采购订单可提交审批', async ({ page }) => {
    await page.goto('/purchase/order/list')
    const draft = page.locator('tr, .el-table__row').filter({ hasText: '草稿' }).first()
    await draft.getByRole('button', { name: /详情/ }).click()
    await expect(page).toHaveURL(/\/purchase\/order\/detail\/\d+/)
    // 提交审批
    await page.getByRole('button', { name: /提交审批/ }).click()
    // 状态变为"待审核"
    await expect(page.getByText('待审核').first()).toBeVisible({ timeout: 5000 })
  })

  test('02-02 审批人可通过采购订单', async ({ page }) => {
    await page.goto('/purchase/order/list')
    const pending = page.locator('tr, .el-table__row').filter({ hasText: '待审核' }).first()
    await pending.getByRole('button', { name: /详情/ }).click()
    // 审批操作
    await page.getByRole('button', { name: /审批/ }).click()
    // 审批意见
    await page.getByLabel(/审批意见|备注/).fill('E2E 采购审批通过')
    await page.getByRole('button', { name: /通过/ }).click()
    // 状态变为"已审核"
    await expect(page.getByText('已审核').first()).toBeVisible({ timeout: 5000 })
  })

  test('02-03 审批人可驳回采购订单', async ({ page }) => {
    await page.goto('/purchase/order/list')
    const pending = page.locator('tr, .el-table__row').filter({ hasText: '待审核' }).first()
    await pending.getByRole('button', { name: /详情/ }).click()
    await page.getByRole('button', { name: /审批/ }).click()
    await page.getByLabel(/审批意见|驳回原因/).fill('E2E 测试驳回：数量超预算')
    await page.getByRole('button', { name: /驳回/ }).click()
    // 状态回到"草稿"或显示"已驳回"
    await expect(page.getByText(/草稿|已驳回/).first()).toBeVisible({ timeout: 5000 })
  })
})
