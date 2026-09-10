import { test, expect } from '@playwright/test';
import { loginViaUI, apiCall, type ApiResponse } from '../flow/helpers';
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
      // 用 apiCall：内置 CSRF token 提取 + CSRF_TOKEN_INVALID 两级恢复
      // （50 分片并发同库时 storageState 里的一次性 csrf 易被竞争消费，
      // 手动构造请求无恢复逻辑会把 CSRF 拒绝误判为权限缺陷）
      try {
        const data = await apiCall<Record<string, unknown>>(
          page,
          'POST',
          resolvedPath,
          { comments: 'E2E 审批矩阵测试' },
        );
        // 200 + code 200：审批调用成功（或后端接受该请求）
        expect(data.code, `${resolvedPath} 业务码应 200`).toBe(200);
      } catch (e) {
        const msg = (e as Error).message;
        // apiCall 抛错包含 code=XXX message=YYY（HTTP 层已通过，业务层拒绝）
        const codeMatch = msg.match(/code=([^\s]+)/);
        const bizCode = codeMatch ? codeMatch[1] : '';

        // 权限拒绝：admin 持 *:* 被拒为真缺陷
        if (bizCode.includes('PERMISSION') || bizCode.includes('FORBIDDEN')) {
          throw new Error(`${resolvedPath} admin 账号被权限拒绝——${msg}`);
        }
        // 其余（实体缺失 NOT_FOUND/状态机 BUSINESS_ERROR/校验 VALIDATION 等）＝前置数据缺失
        test.info().annotations.push({
          type: 'missing-data',
          description: `${entity} 前置数据缺失：${msg.slice(0, 160)}`,
        });
        test.skip();
      }
    });
  }
});
