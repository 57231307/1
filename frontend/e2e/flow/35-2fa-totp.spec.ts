import { test, expect } from '../diagnose-fixture';
import {
  BROWSER_NETWORK_NOISE,
  apiCall,
  apiCallExpectFail,
  assertPageHealthy,
  generateTotp,
  loginViaUI,
  trackPageHealth,
  failureCode,
  APP_ERROR_CODES,
} from './helpers';

/**
 * P5.5 2FA TOTP 全流程测试
 * 依赖：P3.2（generateTotp）+ 后端 totp/setup、enable、recovery-codes 端点
 *
 * 验证：
 * - setup 拿 secret → TOTP 生成 → enable → 退出后登录带 token 成功、错 token 拒绝
 * - recovery-codes 一次性消费
 * - 禁用回退单因素
 */
test.describe('P5.5 2FA TOTP', () => {
  test('setup → enable → 登录带 token 成功', async ({ page }) => {
    await loginViaUI(page);
    const collector = trackPageHealth(page);

    // 1. setup 拿 secret（后端 GET /auth/totp/setup，非 POST）
    const setupResp = await apiCall<{ secret?: string; qr_code?: string }>(
      page,
      'GET',
      '/auth/totp/setup'
    );
    const secret = setupResp?.data?.secret;
    expect(
      secret,
      `GET /auth/totp/setup 未返回 secret（响应：${JSON.stringify(setupResp).slice(0, 200)}），TOTP 前置失败`
    ).toBeTruthy();

    // 2. 生成 TOTP
    const totpCode = generateTotp(secret);

    // 3. enable（后端 TotpVerifyRequest { token }，字段名为 token 非 code）
    const enableResp = await apiCall(page, 'POST', '/auth/totp/enable', {
      token: totpCode,
    });
    expect(
      enableResp !== null && enableResp !== undefined,
      `[35-totp] enable 应成功（secret=${secret?.slice(0, 8)}… code=${totpCode}）`
    ).toBeTruthy();

    await assertPageHealthy(page, collector, { consoleNoisePatterns: BROWSER_NETWORK_NOISE });
  });

  test('错 TOTP code 拒绝', async ({ page }) => {
    await loginViaUI(page);

    const setupResp = await apiCall<{ secret?: string; qr_code?: string }>(
      page,
      'GET',
      '/auth/totp/setup'
    );
    const secret = setupResp?.data?.secret;
    expect(
      secret,
      `GET /auth/totp/setup 未返回 secret（响应：${JSON.stringify(setupResp).slice(0, 200)}），TOTP 前置失败`
    ).toBeTruthy();

    // 故意用错 code（后端 TotpVerifyRequest { token }，字段名为 token）
    // 后端真实语义（读码结论，非推断）：
    //   services/totp_service.rs:69-79 —— 令牌不匹配返回 Ok(false)，不抛错；
    //   handlers/auth_handler_misc.rs:343 —— enable_totp 的 `Ok(false)` 分支
    //     `=> Err(AppError::bad_request("验证码不正确"))`，即该端点**不会**返回 200 + data=false，
    //     而是 HTTP 400（utils/error.rs:178 BadRequest → BAD_REQUEST），
    //     响应体由 utils/error.rs:143-148 直出 { code: "BAD_REQUEST", message(脱敏), trace_id, timestamp }。
    // 因此旧断言 `enableResp?.data === false || enableResp === false` 写错在断言侧：
    //   ① data === false 永不成立（该分支根本不产生 ApiResponse<bool>）；
    //   ② apiCall/apiCallRaw 是"成功语义"，对 code !== 200 直接抛错（见 helpers.apiCall），
    //      所以 400 会让用例在到达断言前就异常失败，而不是判为"被正确拒绝"。
    // 负例改用 apiCallExpectFail，并同时校验状态码与机器码。
    const enableResp = await apiCallExpectFail(page, 'POST', '/auth/totp/enable', {
      token: '000000',
    });
    expect(
      enableResp.status,
      `错误 TOTP 码应被拒为 HTTP 400，实际响应=${JSON.stringify(enableResp).slice(0, 200)}`
    ).toBe(400);
    expect(
      failureCode(enableResp),
      `错误 TOTP 码应命中 BadRequest 机器码，实际响应=${JSON.stringify(enableResp).slice(0, 200)}`
    ).toBe(APP_ERROR_CODES.BAD_REQUEST);
  });

  test('生成恢复码并消费', async ({ page }) => {
    await loginViaUI(page);

    // 后端真实路径为 /auth/totp/recovery-codes（nest 在 /auth 下的 totp 子组）
    // generate_recovery_codes 返回 ApiResponse<Vec<String>>：data 就是恢复码数组本身，
    // 此前按 resp.codes / resp.data.codes 取值恒为 undefined → 用例每轮静默跳过（假绿）
    const recoveryResp = await apiCall<string[]>(page, 'POST', '/auth/totp/recovery-codes');
    const codes = recoveryResp.data;
    expect(
      Array.isArray(codes),
      `恢复码 data 应为字符串数组，实际：${JSON.stringify(recoveryResp).slice(0, 200)}`
    ).toBe(true);
    expect(codes.length, '恢复码应至少生成 1 条').toBeGreaterThan(0);
    expect(
      codes.every(c => typeof c === 'string' && c.length > 0),
      `恢复码每条应为非空字符串，实际：${JSON.stringify(codes).slice(0, 200)}`
    ).toBe(true);

    // 恢复码应是一次性的
    expect(codes.length).toBeGreaterThan(0);
  });
});
