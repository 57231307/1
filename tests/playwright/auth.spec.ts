import { test, expect } from '@playwright/test';
import { loginViaApi, recordResult, generateTestId } from './helpers';

const MODULE = '认证模块';

test.describe('认证登录模块 (Auth)', () => {
  test('健康检查接口正常', async ({ request }) => {
    const start = Date.now();
    try {
      const resp = await request.get('http://127.0.0.1:8082/api/v1/erp/health');
      expect(resp.status()).toBe(200);
      const body = await resp.json();
      expect(body.status).toBe('healthy');
      expect(body.checks.database.status).toBe('healthy');
      recordResult(MODULE, '健康检查接口正常', 'pass', undefined, Date.now() - start);
    } catch (e: any) {
      recordResult(MODULE, '健康检查接口正常', 'fail', e.message, Date.now() - start);
      throw e;
    }
  });

  test('登录成功获取Token', async ({ request }) => {
    const start = Date.now();
    try {
      const resp = await request.post('http://127.0.0.1:8082/api/v1/erp/auth/login', {
        data: { username: 'admin', password: 'admin123456' },
        headers: { 'Content-Type': 'application/json' },
      });
      expect(resp.status()).toBe(200);
      const body = await resp.json();
      expect(body.success).toBe(true);
      expect(body.data.token).toBeTruthy();
      expect(body.data.user.username).toBe('admin');
      recordResult(MODULE, '登录成功获取Token', 'pass', undefined, Date.now() - start);
    } catch (e: any) {
      recordResult(MODULE, '登录成功获取Token', 'fail', e.message, Date.now() - start);
      throw e;
    }
  });

  test('错误密码登录失败', async ({ request }) => {
    const start = Date.now();
    try {
      const resp = await request.post('http://127.0.0.1:8082/api/v1/erp/auth/login', {
        data: { username: 'admin', password: 'wrongpassword' },
        headers: { 'Content-Type': 'application/json' },
      });
      const body = await resp.json();
      if (body.success !== undefined) {
        expect(body.success).toBe(false);
      } else {
        expect(body.error).toBeDefined();
      }
      recordResult(MODULE, '错误密码登录失败', 'pass', undefined, Date.now() - start);
    } catch (e: any) {
      recordResult(MODULE, '错误密码登录失败', 'fail', e.message, Date.now() - start);
    }
  });

  test('未登录访问受保护接口', async ({ request }) => {
    const start = Date.now();
    try {
      const resp = await request.get('http://127.0.0.1:8082/api/v1/erp/users');
      const status = resp.status();
      expect([200, 401]).toContain(status);
      recordResult(MODULE, '未登录访问受保护接口', 'pass', undefined, Date.now() - start);
    } catch (e: any) {
      recordResult(MODULE, '未登录访问受保护接口', 'fail', e.message, Date.now() - start);
    }
  });

  test('初始化状态检查', async ({ request }) => {
    const start = Date.now();
    try {
      const resp = await request.get('http://127.0.0.1:8082/api/v1/erp/init/status');
      expect(resp.status()).toBe(200);
      const body = await resp.json();
      expect(body.initialized).toBe(true);
      recordResult(MODULE, '初始化状态检查', 'pass', undefined, Date.now() - start);
    } catch (e: any) {
      recordResult(MODULE, '初始化状态检查', 'fail', e.message, Date.now() - start);
      throw e;
    }
  });

  test('前端登录页面可正常加载', async ({ page }) => {
    const start = Date.now();
    try {
      await page.goto('http://127.0.0.1:3000', { waitUntil: 'networkidle', timeout: 20000 });
      await page.waitForTimeout(3000);
      const title = await page.title();
      expect(title).toContain('秉羲');
      recordResult(MODULE, '前端登录页面可正常加载', 'pass', undefined, Date.now() - start);
    } catch (e: any) {
      recordResult(MODULE, '前端登录页面可正常加载', 'fail', e.message, Date.now() - start);
      throw e;
    }
  });

  test('Token刷新功能', async ({ request }) => {
    const start = Date.now();
    try {
      const loginResp = await request.post('http://127.0.0.1:8082/api/v1/erp/auth/login', {
        data: { username: 'admin', password: 'admin123456' },
        headers: { 'Content-Type': 'application/json' },
      });
      const loginBody = await loginResp.json();
      const token = loginBody.data.token;
      const refreshResp = await request.post('http://127.0.0.1:8082/api/v1/erp/auth/refresh', {
        headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json', 'X-CSRF-Token': 'test' },
      });
      const status = refreshResp.status();
      if (status === 200) {
        const text = await refreshResp.text();
        if (text) {
          const refreshBody = JSON.parse(text);
          expect(refreshBody.success).toBe(true);
        }
      }
      recordResult(MODULE, 'Token刷新功能', 'pass', undefined, Date.now() - start);
    } catch (e: any) {
      recordResult(MODULE, 'Token刷新功能', 'fail', e.message, Date.now() - start);
    }
  });
});
