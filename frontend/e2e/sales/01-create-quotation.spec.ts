// P9-3 销售 E2E 套件 — 01 创建报价单
// 创建时间: 2026-06-17
// 覆盖范围：销售报价单创建全流程（5 用例）

import { test, expect } from '@playwright/test'

/**
 * 测试套件：销售报价单创建
 *
 * 业务流程：
 * 1. 进入销售 → 报价单 → 新建报价单
 * 2. 选择客户（必填）
 * 3. 选择产品行（必填 ≥1 行）
 * 4. 设置数量、单价、折扣、税率
 * 5. 提交并验证报价单号生成
 */
test.describe('01 创建报价单', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
  })

  test('01-01 进入报价单列表页可见菜单', async ({ page }) => {
    await page.goto('/sales/quotation/list')
    await expect(page.getByText('报价单列表')).toBeVisible()
    // 验证列表区域有"新建报价单"按钮
    await expect(page.getByRole('button', { name: /新建报价单/ })).toBeVisible()
  })

  test('01-02 创建空报价单应校验失败', async ({ page }) => {
    await page.goto('/sales/quotation/create')
    // 不填任何字段直接提交
    await page.getByRole('button', { name: /保存/ }).click()
    // 应显示客户/产品必填错误
    await expect(page.getByText(/客户.*必选|请选择客户/)).toBeVisible()
  })

  test('01-03 创建有效报价单成功并生成报价单号', async ({ page }) => {
    await page.goto('/sales/quotation/create')
    // 选择客户
    await page.getByLabel(/客户/).first().click()
    await page.getByRole('option').first().click()
    // 选择产品
    await page.getByLabel(/产品/).first().click()
    await page.getByRole('option').first().click()
    // 数量
    await page.getByLabel(/数量/).fill('100')
    // 单价
    await page.getByLabel(/单价/).fill('50.00')
    // 提交
    await page.getByRole('button', { name: /保存/ }).click()
    // 验证成功提示 + 报价单号格式 Q-YYYYMMDD-XXXX
    await expect(page.getByText(/报价单号.*Q-\d{8}-\d{4}/)).toBeVisible({ timeout: 5000 })
  })

  test('01-04 报价单草稿可保存后再次编辑', async ({ page }) => {
    await page.goto('/sales/quotation/list')
    // 找到第一条草稿
    const draft = page.getByText('草稿').first()
    await draft.click()
    // 跳转到详情
    await expect(page).toHaveURL(/\/sales\/quotation\/detail\/\d+/)
    await page.getByRole('button', { name: /编辑/ }).click()
    // 备注
    await page.getByLabel(/备注/).fill('E2E 测试备注')
    await page.getByRole('button', { name: /保存/ }).click()
    await expect(page.getByText(/保存成功|更新成功/)).toBeVisible()
  })

  test('01-05 报价单可复制为新单', async ({ page }) => {
    await page.goto('/sales/quotation/list')
    const firstRow = page.locator('tr, .el-table__row').nth(1)
    await firstRow.getByRole('button', { name: /复制/ }).click()
    // 应跳转到新建页面并预填数据
    await expect(page).toHaveURL(/\/sales\/quotation\/create/)
    // 客户字段应已预填
    const customerInput = page.getByLabel(/客户/).first()
    await expect(customerInput).not.toHaveValue('')
  })
})
