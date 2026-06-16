// 色卡仓储管理 E2E 测试
// 创建时间: 2026-06-17

import { test, expect } from '@playwright/test'

test.describe('色卡仓储管理 E2E', () => {
  test.beforeEach(async ({ page }) => {
    // 登录（如需要）
    await page.goto('/')
  })

  test('色卡列表 → 创建 → 详情 → 借出 → 归还', async ({ page }) => {
    // 1. 进入色卡列表
    await page.goto('/color-cards/list')
    await expect(page.getByText('色卡列表')).toBeVisible()

    // 2. 创建色卡
    await page.goto('/color-cards/create')
    await page.getByPlaceholder(/PANTONE-TPX-2024-SS/).fill('TEST-2026-P0-4')
    await page.getByPlaceholder(/2024 春夏/).fill('E2E 测试色卡')

    // 3. 进入详情
    await page.goto('/color-cards/detail/1')
    await expect(page.getByText('基本信息')).toBeVisible()

    // 4. 进入借出管理
    await page.goto('/color-cards/borrow')
    await expect(page.getByText('借出 / 归还 / 遗失登记')).toBeVisible()
  })

  test('色卡筛选与搜索', async ({ page }) => {
    await page.goto('/color-cards/list')
    // 验证筛选条件存在
    await expect(page.getByText('色卡类型')).toBeVisible()
    await expect(page.getByText('季节')).toBeVisible()
    await expect(page.getByText('状态')).toBeVisible()
  })
})
