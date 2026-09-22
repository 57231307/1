import { test, expect } from '../diagnose-fixture';
import { loginViaUI, apiCall } from '../flow/helpers';
import { APPROVE_ENDPOINTS } from './endpoints.config';

/**
 * P5.11 审批端点全量矩阵（48 端点，配置驱动）——可达性判定
 *
 * 每端点以 id=1 POST {comments}，按 apiCall 抛错信息做真实判定（不再 404/400 → test.skip）：
 * - 请求返回非 JSON（裸 404/405）        → 判红：路由未注册 / 方法不匹配（路径写错）
 * - INTERNAL_ERROR / DATABASE_ERROR       → 判红：处理器崩溃
 * - FORBIDDEN / 403 / PERMISSION          → 判红：admin 持 * 仍被拒，真实权限缺陷
 * - NOT_FOUND / BAD_REQUEST / VALIDATION / BUSINESS_ERROR
 *                                         → 已注册，仅 id=1 无数据 / 状态机拒绝，记录
 * 真实审批流（创建前置单据 → approve → 状态断言 → reject 分支）在四套专用流 spec
 * （export/role-change/transfer/writeoffs）覆盖。
 */

/** 视为"端点已注册、仅数据/状态原因拒绝"的稳定业务错误码白名单 */
const REGISTERED_REJECT_CODES = ['NOT_FOUND', 'BAD_REQUEST', 'VALIDATION_ERROR', 'BUSINESS_ERROR'];

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
        const data = await apiCall<Record<string, unknown>>(page, 'POST', resolvedPath, {
          comments: 'E2E 审批矩阵测试',
        });
        // 200 + code 200：审批调用成功（或后端接受该请求）
        expect(data.code, `${resolvedPath} 业务码应 200`).toBe(200);
      } catch (e) {
        const msg = (e as Error).message;
        // 裸 404/405（后端对未注册路由返回空体，无法 JSON 解析）→ 路由未注册，判红
        if (/returned non-JSON/.test(msg)) {
          throw new Error(
            `APPROVE ${resolvedPath} 未被后端注册：请求返回非 JSON（裸 404/405），` +
              `路径拼写错误或后端路由缺失，不能当作数据缺失静默跳过。原始：${msg.slice(0, 160)}`
          );
        }
        // apiCall 抛错包含 code=XXX message=YYY（HTTP 层已通过，业务层拒绝）
        const codeMatch = msg.match(/code=([^\s]+)/);
        const bizCode = codeMatch ? codeMatch[1] : '';

        // 权限拒绝：admin 持 *:* 被拒为真缺陷
        // forbidden_response 业务码为 FORBIDDEN（error.rs 的 error_code），另有数字 403 形态
        if (
          bizCode === '403' ||
          bizCode === 'FORBIDDEN' ||
          bizCode.includes('PERMISSION') ||
          bizCode.includes('FORBIDDEN')
        ) {
          throw new Error(`${resolvedPath} admin 账号被权限拒绝——${msg}`);
        }
        // 处理器内部错误 / 数据库错误（后端映射为 5xx 状态）→ 真崩溃，判红
        if (bizCode === 'INTERNAL_ERROR' || bizCode === 'DATABASE_ERROR') {
          throw new Error(`${resolvedPath} 处理器内部错误（${bizCode}）——${msg.slice(0, 160)}`);
        }
        // 其余：端点已注册，仅 id=1 无数据 / 状态机拒绝。未知错误码仍判红，避免把
        // 新的处理器缺陷伪装成"数据缺失"。
        if (!REGISTERED_REJECT_CODES.includes(bizCode)) {
          throw new Error(
            `${resolvedPath} 返回未预期错误码 ${bizCode || '(未解析)'}——${msg.slice(0, 160)}`
          );
        }
        test.info().annotations.push({
          type: 'registered-no-seed',
          description: `${entity} 端点已注册，id=1 无数据/状态机拒绝（${bizCode}）；真实审批链见 41b-c 与四套专用流 spec`,
        });
      }
    });
  }
});
