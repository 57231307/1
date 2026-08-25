import { test, expect } from '@playwright/test';

test.describe('login 页面冒烟测试', () => {
  test('页面加载 + 登录表单渲染', async ({ page }) => {
    await page.goto('/login');
    await expect(page.locator('input[placeholder="用户名"], input[placeholder="Username"]').first()).toBeVisible({ timeout: 30_000 });
    await expect(page.locator('input[placeholder="密码"], input[placeholder="Password"]').first()).toBeVisible({ timeout: 30_000 });
    await expect(page.locator('form button.el-button--primary').first()).toBeVisible({ timeout: 30_000 });
    await expect(page.locator('.el-checkbox').first()).toBeVisible({ timeout: 30_000 });
  });
});
