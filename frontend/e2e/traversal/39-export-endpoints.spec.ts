import { test, expect } from '../diagnose-fixture';
import { loginViaUI } from '../flow/helpers';
import { SENSITIVE_EXPORT_ENDPOINTS, NON_SENSITIVE_EXPORT_ENDPOINTS } from './endpoints.config';
import {
  classifyStatus,
  assertEndpointRegistered,
  assertNotPermissionDenied,
  parseAppError,
} from './matrix-probe';

/**
 * P5.9 导出端点全量矩阵（敏感 12 + 非敏感 26，配置驱动）
 *
 * 敏感端点 fail-closed 断言：
 * - 无 token → 403（P1.1 fail-closed 生效）
 * - 伪造 token → 403
 * 完整审批链（申请→审批→持 token 200→二次消费拒）在审批流专项 spec 覆盖
 *
 * 非敏感端点：不再有 404/400 → test.skip 的假绿。按响应形态真实判定：
 * - 5xx / 裸 404·405（无标准错误体）→ 判红：处理器崩溃 / 路由未注册（路径写错，
 *   典型如复数化 /purchases/orders/export 这类恒 404 的错路径）
 * - 2xx/3xx                          → 导出成功（xlsx/JSON 均可，非敏感导出不要求实体数据）
 * - 4xx 且为标准错误体               → 已注册、仅业务/数据原因拒绝，记录；admin 权限拒绝判红
 */

const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp';

test.describe('P5.9 敏感导出 fail-closed 矩阵', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  for (const { path, resource } of SENSITIVE_EXPORT_ENDPOINTS) {
    test(`FAIL-CLOSED ${path} [${resource}] 无 token 403`, async ({ page }) => {
      const resp = await page.request.get(`${API_BASE}${API_PREFIX}${path}`);

      // fail-closed：无 token 必须被拒绝（403 无权限 / 400 参数校验先行均属拒绝，绝不允许 200/2xx）
      const status = resp.status();
      expect(
        [400, 401, 403].includes(status),
        `${path} 无审批令牌应被拒绝（400/401/403，fail-closed），实际 ${status}`
      ).toBe(true);
    });

    test(`FAIL-CLOSED ${path} [${resource}] 伪造 token 403`, async ({ page }) => {
      const resp = await page.request.get(
        `${API_BASE}${API_PREFIX}${path}?download_token=fake-token-for-e2e-test`
      );

      // fail-closed：伪造令牌必须被拒绝（400/401/403）
      const status = resp.status();
      expect(
        [400, 401, 403].includes(status),
        `${path} 伪造令牌应被拒绝（400/401/403，fail-closed），实际 ${status}`
      ).toBe(true);
    });
  }
});

test.describe('P5.9 非敏感导出矩阵', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  for (const path of NON_SENSITIVE_EXPORT_ENDPOINTS) {
    test(`EXPORT ${path}`, async ({ page }) => {
      const resp = await page.request.get(`${API_BASE}${API_PREFIX}${path}`);

      const status = resp.status();
      const bodyText = status >= 400 ? await resp.text() : '';

      const outcome = classifyStatus(status, bodyText);
      // 崩溃 / 未注册（裸 404·405，含路径写错）→ 判红（去掉原来的 test.skip()）
      assertEndpointRegistered(`EXPORT ${path}`, outcome, { status, bodyText });

      if (outcome === 'registered-error') {
        // 处理器已注册，仅业务/数据原因拒绝：admin 权限拒绝是真实缺陷
        const err = parseAppError(bodyText);
        assertNotPermissionDenied(`EXPORT ${path}`, err?.code ?? '');
        test.info().annotations.push({
          type: 'registered-rejected',
          description: `端点 ${path} 已注册，返回 ${status}（${err?.code}）`,
        });
        return;
      }

      // outcome === 'ok'
      expect(status, `${path} 应返回 2xx/3xx，实际 ${status}`).toBeLessThan(400);
    });
  }
});
