import { test, expect } from '@playwright/test';
import { login } from '../fixtures/real-auth';

test.describe('finance-report 页面冒烟测试', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('页面加载', async ({ page }) => {
    await page.goto('/finance-report');
    await expect(page.locator('.el-table, .el-table-v2, .el-card, .el-form, .el-tabs, .dashboard-container, canvas, .echarts').first()).toBeAttached({ timeout: 10_000 });
  });
});