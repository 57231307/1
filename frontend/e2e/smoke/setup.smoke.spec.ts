import { test, expect } from '@playwright/test';

test.describe('setup 页面冒烟测试', () => {
  test('页面加载 + 初始化表单', async ({ page }) => {
    await page.goto('/setup');
    await expect(page.locator('form, .setup-container, .el-form').first()).toBeAttached({ timeout: 30_000 });
  });
});
