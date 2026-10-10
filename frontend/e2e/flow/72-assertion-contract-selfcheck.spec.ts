import { test, expect } from '../diagnose-fixture';
import {
  CSRF_ERROR_CODES,
  APP_ERROR_CODES,
  expectDenied,
  expectStateGateRejection,
  isStateGateRejection,
  type ApiFailureResult,
} from './helpers';

/**
 * 断言判据自身的契约自证（负向自证套件）
 *
 * 为什么要有这一份：本仓 CSRF 中间件与权限门**都**直出 HTTP 403
 * （backend/src/middleware/csrf.rs:232-234 → code=`CSRF_*`；
 * backend/src/utils/response.rs:145 + backend/src/utils/error.rs:709/742 → code=`FORBIDDEN`），
 * 所以"只判 status"的越权/门禁断言是假绿——权限门或状态门被整条删掉，用例照样绿。
 * helpers 里的 `expectDenied` / `expectStateGateRejection` 是这类断言的唯一判据入口
 * （`verifyPermissionDenied` / `verifyIllegalTransition` / `verifyBulkColorDeliveryBlock`
 * 都转交它们）。本套件把"必须判红"的反例钉在这里：一旦有人把判据放宽回只判 status，
 * 这些用例会立即红，而不是等到越权缺陷漏到线上。
 *
 * 输入形态说明：这里传入的对象就是 `apiCallExpectFail` 的真实返回形状
 * （helpers.ts:1579 `return { status, code, message }`），不构造未注册路由、不依赖会话，
 * 因此本套件不发任何 HTTP 请求、不开浏览器（不使用 page fixture）。
 */
test.describe.serial('P7.2 断言判据契约自证（负向自证，不触网）', () => {
  test('S-1 越权断言必须把 CSRF 家族 403 判红（三种机器码逐个）', () => {
    for (const code of Object.values(CSRF_ERROR_CODES)) {
      const result: ApiFailureResult = { status: 403, code };
      expect(
        () => expectDenied(result, `CSRF 冒名越权负例 code=${code}`),
        `expectDenied 放过了 403+${code}——CSRF 拒绝会冒名权限拒绝，判据已被放宽回只判 status`
      ).toThrow();
    }
  });

  test('S-2 越权断言必须让权限门 403+FORBIDDEN 通过（正向对照，防"永远红"式假收紧）', () => {
    expect(() =>
      expectDenied({ status: 403, code: APP_ERROR_CODES.FORBIDDEN }, '真越权负例')
    ).not.toThrow();
  });

  test('S-3 越权断言对无法归因的 403 必须判红（code 缺失 / 非字符串 / 非 403）', () => {
    expect(() => expectDenied({ status: 403 }, 'code 缺失')).toThrow();
    expect(() => expectDenied({ status: 403, code: 403 }, 'code 为数字')).toThrow();
    expect(() =>
      expectDenied({ status: 200, code: APP_ERROR_CODES.FORBIDDEN }, '根本没拒')
    ).toThrow();
    expect(() => expectDenied({ status: 500, code: 'INTERNAL_ERROR' }, '5xx 裸崩')).toThrow();
  });

  test('S-4 状态门断言必须把 CSRF 家族 403 判红（门禁被删也不得算"拒绝生效"）', () => {
    for (const code of Object.values(CSRF_ERROR_CODES)) {
      const result: ApiFailureResult = { status: 403, code };
      expect(
        () => expectStateGateRejection(result, `状态机冒名负例 code=${code}`),
        `expectStateGateRejection 放过了 403+${code}——CSRF/权限拒绝会冒名状态门拒绝`
      ).toThrow();
      expect(
        isStateGateRejection(result),
        `isStateGateRejection 把 403+${code} 判成了门禁拒绝`
      ).toBe(false);
    }
  });

  test('S-5 状态门断言必须把裸 5xx 与 404 判红（DB 约束兜底/路由漂移不算业务拒绝）', () => {
    for (const result of [
      { status: 500, code: 'INTERNAL_ERROR' },
      { status: 500, code: 'DATABASE_ERROR' },
      { status: 501, code: 'NOT_IMPLEMENTED' },
      { status: 404, code: APP_ERROR_CODES.NOT_FOUND },
    ] as ApiFailureResult[]) {
      expect(
        () => expectStateGateRejection(result, `非状态门拒绝 status=${result.status}`),
        `expectStateGateRejection 放过了 ${result.status}+${String(result.code)}——裸崩/未注册会冒名拒绝生效`
      ).toThrow();
    }
  });

  test('S-6 状态门断言必须让真业务拒绝通过（正向对照：400 + 三个机器码族成员）', () => {
    for (const code of [
      APP_ERROR_CODES.BUSINESS_ERROR,
      APP_ERROR_CODES.VALIDATION_ERROR,
      APP_ERROR_CODES.BAD_REQUEST,
    ]) {
      const result: ApiFailureResult = { status: 400, code, message: 'x' };
      expect(
        () => expectStateGateRejection(result, `合法业务拒绝 code=${code}`),
        `expectStateGateRejection 误杀了 400+${code}（判据过严会把真绿打成假红）`
      ).not.toThrow();
      expect(isStateGateRejection(result), `isStateGateRejection 漏判 400+${code}`).toBe(true);
    }
  });
});
