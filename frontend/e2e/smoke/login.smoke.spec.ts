import { test, expect } from '@playwright/test';

test.describe('login 页面冒烟测试', () => {
  test('页面加载 + 登录表单渲染', async ({ page }) => {
    await page.goto('/login');
    await expect(page.locator('input[name="username"]')).toBeVisible({ timeout: 10_000 });
    await expect(page.locator('input[name="password"]')).toBeVisible({ timeout: 5_000 });
    await expect(page.locator('button[type="submit"]')).toBeVisible({ timeout: 5_000 });
    await expect(page.locator('.el-checkbox')).toBeVisible({ timeout: 5_000 });
  });
});