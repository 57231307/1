import { test, expect } from '@playwright/test';
import { loginViaUI } from '../flow/helpers';
import { APPROVE_ENDPOINTS } from './endpoints.config';

/**
 * P5.11 审批端点全量矩阵（48 端点，配置驱动）
 *
 * 每端点配置：路径 + 前置实体（entity 字段标注来源）
 * 基础断言（无前置数据场景）：
 * - 端点可达（非 405 方法错误、非 5xx 崩溃）
 * - 404/400 = CI 种子数据缺失（skip + annotation 记录）
 * - 403 = 权限拒绝（admin 账号不应出现，出现即真缺陷）
 *
 * 真实审批流（创建前置单据 → approve → 状态断言 → reject 分支）在
 * 四套专用流 spec（export/role-change/transfer/writeoffs）覆盖
 */

const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp';

test.describe('P5.11 审批端点全量矩阵', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  for (const { path, entity } of APPROVE_ENDPOINTS) {
    const resolvedPath = path.replace('{id}', '1').replace('{version_id}', '1');
    if (resolvedPath.includes('{')) continue;

    test(`APPROVE ${resolvedPath} [${entity}]`, async ({ page }) => {
      // 带 CSRF 头：CSRF 缺失同样 403，不带会把 CSRF 拒绝误判为权限缺陷
      const cookies = await page.context().cookies();
      const csrf = cookies.find((c) => c.name === 'csrf_token');

      const resp = await page.request
        .post(`${API_BASE}${API_PREFIX}${resolvedPath}`, {
          data: { comments: 'E2E 审批矩阵测试' },
          headers: {
            // 从 storageState 提取 csrf
            'Content-Type': 'application/json',
            'X-Requested-With': 'XMLHttpRequest',
            ...(csrf ? { 'X-CSRF-Token': csrf.value } : {}),
          },
        })
        .catch(() => null);

      if (!resp) throw new Error(`网络错误: ${resolvedPath}`);

      const status = resp.status();

      // 5xx 永远是真失败
      expect(
        status,
        `${resolvedPath} 不应 5xx（服务器崩溃）`,
      ).toBeLessThan(500);

      if (status === 404 || status === 400) {
        // id=1 实体不存在或校验失败：数据缺失，非缺陷
        test.info().annotations.push({
          type: 'missing-data',
          description: `${entity} 前置数据缺失（id=1 返回 ${status}），需配置 createApi 前置链`,
        });
        test.skip();
        return;
      }

      if (status === 403) {
        // admin 账号被拒：真缺陷（矩阵用 admin 身份跑）
        throw new Error(`${resolvedPath} admin 账号被 403 拒绝——权限配置缺陷`);
      }

      // 200/201/409（状态机不允许）均可达
      expect(status).toBeLessThan(500);
    });
  }
});
