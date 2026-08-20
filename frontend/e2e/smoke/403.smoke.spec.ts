import { test, expect } from '@playwright/test';

test.describe('403 页面冒烟测试', () => {
  test('页面加载 + 无权限提示', async ({ page }) => {
    await page.goto('/403');
    await expect(page.locator('text=403, text=无权限, text=Forbidden').first()).toBeAttached({ timeout: 10_000 });
  });
});