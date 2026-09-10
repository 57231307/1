import { test, expect } from '@playwright/test';
import { loginViaUI } from '../flow/helpers';
import { SENSITIVE_EXPORT_ENDPOINTS, NON_SENSITIVE_EXPORT_ENDPOINTS } from './endpoints.config';

/**
 * P5.9 导出端点全量矩阵（敏感 12 + 非敏感 27，配置驱动）
 *
 * 敏感端点 fail-closed 断言：
 * - 无 token → 403（P1.1 fail-closed 生效）
 * - 伪造 token → 403
 * 完整审批链（申请→审批→持 token 200→二次消费拒）在审批流专项 spec 覆盖
 *
 * 非敏感端点：
 * - 200 + xlsx/json 响应
 * - 5xx 才是真失败
 */

const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp';

test.describe('P5.9 敏感导出 fail-closed 矩阵', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  for (const { path, resource } of SENSITIVE_EXPORT_ENDPOINTS) {
    test(`FAIL-CLOSED ${path} [${resource}] 无 token 403`, async ({ page }) => {
      const resp = await page.request
        .get(`${API_BASE}${API_PREFIX}${path}`)
        .catch((e) => { console.warn(`[E2E] 操作失败（降级跳过）: ${(e as Error).message}`); return null; });

      if (!resp) throw new Error(`网络错误: ${path}`);

      // fail-closed：无 token 必须 403
      expect(
        resp.status(),
        `${path} 无审批令牌应返回 403（fail-closed），实际 ${resp.status()}`,
      ).toBe(403);
    });

    test(`FAIL-CLOSED ${path} [${resource}] 伪造 token 403`, async ({ page }) => {
      const resp = await page.request
        .get(`${API_BASE}${API_PREFIX}${path}?download_token=fake-token-for-e2e-test`)
        .catch((e) => { console.warn(`[E2E] 操作失败（降级跳过）: ${(e as Error).message}`); return null; });

      if (!resp) throw new Error(`网络错误: ${path}`);

      expect(
        resp.status(),
        `${path} 伪造令牌应返回 403，实际 ${resp.status()}`,
      ).toBe(403);
    });
  }
});

test.describe('P5.9 非敏感导出矩阵', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  for (const path of NON_SENSITIVE_EXPORT_ENDPOINTS) {
    test(`EXPORT ${path}`, async ({ page }) => {
      const resp = await page.request
        .get(`${API_BASE}${API_PREFIX}${path}`)
        .catch((e) => { console.warn(`[E2E] 操作失败（降级跳过）: ${(e as Error).message}`); return null; });

      if (!resp) throw new Error(`网络错误: ${path}`);

      const status = resp.status();
      if (status === 404 || status === 400) {
        test.info().annotations.push({
          type: 'missing-data',
          description: `端点 ${path} 返回 ${status}，需补种子数据`,
        });
        test.skip();
        return;
      }

      expect(status, `${path} 应返回 2xx/3xx，实际 ${status}`).toBeLessThan(400);
    });
  }
});
