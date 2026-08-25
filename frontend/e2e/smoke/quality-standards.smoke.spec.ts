import { test, expect } from '@playwright/test';
import { gotoWithRetry } from './_goto';

test.describe('quality-standards 页面冒烟测试', () => {
  test('页面加载', async ({ page }) => {
    await gotoWithRetry(page, '/quality-standards');
    await expect(page.locator('body')).toBeVisible({ timeout: 30_000 });
    await expect(page.locator('.el-table, .el-table-v2, .el-card, .el-form, .el-tabs, .dashboard-container, canvas, .echarts, .el-result, .error-page, .el-empty').first()).toBeAttached({ timeout: 30_000 });
  });
});
