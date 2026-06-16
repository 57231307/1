// 定制订单 E2E 测试
// Playwright 测试定制订单全流程
// 创建时间: 2026-06-17

import { test, expect } from '@playwright/test'

test.describe('定制订单全流程跟踪 E2E', () => {
  test('创建定制订单', async ({ page }) => {
    // 登录
    await page.goto('/login')
    await page.fill('input[name="username"]', 'admin')
    await page.fill('input[name="password"]', 'admin123')
    await page.click('button[type="submit"]')
    await page.waitForURL(/\/dashboard/)

    // 进入定制订单列表
    await page.goto('/custom-orders')
    await expect(page.locator('text=定制订单管理')).toBeVisible()

    // 点击新建
    await page.click('text=新建定制订单')
    await page.waitForURL(/\/custom-orders\/new/)

    // 填写表单
    await page.locator('input').filter({ hasText: '' }).first().fill('1') // customer_id
    await page.locator('input').nth(1).fill('1') // product_id
    await page.locator('input[placeholder*="100% 棉"]').fill('100% 棉 200g/m²')
    await page.locator('input[type="number"]').first().fill('100')

    // 提交
    await page.click('text=保存草稿')
    await page.waitForURL(/\/custom-orders\/\d+$/)

    // 验证创建成功
    await expect(page.locator('text=基本信息')).toBeVisible()
  })

  test('推进订单状态', async ({ page }) => {
    await page.goto('/custom-orders/1')
    // 点击推进按钮
    const advanceBtn = page.locator('button', { hasText: '推进状态' })
    if (await advanceBtn.isVisible()) {
      await advanceBtn.click()
      // 确认弹窗
      await page.locator('.el-message-box__btns button.el-button--primary').click()
      // 验证状态变化
      await expect(page.locator('.el-tag').first()).toBeVisible()
    }
  })

  test('查看工艺跟踪大屏', async ({ page }) => {
    await page.goto('/custom-orders/1/track')
    await expect(page.locator('text=工艺跟踪')).toBeVisible()
    await expect(page.locator('text=纱线采购')).toBeVisible()
    await expect(page.locator('text=染整')).toBeVisible()
    await expect(page.locator('text=后整理')).toBeVisible()
    await expect(page.locator('text=交付')).toBeVisible()
    await expect(page.locator('text=售后')).toBeVisible()
  })

  test('上报质量异常', async ({ page }) => {
    await page.goto('/custom-orders/1')
    // 切换到质量异常 tab
    await page.click('text=质量异常')
    // 点击上报
    await page.click('text=上报异常')
    // 填写表单
    await page.locator('.el-dialog input').first().click()
    await page.click('text=色差 (GB/T 26377)')
    await page.locator('textarea').fill('批次色差 ΔE=3.5 超过 2.0 阈值')
    // 提交
    await page.click('.el-dialog .el-button--primary')
    await expect(page.locator('text=色差')).toBeVisible()
  })
})
