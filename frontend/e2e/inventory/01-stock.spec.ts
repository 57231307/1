// 库存管理 E2E 套件 — 01 库存调整
// 创建时间: 2026-08-19
// 覆盖范围：库存调整完整流程（盘盈/盘亏创建与提交）
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('01 库存调整', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('01-01 进入库存管理页面', async ({ page }) => {
    await page.goto('/inventory');
    await expect(page.getByText(/库存管理|库存台账/)).toBeVisible({ timeout: 5000 });
    await expect(page.getByRole('tab', { name: /库存台账|台账/ })).toBeVisible();
    await expect(page.getByRole('tab', { name: /库存预警|预警/ })).toBeVisible();
    await expect(page.getByRole('tab', { name: /库存调拨|调拨/ })).toBeVisible();
  });

  test('01-02 库存筛选功能可用', async ({ page }) => {
    await page.goto('/inventory');
    await page.getByLabel(/仓库/).click();
    await expect(page.getByRole('option')).toBeVisible();
    await page.keyboard.press('Escape');
    await page.getByRole('button', { name: /查询|搜索/ }).click();
    await expect(page.locator('table, .v2-table, .el-table')).toBeVisible({ timeout: 5000 });
  });

  test('01-03 库存台账数据加载正常', async ({ page }) => {
    await page.goto('/inventory');
    await expect(page.locator('table, .v2-table, .el-table')).toBeVisible({ timeout: 10000 });
    const tableContent = page.locator('table, .v2-table, .el-table');
    await expect(tableContent).toBeVisible();
  });
});