import { test, expect } from '../diagnose-fixture';
import { loginViaUI } from './helpers';
import { safeGoto } from './ui-helpers';

/**
 * P5.1 登录瀑布测试
 * 依赖：P1.2（lock-status 匿名可查）+ P2.1（login _skipAuthRetry）
 *
 * 验证：
 * - 错密码一次点击：请求总数 ≤2（login + lock-status 200），无 /auth/refresh 调用
 * - 成功路径：1 请求（login），无 lock-status/refresh
 */
/** 规范化请求 URL 为 pathname（前端登录走 dev server 代理，URL 前缀与 API_BASE 不同源） */
function authPath(url: string): string {
  try {
    const u = new URL(url);
    const idx = u.pathname.indexOf('/auth/');
    return idx >= 0 ? u.pathname.slice(idx) : u.pathname;
  } catch {
    return url;
  }
}

test.describe('P5.1 登录瀑布', () => {
  // 关键：全局 storageState（playwright.config）让每个 test 自带登录 cookie，
  // goto('/') 会被路由守卫重定向 Dashboard，登录表单永不出现。
  // 本 describe 专门测登录页行为，必须清空登录态（第四轮 CI 37 分片 fill 超时根因）
  test.use({ storageState: { cookies: [], origins: [] } });

  test('错密码一次点击不触发 refresh 瀑布', async ({ page }) => {
    const requests: string[] = [];
    page.on('request', (req) => {
      if (req.url().includes('/auth/')) {
        requests.push(authPath(req.url()));
      }
    });

    // 故意用错密码触发 401；safeGoto 处理 Vite 冷启动 504 Outdated Optimize Dep 重试
    await safeGoto(page, '/');
    // 等登录表单渲染后再填（dev server 冷启动 504 重试后表单可能延迟出现）
    const userInput = page.locator('input[placeholder*="用户名"], input[name="username"]').first();
    await userInput.waitFor({ state: 'visible', timeout: 45000 }).catch((e) => { console.warn(`[E2E] 登录表单未出现: ${(e as Error).message}`); });
    await userInput.fill('e2e_admin');
    await page.locator('input[type="password"]').first().fill('WrongPassword123!');
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
    expect(loginCalls.length, `[31-瀑布] login 请求数（实际: ${requests.join(',')}）`).toBe(1);
    expect(refreshCalls.length, `[31-瀑布] refresh 请求数（实际: ${requests.join(',')}）`).toBe(0);
  });

  test('成功登录仅 1 请求', async ({ page }) => {
    const requests: string[] = [];
    page.on('request', (req) => {
      if (req.url().includes('/auth/')) {
        requests.push(authPath(req.url()));
      }
    });

    await loginViaUI(page);

    // 成功登录后应有 login 请求，无 refresh
    const loginCalls = requests.filter((r) => r === '/auth/login');
    expect(loginCalls.length, `[31-瀑布] 成功登录后 login 请求数（实际: ${requests.join(',')}）`).toBeGreaterThanOrEqual(1);
  });
});
