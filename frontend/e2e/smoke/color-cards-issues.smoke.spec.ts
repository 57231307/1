import { test, expect } from '@playwright/test';


test.describe('color-cards issues 页面冒烟测试', () => {
  test.beforeEach(async ({ page, context }) => {
    await context.route('**/api/**', async (route) => {
      await route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ code: 200, data: { items: [], total: 0 } }) });
    });
  });

  test('页面加载', async ({ page }) => {
    await page.goto('/color-cards/issues');
    await expect(page.locator('.el-table, .el-table-v2, .el-card, .el-form, .el-tabs, .dashboard-container, canvas, .echarts').first()).toBeAttached({ timeout: 30_000 });
  });
});