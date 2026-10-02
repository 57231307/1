import { test, expect } from '../diagnose-fixture';
import { loginViaUI, apiCall } from '../flow/helpers';
import { APPROVE_ENDPOINTS } from './endpoints.config';

/**
 * P5.11 审批端点全量矩阵（48 端点，配置驱动）——可达性判定
 *
 * 每端点以 id=1 按配置 method（缺省 POST）发 {comments}，按 apiCall 抛错信息做真实判定
 * （不再 404/400 → test.skip）：
 * - 请求返回非 JSON 且 HTTP 404        → 判红：路由未注册（路径写错）
 * - 请求返回非 JSON 且 HTTP 405        → 判红：方法不匹配（后端未以该方法注册，多为
 *                                         配置漏标 method，已在 endpoints.config 逐项核对）
 * - 请求返回非 JSON（任何状态，含 422）→ 判红：后端已把请求解码失败统一收进 AppError
 *                                         信封（middleware/trace_context.rs 的
 *                                         normalize_extractor_rejection：axum 提取器纯文本
 *                                         400/415/422 → 400 + code=VALIDATION_ERROR JSON），
 *                                         已注册端点不应再以非 JSON 裸状态出参；再出现
 *                                         即信封收口契约回退。**旧「422 纯文本 = 端点已
 *                                         注册」判据自此废止**（按 422 判注册在新契约下
 *                                         永不命中，会误把解码失败漏进未预期码判红路径）。
 * - code=VALIDATION_ERROR（400 信封）  → 已注册：请求已过路由/方法匹配、进入后端解码/
 *                                         校验层（serde 必填/类型校验未过），可达性已达成，
 *                                         单独记录（真实审批链在专用流 spec 覆盖）
 * - INTERNAL_ERROR / DATABASE_ERROR       → 判红：处理器崩溃
 * - FORBIDDEN / 403 / PERMISSION          → 判红：admin 持 * 仍被拒，真实权限缺陷
 *                                         （403/401 拒绝的 message 判定维持脱敏常量口径，
 *                                          本仓硬约束：不得改成期待真实原因外显）
 * - NOT_FOUND / BAD_REQUEST / BUSINESS_ERROR
 *                                         → 已注册，仅 id=1 无数据 / 状态机拒绝，记录
 * 真实审批流（创建前置单据 → approve → 状态断言 → reject 分支）在四套专用流 spec
 * （export/role-change/transfer/writeoffs）覆盖。
 */

/** 视为"端点已注册、仅数据/状态原因拒绝"的稳定业务错误码白名单
 *  （VALIDATION_ERROR 另有前置的"已进入校验层"专属判定分支，不走本白名单） */
const REGISTERED_REJECT_CODES = ['NOT_FOUND', 'BAD_REQUEST', 'BUSINESS_ERROR'];

test.describe('P5.11 审批端点全量矩阵', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  for (const { path, entity, method } of APPROVE_ENDPOINTS) {
    const resolvedPath = path.replace('{id}', '1').replace('{version_id}', '1');
    if (resolvedPath.includes('{')) continue;
    const httpMethod = method ?? 'POST';

    test(`APPROVE ${httpMethod} ${resolvedPath} [${entity}]`, async ({ page }) => {
      // 用 apiCall：内置 CSRF token 提取 + CSRF_TOKEN_INVALID 两级恢复
      // （50 分片并发同库时 storageState 里的一次性 csrf 易被竞争消费，
      // 手动构造请求无恢复逻辑会把 CSRF 拒绝误判为权限缺陷）
      try {
        const data = await apiCall<Record<string, unknown>>(page, httpMethod, resolvedPath, {
          comments: 'E2E 审批矩阵测试',
        });
        // 200 + code 200：审批调用成功（或后端接受该请求）
        expect(data.code, `${resolvedPath} 业务码应 200`).toBe(200);
      } catch (e) {
        const msg = (e as Error).message;
        // apiCall 对无法 JSON 解析的响应抛 "returned non-JSON (status NNN)"。
        // 裸 404/405（后端对未注册路由/方法不匹配返回空体）→ 路由未注册，判红；
        // 请求解码失败已由 trace_context.rs normalize_extractor_rejection 收进统一信封
        // （400 + code=VALIDATION_ERROR JSON），任何已注册端点都不该再以非 JSON 出参——
        // 非 JSON 一律判红（含 422：再现即"纯文本解码拒绝"回潮，信封收口契约被击穿，
        // 不得按旧判据放行）。
        if (/returned non-JSON/.test(msg)) {
          const status = msg.match(/status (\d{3})/)?.[1];
          throw new Error(
            `APPROVE ${httpMethod} ${resolvedPath} 未被后端正确注册，或请求解码拒绝的 ` +
              `AppError 信封收口回退（422 纯文本已契约废止）：请求返回非 JSON（裸 ` +
              `404=路径写错 / 405=方法不匹配，实际 status=${status ?? '?'}），` +
              `不能当作数据缺失静默跳过。原始：${msg.slice(0, 160)}`
          );
        }
        // apiCall 抛错包含 code=XXX message=YYY（HTTP 层已通过，业务层拒绝）
        const codeMatch = msg.match(/code=([^\s]+)/);
        const bizCode = codeMatch ? codeMatch[1] : '';

        // 权限拒绝：admin 持 *:* 被拒为真缺陷
        // forbidden_response 业务码为 FORBIDDEN（error.rs 的 error_code），另有数字 403 形态。
        // 判定只看机器码/状态，message 维持脱敏常量口径（本仓硬约束，不期待真实原因外显）。
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
        // 按机器码判"已注册"（取代改前依赖 422 纯文本的判据）：VALIDATION_ERROR =
        // 请求已过路由/方法匹配、进入后端解码/校验层（id=1 缺省 {comments} 未过 serde
        // 必填校验，或校验器拒绝）。可达性已达成，记录。
        if (bizCode === 'VALIDATION_ERROR') {
          test.info().annotations.push({
            type: 'registered-validation-envelope',
            description:
              `${entity} 端点已注册（code=VALIDATION_ERROR：路由与方法匹配且已进入校验层，` +
              `id=1 缺省 {comments} 未过必填/类型校验，400 统一信封）；` +
              `真实审批链见 41b-c 与专用流 spec`,
          });
          return;
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
