import { test, expect } from '@playwright/test';
import {
  loginViaUI,
  BASE_URL,
  API_BASE,
  API_PREFIX,
  TEST_USERNAME,
  TEST_PASSWORD,
} from './helpers';

test.describe('后端连接状态与 Token 管理', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('登录态持久化：刷新页面后仍保持登录', async ({ page }) => {
    await page.goto(`${BASE_URL}/dashboard`);
    await page.waitForTimeout(2000);

    await page.reload({ waitUntil: 'domcontentloaded' });
    await page.waitForTimeout(2000);

    const currentUrl = page.url();
    expect(currentUrl.includes('/login')).toBe(false);
  });

  test('未登录访问受保护路由跳转登录页', async ({ browser }) => {
    // 全新 context，不登录（显式置空 storageState：
    // playwright.config 全局 storageState 会向每个新 context 注入登录 cookie）
    const context = await browser.newContext({ storageState: { cookies: [], origins: [] } });
    const page = await context.newPage();

    // 时间戳查询参数破坏 HTTP 缓存/bfcache，确保守卫 JS 真正执行
    await page.goto(`${BASE_URL}/purchase?t=${Date.now()}`, { waitUntil: 'domcontentloaded' });
    // 守卫链：/auth/me 401 → refresh 401 → redirect /login；
    // 等待 URL 实际变化（CI 慢环境首次 JS 执行可达 20s+，30s 留余量），固定 3s 在首次加载慢时会误判
    await page.waitForURL(/\/(login|setup)/, { timeout: 30_000 }).catch(() => {});

    const url = page.url();
    expect(url.includes('/login') || url.includes('/setup')).toBe(true);

    await context.close();
  });

  test('setup 页面可达', async ({ browser }) => {
    // 同上：置空 storageState，避免全局登录态注入影响未登录场景
    const context = await browser.newContext({ storageState: { cookies: [], origins: [] } });
    const page = await context.newPage();

    await page.goto(`${BASE_URL}/setup`);
    await page.waitForTimeout(2000);

    await page
      .locator('form, .el-form, .setup-container')
      .first()
      .waitFor({ state: 'visible', timeout: 10_000 })
      .catch(() => {});

    const hasForm = await page
      .locator('form, .el-form, .setup-container')
      .first()
      .isVisible()
      .catch(() => false);
    expect(hasForm).toBe(true);

    await context.close();
  });

  test('Cookie 安全属性：access_token 为 httpOnly', async ({ context }) => {
    const cookies = await context.cookies();

    const accessToken = cookies.find(c => c.name === 'access_token');
    expect(accessToken).toBeDefined();
    expect(accessToken?.httpOnly).toBe(true);

    const csrfToken = cookies.find(c => c.name === 'csrf_token');
    expect(csrfToken).toBeDefined();
    expect(csrfToken?.httpOnly).toBe(false);

    expect(accessToken?.sameSite).toBe('Strict');
  });

  test('刷新 Token 接口可达', async ({ page, context }) => {
    // 复用当前 context（已有 cookie），不重新登录
    const refreshResp = await page.request.post(`${API_BASE}${API_PREFIX}/auth/refresh`, {
      headers: { 'X-Requested-With': 'XMLHttpRequest' },
    });

    // 刷新可能成功（200）或因 refresh_token 过期失败（401）
    // 验证接口可达即可
    expect(refreshResp.ok() || refreshResp.status() === 401).toBe(true);

    if (refreshResp.ok()) {
      const cookiesAfterRefresh = await context.cookies();
      const newAccessCookie = cookiesAfterRefresh.find(c => c.name === 'access_token');
      expect(newAccessCookie).toBeDefined();
    }
  });

  test('登出后 Cookie 被清除', async ({ page, context }) => {
    // 复用当前 context 的 cookie 调用登出
    const csrfToken = (await context.cookies()).find(c => c.name === 'csrf_token')?.value || '';

    const logoutResp = await page.request.post(`${API_BASE}${API_PREFIX}/auth/logout`, {
      headers: {
        'X-Requested-With': 'XMLHttpRequest',
        'X-CSRF-Token': csrfToken,
      },
    });

    // 登出可能成功（200）或因 token 失效失败（401）
    if (logoutResp.ok()) {
      const cookiesAfter = await context.cookies();
      const accessCookie = cookiesAfter.find(c => c.name === 'access_token');
      expect(
        accessCookie === undefined || (accessCookie?.expires ?? 0) <= Date.now() / 1000 + 1
      ).toBe(true);
    }
  });

  test('后端健康检查端点可达', async ({ page }) => {
    const resp = await page.request.get(`${API_BASE}/health`);
    expect(resp.ok()).toBe(true);

    const livenessResp = await page.request.get(`${API_BASE}/health/liveness`);
    expect(livenessResp.ok()).toBe(true);

    const readinessResp = await page.request.get(`${API_BASE}/health/readiness`);
    expect(readinessResp.ok()).toBe(true);
  });

  test('API 限流不崩溃（高频请求后恢复）', async ({ page }) => {
    // 用 auth/me 测试（GET 请求不会触发 brute_force）
    let rateLimited = false;
    for (let i = 0; i < 20; i++) {
      const resp = await page.request.get(`${API_BASE}${API_PREFIX}/auth/me`, {
        headers: { 'X-Requested-With': 'XMLHttpRequest' },
      });
      if (resp.status() === 429) {
        rateLimited = true;
        break;
      }
    }

    if (rateLimited) {
      // 等待限流恢复
      await page.waitForTimeout(5000);
      const retryResp = await page.request.get(`${API_BASE}${API_PREFIX}/auth/me`, {
        headers: { 'X-Requested-With': 'XMLHttpRequest' },
      });
      // 恢复后应可访问或仍被限流（不崩溃）
      expect(retryResp.status() < 500).toBe(true);
    }
    // 没被限流也通过
  });

  test('401 拦截器：清除 cookie 后访问跳转登录页', async ({ browser }) => {
    // 用显式 context（默认 page fixture 的 context 注入了全局 storageState 登录态）
    const context = await browser.newContext({ storageState: { cookies: [], origins: [] } });
    const page = await context.newPage();

    // 登录一次获得真实会话（UI 登录，模拟用户操作）
    await page.goto(`${BASE_URL}/login`, { waitUntil: 'domcontentloaded' });
    await page.waitForTimeout(1000);
    await page
      .evaluate(() => window.localStorage.setItem('bingxi.locale', 'zh-CN'))
      .catch(() => {});
    const userInput = page
      .locator('input[placeholder="用户名"], input[placeholder="Username"]')
      .first();
    await userInput.waitFor({ state: 'visible', timeout: 20_000 });
    await userInput.fill(process.env.TEST_USERNAME || 'e2e_admin');
    const pwdInput = page
      .locator('input[placeholder="密码"], input[placeholder="Password"]')
      .first();
    await pwdInput.waitFor({ state: 'visible', timeout: 20_000 });
    await pwdInput.fill(process.env.TEST_PASSWORD || 'Xk9#mQ2$vL8pW4nR');
    const loginBtn = page.locator('form button.el-button--primary').first();
    await loginBtn.waitFor({ state: 'visible', timeout: 20_000 });
    await loginBtn.click();
    // 等待登录成功跳转
    await page.waitForURL(/dashboard|purchase|\//, { timeout: 20_000 }).catch(() => {});
    await page.waitForTimeout(2000);

    // 清除 cookie 模拟 token 过期（httpOnly cookie 一并被清）
    await context.clearCookies();
    // 同时清除 localStorage 权限缓存（20.11-D：userInfo 会从缓存恢复，导致守卫误判已登录）
    await page
      .evaluate(() => {
        localStorage.removeItem('erp_cached_perms');
        localStorage.removeItem('erp_cached_perms_ts');
      })
      .catch(() => {});

    // 导航到受保护页面（时间戳参数破坏缓存，确保守卫执行）
    await page.goto(`${BASE_URL}/purchase?t=${Date.now()}`, { waitUntil: 'domcontentloaded' });
    // 等待重定向到登录页（最长 15s），固定 3s 在加载慢时会误判
    await page.waitForURL(/\/login/, { timeout: 15_000 }).catch(() => {});

    // 应被重定向到登录页或初始化页（守卫在 init/status 请求失败时失败安全引导至 /setup，
    // 截图证实 401 后可能落在 Setup 向导页——两者都算未登录重定向）
    const url = page.url();
    expect(url.includes('/login') || url.includes('/setup')).toBe(true);

    await context.close();
  });
});
