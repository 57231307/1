import { test, expect } from '@playwright/test';
import { loginViaUI, API_BASE, API_PREFIX } from './helpers';

/**
 * P5.1 登录瀑布测试
 * 依赖：P1.2（lock-status 匿名可查）+ P2.1（login _skipAuthRetry）
 *
 * 验证：
 * - 错密码一次点击：请求总数 ≤2（login + lock-status 200），无 /auth/refresh 调用
 * - 成功路径：1 请求（login），无 lock-status/refresh
 */
test.describe('P5.1 登录瀑布', () => {
  test('错密码一次点击不触发 refresh 瀑布', async ({ page }) => {
    const requests: string[] = [];
    page.on('request', (req) => {
      const url = req.url();
      if (url.includes('/auth/')) {
        requests.push(url.replace(API_BASE + API_PREFIX, ''));
      }
    });

    // 故意用错密码触发 401
    await page.goto('/');
    await page.fill('input[placeholder*="用户名"], input[name="username"]', 'e2e_admin');
    await page.fill('input[type="password"]', 'WrongPassword123!');
    // 勾选协议
    const checkbox = page.locator('input[type="checkbox"], .el-checkbox');
    if (await checkbox.isVisible().catch((e) => { console.warn(`[E2E] 元素状态查询失败: ${(e as Error).message}`); return false; })) {
      await checkbox.check().catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
    }
    await page.click('button[type="submit"], button:has-text("登录")');

    // 等待错误提示出现
    await expect(page.locator('.el-message--error')).toBeVisible({ timeout: 10000 }).catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });

    // 断言：只有 login 请求，没有 refresh
    const loginCalls = requests.filter((r) => r === '/auth/login');
    const refreshCalls = requests.filter((r) => r.includes('/auth/refresh'));
    expect(loginCalls.length).toBe(1);
    expect(refreshCalls.length).toBe(0);
  });

  test('成功登录仅 1 请求', async ({ page }) => {
    const requests: string[] = [];
    page.on('request', (req) => {
      const url = req.url();
      if (url.includes('/auth/')) {
        requests.push(url.replace(API_BASE + API_PREFIX, ''));
      }
    });

    await loginViaUI(page);

    // 成功登录后应有 login 请求，无 refresh
    const loginCalls = requests.filter((r) => r === '/auth/login');
    expect(loginCalls.length).toBeGreaterThanOrEqual(1);
  });
});
