// 库存管理 E2E 套件 — 01 库存台账
// 覆盖范围：库存台账列表加载、筛选、统计卡片
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('库存管理 - 01 库存台账', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('库存管理页面可访问', async ({ page }) => {
    await page.goto('/inventory');
    await expect(page.getByText(/库存管理/)).toBeVisible();
  });

  test('库存台账 Tab 数据加载', async ({ page }) => {
    await page.goto('/inventory');
    await expect(page.getByRole('tab', { name: /库存台账/ })).toBeVisible();
    await page.getByRole('tab', { name: /库存台账/ }).click();
    await expect(page.locator('.v2-table, .el-table')).toBeVisible({ timeout: 30000 });
  });

  test('库存筛选功能', async ({ page }) => {
    await page.goto('/inventory');
    await page.getByLabel(/仓库/).click();
    await expect(page.getByRole('option')).toBeVisible({ timeout: 30000 });
    await page.keyboard.press('Escape');
    await page.getByLabel(/状态/).click();
    await expect(page.getByRole('option')).toBeVisible({ timeout: 30000 });
    await page.keyboard.press('Escape');
    await page.getByRole('button', { name: /查询/ }).click();
  });

  test('库存预警 Tab', async ({ page }) => {
    await page.goto('/inventory');
    await page.getByRole('tab', { name: /库存预警/ }).click();
    await expect(page.locator('.el-table')).toBeVisible({ timeout: 30000 });
    await expect(page.getByText(/预警等级|紧急|预警/)).toBeVisible({ timeout: 30000 });
  });
});