import { test, expect } from '@playwright/test';

test.describe('ar-reconciliation 页面冒烟测试', () => {
  test('页面加载', async ({ page }) => {
    await page.goto('/ar-reconciliation');
    await expect(page.locator('.el-table, .el-table-v2, .el-card, .el-form, .el-tabs, .dashboard-container, canvas, .echarts').first()).toBeAttached({ timeout: 30_000 });
  });
});
