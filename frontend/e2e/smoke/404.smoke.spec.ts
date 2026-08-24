import { test, expect } from '@playwright/test';

test.describe('404 页面冒烟测试', () => {
  test('页面加载 + 未找到提示', async ({ page }) => {
    await page.goto('/404');
    await expect(page.locator('text=404, text=未找到, text=Not Found').first()).toBeAttached({ timeout: 30_000 });
  });
});