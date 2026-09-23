import { test, expect } from '../diagnose-fixture';
import { request as pwRequest } from '@playwright/test';
import {
  loginViaUI,
  BASE_URL,
  API_BASE,
  API_PREFIX,
  TEST_USERNAME,
  TEST_PASSWORD,
} from './helpers';

/**
 * 在独立 context 中完成 UI 登录，返回该 context。
 * 用于 refresh/logout 测试——这些操作会吊销会话（后端 revoke_jti 即时拉黑），
 * 若在分片共享 storageState 播种的默认 context 上执行，会导致后续同分片所有 spec 401。
 */
async function loginInIsolatedContext(browser: import('@playwright/test').Browser) {
  const ctx = await browser.newContext({ storageState: { cookies: [], origins: [] } });
  const p = await ctx.newPage();
  await p.goto(`${BASE_URL}/login`, { waitUntil: 'domcontentloaded' });
  await p.evaluate(() => window.localStorage.setItem('bingxi.locale', 'zh-CN'));
  await p.waitForTimeout(1000);

  const userInput = p.locator('input[placeholder="用户名"], input[placeholder="Username"]').first();
  await userInput.waitFor({ state: 'visible', timeout: 20_000 });
  await userInput.fill(TEST_USERNAME);

  const pwdInput = p.locator('input[placeholder="密码"], input[placeholder="Password"]').first();
  await pwdInput.waitFor({ state: 'visible', timeout: 20_000 });
  await pwdInput.fill(TEST_PASSWORD);

  // 勾选用户协议（表单要求 agreedToTerms=true）
  const checkboxInput = p.locator('.el-checkbox input').first();
  const isChecked = await checkboxInput.isChecked().catch(() => false);
  if (!isChecked) {
    await p.locator('.el-checkbox__inner').first().click();
    await p.waitForTimeout(300);
  }

  const loginBtn = p.locator('form button.el-button--primary').first();
  await loginBtn.waitFor({ state: 'visible', timeout: 20_000 });
  await loginBtn.click();
  await p.waitForURL(url => !url.pathname.includes('/login'), { timeout: 40_000 });
  await p.waitForTimeout(1000);

  // 验证登录确实成功（cookie 已写入）
  const cookies = await ctx.cookies();
  if (!cookies.some(c => c.name === 'access_token')) {
    await ctx.close();
    throw new Error('[loginInIsolatedContext] 独立 context 登录后未获得 access_token');
  }
  return { ctx, page: p };
}

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
    await page.waitForURL(/\/(login|setup)/, { timeout: 60_000 });

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
      .waitFor({ state: 'visible', timeout: 10_000 });

    const hasForm = await page.locator('form, .el-form, .setup-container').first().isVisible();
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

  test('刷新 Token 接口契约：成功轮换且旧会话即时失效', async ({ browser }) => {
    // 在独立 context 中登录和操作，避免吊销分片共享 storageState 的会话 S0
    const { ctx, page } = await loginInIsolatedContext(browser);

    try {
      // 记录刷新前的 cookie 值（用于验证轮换与旧 token 失效）
      const cookiesBefore = await ctx.cookies();
      const oldAccessToken = cookiesBefore.find(c => c.name === 'access_token')?.value ?? '';
      const oldRefreshToken = cookiesBefore.find(c => c.name === 'refresh_token')?.value ?? '';
      expect(oldAccessToken).toBeTruthy();
      expect(oldRefreshToken).toBeTruthy();

      // 调用 refresh
      const refreshResp = await page.request.post(`${API_BASE}${API_PREFIX}/auth/refresh`, {
        headers: { 'X-Requested-With': 'XMLHttpRequest' },
      });

      // 真实契约断言：成功返回 200
      expect(refreshResp.status()).toBe(200);

      // 响应体符合后端 RefreshTokenResponse 契约（csrf_token 字段 skip，仅输出 expires_in）
      const body = await refreshResp.json();
      expect(body.data).toBeDefined();
      expect(body.data.expires_in).toBeGreaterThan(0);

      // 新 access_token 已下发且与旧值不同（证明轮换实际发生）
      const cookiesAfter = await ctx.cookies();
      const newAccessToken = cookiesAfter.find(c => c.name === 'access_token')?.value ?? '';
      expect(newAccessToken).toBeTruthy();
      expect(newAccessToken).not.toBe(oldAccessToken);

      // 新 refresh_token 已轮换
      const newRefreshToken = cookiesAfter.find(c => c.name === 'refresh_token')?.value ?? '';
      expect(newRefreshToken).toBeTruthy();
      expect(newRefreshToken).not.toBe(oldRefreshToken);

      // 新会话可用：GET /auth/me → 200
      const meResp = await page.request.get(`${API_BASE}${API_PREFIX}/auth/me`, {
        headers: { 'X-Requested-With': 'XMLHttpRequest' },
      });
      expect(meResp.status()).toBe(200);

      // 旧 access_token 已被吊销（后端 revoke_jti 即时拉黑 session_id）
      // 使用独立 bare request context 发送旧 token，不污染已更新的浏览器 context
      const bareReq = await pwRequest.newContext({
        baseURL: API_BASE,
        extraHTTPHeaders: { 'X-Requested-With': 'XMLHttpRequest' },
      });
      const oldMeResp = await bareReq.get(`${API_PREFIX}/auth/me`, {
        headers: { Cookie: `access_token=${oldAccessToken}` },
      });
      expect(oldMeResp.status()).toBe(401);
      await bareReq.dispose();

      // 旧 refresh_token 已入黑名单（revoke_old_token 将其写入 token_blacklist）
      const bareRefreshResp = await pwRequest.newContext({
        baseURL: API_BASE,
        extraHTTPHeaders: { 'X-Requested-With': 'XMLHttpRequest' },
      });
      const oldRefreshAttempt = await bareRefreshResp.post(`${API_PREFIX}/auth/refresh`, {
        headers: { Cookie: `refresh_token=${oldRefreshToken}` },
      });
      expect(oldRefreshAttempt.status()).toBe(401);
      await bareRefreshResp.dispose();
    } finally {
      await ctx.close();
    }
  });

  test('登出后 Cookie 被清除且会话失效', async ({ browser }) => {
    // 在独立 context 中登录和操作，避免吊销分片共享会话 S0
    const { ctx, page } = await loginInIsolatedContext(browser);

    try {
      // 登出前验证会话有效
      const meBefore = await page.request.get(`${API_BASE}${API_PREFIX}/auth/me`, {
        headers: { 'X-Requested-With': 'XMLHttpRequest' },
      });
      expect(meBefore.status()).toBe(200);

      // 获取 CSRF token 并执行登出
      const csrfToken = (await ctx.cookies()).find(c => c.name === 'csrf_token')?.value || '';
      const logoutResp = await page.request.post(`${API_BASE}${API_PREFIX}/auth/logout`, {
        headers: {
          'X-Requested-With': 'XMLHttpRequest',
          'X-CSRF-Token': csrfToken,
        },
      });

      // 真实断言：登出必须成功（独立 context 中刚登录，token 一定有效）
      expect(logoutResp.status()).toBe(200);
      const logoutBody = await logoutResp.json();
      expect(logoutBody.data.success).toBe(true);

      // 登出后 access_token cookie 被清除（后端 Set-Cookie maxAge=0 删除）
      const cookiesAfter = await ctx.cookies();
      const accessCookie = cookiesAfter.find(c => c.name === 'access_token');
      expect(
        accessCookie === undefined ||
          accessCookie.value === '' ||
          accessCookie.expires <= Date.now() / 1000 + 1
      ).toBe(true);

      // 登出后会话彻底失效：/auth/me → 401
      const meAfter = await page.request.get(`${API_BASE}${API_PREFIX}/auth/me`, {
        headers: { 'X-Requested-With': 'XMLHttpRequest' },
      });
      expect(meAfter.status()).toBe(401);
    } finally {
      await ctx.close();
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
    await page.evaluate(() => window.localStorage.setItem('bingxi.locale', 'zh-CN'));
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
    await page.waitForURL(/dashboard|purchase|\//, { timeout: 20_000 });
    await page.waitForTimeout(2000);

    // 清除 cookie 模拟 token 过期（httpOnly cookie 一并被清）
    await context.clearCookies();
    // 同时清除 localStorage 权限缓存（20.11-D：userInfo 会从缓存恢复，导致守卫误判已登录）
    await page.evaluate(() => {
      localStorage.removeItem('erp_cached_perms');
      localStorage.removeItem('erp_cached_perms_ts');
    });

    // 导航到受保护页面（时间戳参数破坏缓存，确保守卫执行）
    await page.goto(`${BASE_URL}/purchase?t=${Date.now()}`, { waitUntil: 'domcontentloaded' });
    // 等待重定向到登录页（CI 慢环境守卫链 /auth/me 401 → refresh 401 → redirect
    // 首次 JS 执行可达 20s+，60s 留余量；守卫 init/status 失败安全时落 /setup 也接受）
    await page.waitForURL(/\/(login|setup)/, { timeout: 60_000 });

    // 应被重定向到登录页或初始化页（守卫在 init/status 请求失败时失败安全引导至 /setup，
    // 截图证实 401 后可能落在 Setup 向导页——两者都算未登录重定向）
    const url = page.url();
    expect(url.includes('/login') || url.includes('/setup')).toBe(true);

    await context.close();
  });
});
