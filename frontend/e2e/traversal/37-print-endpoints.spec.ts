import { test, expect } from '../diagnose-fixture';
import { loginViaUI } from '../flow/helpers';
import { PRINT_ENDPOINTS } from './modules.config';
import {
  classifyStatus,
  assertEndpointRegistered,
  assertZipDocxResponse,
  assertNotPermissionDenied,
  parseAppError,
} from './matrix-probe';

/**
 * P5.7 打印端点全量矩阵（58 端点，配置驱动）——可达性 + 容器判定
 *
 * 端点以 id=1 探测，按"响应形态"做真实判定（不再是 404/400 → test.skip 的假绿）：
 * - 5xx / 裸 404·405（无标准错误体）        → 判红：处理器崩溃 / 路由未注册（路径写错）
 * - 200                                     → 必须是真实 docx zip 容器（PK magic + >1KB + OOXML）
 * - 4xx 且为标准错误体（已注册，仅 id=1 无种子数据） → 记录为"已注册-无数据"，并断言其
 *   不是 admin 权限拒绝；真实存在实体上的 200+内容匹配在 37b-print-content 覆盖。
 */

const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp';

test.describe('P5.7 打印端点全量矩阵', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  for (const endpoint of PRINT_ENDPOINTS) {
    const resolvedPath = endpoint.replace('{id}', '1').replace('{delivery_id}', '1');
    // 含未解析占位符的路径跳过（多级参数端点需要专用前置链）
    if (resolvedPath.includes('{')) continue;

    test(`PRINT ${resolvedPath}`, async ({ page }) => {
      const resp = await page.request.get(`${API_BASE}${API_PREFIX}${resolvedPath}`);

      const status = resp.status();
      const body = await resp.body();
      const bodyText = status >= 400 ? body.toString('utf8') : '';

      const outcome = classifyStatus(status, bodyText);
      // 崩溃 / 未注册 → 判红（这里去掉原来的 test.skip()）
      assertEndpointRegistered(`PRINT ${resolvedPath}`, outcome, { status, bodyText });

      if (outcome === 'registered-error') {
        // 处理器已注册，仅因 id=1 无种子数据被业务层拒绝：admin 被权限拒绝是真实缺陷
        const err = parseAppError(bodyText);
        assertNotPermissionDenied(`PRINT ${resolvedPath}`, err?.code ?? '');
        test.info().annotations.push({
          type: 'registered-no-seed',
          description: `端点 ${resolvedPath} 已注册，id=1 无数据（${err?.code}）；真实数据打印见 37b`,
        });
        return;
      }

      // outcome === 'ok'：200 必须是真实 docx zip 容器
      expect(status, `${resolvedPath} 应返回 200`).toBe(200);
      assertZipDocxResponse(`PRINT ${resolvedPath}`, body, resp.headers()['content-type'] ?? '');
    });
  }
});
